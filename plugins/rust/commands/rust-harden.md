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

## Preparation

1. Find the workspace root: run `cargo locate-project --workspace --message-format plain 2>/dev/null | xargs dirname`.
2. Identify the target: use the `target` argument, or default to the workspace's `src/`.
3. Determine if this is a library or application crate (check for `lib.rs` vs `main.rs`).

## Hardening Steps

Execute these in order. For each finding, make the fix directly — don't just report it.

### Step 1: Eliminate Panic Sources

Run in the shell:
```bash
rg --type rust '\.(unwrap|expect)\(' src/ --glob '!*test*' -n
```

For each match:
- If the unwrap is on a `Result`: replace with `?` and add `.context("description")` if anyhow is available
- If the unwrap is on an `Option`: replace with `.ok_or_else(|| Error::...)` then `?`
- If it's genuinely an invariant (e.g., after a length check): change to `.expect("reason: invariant X holds because Y")` with a clear explanation

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

## Output

Report what was hardened with a count:
- X unwrap/expect calls replaced
- X unsafe blocks documented
- X overflow-checks added
- X indexing operations reviewed
- X debug artifacts removed
