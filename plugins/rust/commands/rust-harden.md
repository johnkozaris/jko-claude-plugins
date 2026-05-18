---
description: Harden Rust code — replace unwrap with proper error handling, add safety comments to unsafe blocks, enable overflow checks, validate inputs at boundaries. The defensive hardening pass.
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# Rust Harden

Systematically harden the target Rust code against production failures. This is the defensive pass — every change reduces crash risk.

> **Note**: this command is a focused entry point for the defensive pass. `/rust-critique --harden` runs the same scans inside the broader critique flow. Use this command when hardening is your only goal; use the critique mode when you want defensive findings surfaced alongside other concerns.

## Preparation

1. Find the workspace root: run `cargo locate-project --workspace --message-format plain 2>/dev/null | xargs dirname`.
2. Identify the target: use the `target` argument, or default to the workspace's `src/`.
3. Determine if this is a library or application crate (check for `lib.rs` vs `main.rs`).

## Hardening Steps

Execute these in order. For each finding, make the fix directly — don't just report it.

### Step 1: Eliminate Panic Sources on External Input

Run in the shell:

```bash
rg --type rust '\.(unwrap|expect)\(' src/ --glob '!*test*' -n
```

For each match, classify it:

**External input (parse, IO, env, deserialize, network, user input)** — NEVER unwrap. This is what took down [Cloudflare on Nov 18, 2025](https://blog.cloudflare.com/18-november-2025-outage/). Replace:
- Unwrap on `Result`: replace with `?` and add `.context("description")` if anyhow is available
- Unwrap on `Option`: replace with `.ok_or_else(|| Error::...)` then `?`

**Genuine invariant** (programmer guarantee, e.g., after a length check or freshly-inserted map key) — keep but document. Change to `.expect("invariant X holds because Y")` with the reason in the message. Per BurntSushi: unwrap-as-assertion is OK; unwrap-as-error-handling is not.

**Mutex::lock() poison case** — `.expect("mutex poisoned — programmer error elsewhere")` is acceptable. Or migrate to `parking_lot::Mutex` which doesn't poison.

**Startup init that MUST succeed** — `.expect("could not load config")` acceptable. The program can't proceed without it.

The Cloudflare receipt is the universal "why": their FL2 proxy hit a hard-coded 200-feature limit; `.unwrap()` on the `Err` panicked in `fl2_worker_thread`; 5xx globally. The fix Cloudflare itself describes isn't "no unwrap" — it's "defensive boundaries: type checks, explicit guards, input validation, limits, and staged rollout." The unwrap was a symptom of unvalidated external input meeting brittle assumptions about its shape.

### Step 2: Document Unsafe Blocks

Run in the shell:

```bash
rg --type rust -B1 'unsafe \{' src/ -n | head -50
```

For each `unsafe` block without a preceding `// SAFETY:` comment, add one explaining:

- What invariant is being upheld
- Why it is safe in this context
- Under what conditions it would become unsound

### Step 3: Check Overflow Protection

Read `Cargo.toml` and check if `[profile.release]` has `overflow-checks = true`. If not, add it:

```toml
[profile.release]
overflow-checks = true
```

**Evidence:** CVE-2018-1000810 (std `str::repeat`) — silent integer overflow in release build. Debug builds panic, release builds silently wrap.

### Step 4: Replace Indexing with Safe Access

Run in the shell:

```bash
rg --type rust '\[\w+\]' src/ --glob '!*test*' -n | head -30
```

For array/slice indexing `x[i]` in non-test code, evaluate whether `.get(i)` with proper error handling is safer. Slice indexing is potentially the main source of panics in non-trivial Rust programs.

### Step 5: Input Validation at Boundaries

Scan public functions that accept external input (CLI args, network data, file content, user strings). Ensure each has validation — ideally via `TryFrom` converting raw input into validated domain types.

### Step 6: Remove Debug Artifacts

Run in the shell:

```bash
rg --type rust '(println!|dbg!|eprintln!|#\[allow\(unused)' src/ -n
```

Replace `println!` with `tracing::info!` or `log::info!`. Remove `dbg!()`. Remove `#[allow(unused)]` — either use the item or delete it. Replace `#[allow(lint)]` with `#[expect(lint)]`.

### Step 7: Security Hardening Pass

Consult the [security reference](../skills/rust-expert/references/security.md) and apply applicable items:

- **TOCTOU**: any `path.is_dir()` / `is_file()` followed by an fs op on the same path → switch to open-with-`O_NOFOLLOW`-then-operate-on-handle.
- **Constant-time comparison**: scan for `==` on `&[u8]`, `Vec<u8>`, `String` containing passwords/tokens/MACs/signatures → switch to `subtle::ConstantTimeEq`.
- **Bounded input**: HTTP body / decompression / `serde` entry points must have explicit size caps.
- **Secret leakage**: any `#[derive(Debug)]` on a struct with `password`, `token`, `secret`, `api_key`, `private_key` field → wrap field in a redacting newtype or `secrecy::SecretString`.
- **Numeric narrowing**: `as iN` / `as uN` where the source is wider → switch to `TryFrom` with error propagation.
- **`Path::join` with absolute argument**: if input is user-controlled, validate `!arg.is_absolute()` first.
- **Dependency audit**: run `cargo audit` and resolve any `RUSTSEC-*` advisories before merge.

## Output

Report what was hardened with a count:

- X unwrap/expect calls replaced
- X unsafe blocks documented
- X overflow-checks added
- X indexing operations reviewed
- X debug artifacts removed
- X security findings fixed (TOCTOU / timing / bounds / secret-leak / narrowing / path-traversal / advisory)
