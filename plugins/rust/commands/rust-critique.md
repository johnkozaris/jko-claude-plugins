---
description: Deep code critique — read the target Rust code and apply the full review process. Evaluates soundness, ownership, error handling, type design, async correctness, performance, architecture, and pre-merge polish. Routes to focused commands when findings cluster around a single concern. Think like a senior Rust engineer giving honest feedback.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "<target> [--pre-merge | --harden | --architect]"
---

# Rust Critique

Conduct a thorough code critique. Think like a senior Rust engineer reviewing a PR — be direct, be specific, explain WHY each finding matters in production terms.

**First**: Use the rust-expert skill for review process, reference files, and decision rules.

## Modes

The default mode surfaces everything. Modes prioritize a subset:

- **(default)** — holistic review: soundness, correctness, design, types, performance, security, architecture, polish, documentation. All severities surfaced.
- **`--pre-merge`** — prioritize polish-level findings (clippy clean, formatting, dead code, debug artifacts, docs, dependency hygiene, RUSTSEC advisories). Use as the last pass before merging a PR.
- **`--harden`** — prioritize defensive findings (unwrap on external input, unsafe without SAFETY, missing overflow-checks, input validation at boundaries, TOCTOU, constant-time comparison, secret leakage, deprecated unmaintained deps). Use when prepping for production deployment.
- **`--architect`** — prioritize architecture-level findings (workspace organization, pattern fit, ports/adapters mismatch, premature abstraction, missing seams). Route to `/rust-architect` for design-level decisions.

## Preparation

1. Find the workspace root: `cargo locate-project --workspace --message-format plain 2>/dev/null | xargs dirname`.
2. Determine if this is a library, application, or workspace.
3. Read the target files. If no target specified, scan `src/` starting with `lib.rs` or `main.rs`.
4. Check `Cargo.toml` for edition, rust-version, dependencies, features, lint config.
5. Note the project's MSRV from `rust-version` — only suggest APIs available at that MSRV. Cross-reference `references/2026-currency.md`.

## Automated Scans

Run these to gather data before the review:

```bash
# Panic sources (anywhere)
rg --type rust '\.(unwrap|expect)\(' src/ --glob '!*test*' -c 2>/dev/null | awk -F: '{s+=$2} END {print "unwrap/expect:", s+0}'

# Unsafe without safety comments
rg --type rust -B1 'unsafe \{' src/ -n 2>/dev/null | rg -v 'SAFETY' | rg -c 'unsafe' 2>/dev/null | awk -F: '{s+=$2} END {print "unsafe without SAFETY:", s+0}'

# Clone frequency (a sea of clones is an AI-slop / borrow-checker fight indicator)
rg --type rust '\.clone()' src/ -c 2>/dev/null | awk -F: '{s+=$2} END {print "clone calls:", s+0}'

# Arc<Mutex> as god-object indicator
rg --type rust 'Arc<Mutex<' src/ -c 2>/dev/null | awk -F: '{s+=$2} END {print "Arc<Mutex<>:", s+0}'

# Debug artifacts
rg --type rust '(println!|dbg!|todo!|unimplemented!)' src/ --glob '!*test*' -c 2>/dev/null | awk -F: '{s+=$2} END {print "debug artifacts:", s+0}'

# Deprecated / unmaintained dependencies (RUSTSEC + supersession watchlist)
echo ""
echo "=== Deprecated dependencies ==="
rg '^(async-std|bincode|lazy_static|once_cell|cfg-if|if_chain)\s*=' Cargo.toml 2>/dev/null && \
  echo "  → see references/2026-currency.md for migration paths"
rg '\b(addr_of!|addr_of_mut!)\b' src/ -n 2>/dev/null | head -3 && \
  echo "  → use &raw const / &raw mut (stable 1.82)"
rg 'async-trait' Cargo.toml 2>/dev/null && \
  echo "  → for static dispatch, use native AFIT (stable 1.75); keep async-trait only for dyn Trait"
rg --type toml '^sled\s*=' Cargo.toml 2>/dev/null && \
  echo "  → sled has been beta for ~6 years; consider redb 4.x (B-tree) or fjall 3.x (LSM) for new projects"

# Deprecated GitHub Actions in CI
find .github/workflows -maxdepth 1 \( -name '*.yml' -o -name '*.yaml' \) 2>/dev/null \
    | xargs grep -l 'actions-rs/' 2>/dev/null \
    | head -3 \
    | grep -q . && \
    echo "  → migrate to dtolnay/rust-toolchain + Swatinem/rust-cache + taiki-e/install-action"

# unbounded_channel usage in production
echo ""
echo "=== Concurrency smells ==="
rg --type rust 'unbounded_channel' src/ --glob '!*test*' -n 2>/dev/null | head -5
[ -n "$(rg --type rust 'unbounded_channel' src/ --glob '!*test*' 2>/dev/null)" ] && \
    echo "  → unbounded mpsc in production is how services OOM. See references/decision-rules.md Rule 9."

# Critique: missing tests on non-trivial public functions
echo ""
echo "=== Coverage ==="
# rough heuristic — count public fns vs test fns
PUBFN=$(rg --type rust '^pub fn|^pub async fn' src/ -c 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
TESTFN=$(rg --type rust '#\[(tokio::)?test\]' --glob '!benches' -c 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')
echo "  pub fns: $PUBFN, test fns: $TESTFN"

# Clippy
echo ""
echo "=== Clippy ==="
cargo clippy --all-targets --all-features --message-format short 2>&1 | tail -10

# Cargo audit (if installed)
echo ""
echo "=== RustSec advisories ==="
command -v cargo-audit >/dev/null 2>&1 && cargo audit 2>&1 | tail -10 || echo "  cargo-audit not installed; run \`cargo install cargo-audit\`"
```

## The Review

Work through findings in severity order:

1. **blocking** — Soundness bug, UB, data race, guaranteed panic on plausible input, security flaw, RUSTSEC-flagged dependency. Must fix before merge.
2. **important** — Wrong error handling on external input (Cloudflare-class), performance cliff in measured hot path, design pain that will cause future churn, missing tests for non-trivial logic, deprecated dependencies needing a migration plan.
3. **architecture** — Misfit pattern, premature abstraction, missing seam, workspace-split signal. Route to `/rust-architect` for design-level work.
4. **nit** — Style, naming, minor idiom, cosmetic.
5. **polish** — Pre-merge cleanup: clippy warnings, formatting drift, dead code, debug artifacts, doc coverage on public items, intra-doc links, dependency hygiene. Surface these prominently in `--pre-merge` mode or when `/rust-polish` is invoked.
6. **suggestion** — Alternative worth considering. No action required.
7. **praise** — Highlight well-written code. Reinforce good patterns.

The SKILL.md uses the 5-level scaffold (`blocking / important / nit / suggestion / praise`) for general code review; this command adds `architecture` and `polish` because the critique workflow needs to route architectural concerns to a focused command and to surface pre-merge cleanup as a distinct prioritization.

### For each finding

1. **File and line** — exactly where.
2. **Severity** — the tier above.
3. **What** — name the problem clearly.
4. **Why it matters** — describe the concrete production cost in plain language: what bug appears, what crashes, what gets corrupted, what slows down, what's hard to change later. When a well-known incident or vulnerability demonstrates the cost on a real codebase, cite it (the Cloudflare Nov 2025 unwrap-panic, a relevant CVE, a RUSTSEC advisory). When no such receipt exists, say so plainly — don't manufacture authority. **If you cannot name a concrete consequence at all, the finding belongs as `suggestion` or shouldn't be raised.**
5. **Fix** — before/after code block when non-obvious.
6. **Command routing** — if a focused workflow would help the user, suggest the matching command. Use judgment, not a count: if the unsafe boundaries need a defensive pass, route to `/rust-harden`. If the primitive types are weak, route to `/rust-types`. If the codebase is approaching the seams a workspace split would clarify, route to `/rust-architect`. Don't suggest a follow-up command just because one finding fell into that category.

Command map (when the workflow fits):
- `/rust-harden` — defensive pass: unwrap on external input, unsafe boundaries, input validation, overflow checks
- `/rust-types` — type-system strengthening: newtypes, illegal states, primitive obsession
- `/rust-polish` — pre-merge cleanup: clippy, formatting, dead code, docs, deps
- `/rust-architect` — design-level decisions: pattern fit, workspace shape, port-and-adapter boundaries
- `/rust-teach` — project conventions to CLAUDE.md (one-time setup)

## Decision rules to apply

Consult `references/decision-rules.md` BEFORE writing any "consider X vs Y" sentence. The genuinely optional choices have criteria-based answers:

- `.unwrap()` policy → Rule 1
- Shared state primitive (Mutex vs RwLock vs ArcSwap vs channel vs actor) → Rule 2
- Newtype or raw primitive → Rule 3
- Builder crate (bon / derive_builder / typed-builder) → Rule 4
- Date/time (jiff / chrono / time) → Rule 5
- Workspace split or single crate → Rule 6
- Edition 2024 migration → Rule 7
- parking_lot or std::sync::Mutex → Rule 8
- Bounded or unbounded channels → Rule 9
- Test ratios → Rule 10

If your finding lives in any of these rules, cite the rule rather than hedging.

## Generate Critique Report

### Quick Stats

Start with the automated scan numbers. These set context for the rest.

### What's Working

Highlight 2-3 things done well. Be specific about WHY they work. Use **praise** severity. This isn't sycophancy — it's reinforcing good patterns so they propagate.

### Priority Issues

The 3-5 most impactful problems, in severity order:
- Blocking issues first (soundness, UB, panics, RUSTSEC, security)
- Then important (error handling on external input, performance cliffs, missing tests, deprecated deps)
- Then architecture-level (pattern misfit, workspace shape, missing seams)
- Then nits / polish / suggestion

For each: file:line, severity, what, why, fix, and **command routing** if applicable.

### Minor Observations

Quick notes on smaller issues. One sentence each.

### What you're not flagging, and why

Sometimes the code does something that looks like it should be flagged but the right call is to leave it alone. When you decide not to flag something, say so explicitly — that documentation is often more valuable than the findings themselves, because it tells the team six months from now what was considered and why.

For example: a working `Arc<Mutex<Cache>>` with short critical sections that aren't held across `.await` isn't the god-object anti-pattern, it's a normal use of a mutex; leave it. An `.unwrap()` on a value the caller's invariant guarantees is fine — though if the invariant isn't documented, upgrading to `.expect("reason")` makes the assumption visible. A trait with one implementor that's deliberately positioned as a seam for a planned second implementation is reasonable; leave it. A `bincode` dependency in code that works is worth flagging for awareness (the maintainers have stopped working on it) but not force-migrating, since the existing v1.3.3 release is complete per the team. Existing `parking_lot` or `derive_builder` usage that works is similarly fine — std and `bon` are alternatives, not migrations.

The general principle: re-litigating an architectural or dependency decision from scratch costs the team more than reading a one-paragraph note explaining why something was left alone. So when you leave something alone, write the note.

### Pattern-Recognition Pass (architecture severity)

Step back from line-by-line and ask:
- **Is this code module-driven, layered, hexagonal, ECS, actor-based, or sans-IO?** Is the chosen shape right for what's being built? (Consult `references/architecture.md`.)
- **Is `Arc<Mutex<T>>` being used where actor / RwLock / ArcSwap / channel would fit better?** (Rule 2.)
- **Is there a workspace-split signal being ignored?** (Rule 6 + `references/workspace-organization.md`.)
- **Are public traits being created with one implementor?** (AI-slop fingerprint #4.)
- **Are advanced features (HRTB, gnarly lifetimes, deep generics) leaking into application code?** Keep them behind crisp library interfaces — Reddit canon (322↑).

If architecture-level findings are non-trivial, **explicitly recommend `/rust-architect`** for a design-level pass.

### Questions to Consider

Provocative questions that might unlock better design:
- "Who should own this data?"
- "Does this need to be this complex?"
- "What would happen if this input is malformed?"
- "Could the type system enforce this invariant?"
- "What would BurntSushi / Niko / Ryhl do here?"
- "If this codebase had to swap Postgres for DynamoDB tomorrow, where does the work happen?"

### Suggested Follow-up Commands

End with concrete next steps:

```
Next steps:
  - /rust-harden       — N findings about unsafe boundaries and external-input unwraps
  - /rust-types        — M findings about primitive obsession
  - /rust-polish       — K findings about pre-merge cleanup
  - /rust-architect    — P findings about pattern fit or workspace shape (if any)
```

Only list commands with ≥1 routed finding. Skip if none.

## How to do this well

Follow the scientific method described in the rust-expert skill: discover, evaluate against evidence, understand before suggesting, propose with verification, use judgment. The skill has the full framing; don't re-derive it here.

A few critique-specific reminders:

Be direct and specific. "Line 42 of parser.rs has a TOCTOU race: the `is_file()` check can be invalidated before the `open()` call" beats "some functions might have race conditions." Vagueness isn't humble, it's lower-quality work.

Every fix gets a verification plan. "Replace `.unwrap()` with `?` and add `.context(...)`" → "the test for malformed input should now return an error instead of panicking — if that test doesn't exist, this is the time to write it." A fix without a verification plan is half a fix.

Prioritize ruthlessly. A finding without a concrete consequence is a suggestion or a nit, not an important. If everything is important, nothing is.

On settled questions, give the settled answer. The rules in `references/decision-rules.md` and the bad-pattern list in `references/architecture.md` exist so the plugin doesn't relitigate the same choices each review. Reserve "this is unsettled" for the actually-unsettled 2026 design questions (AsyncDrop, Polonius, Pin language support, AsyncIterator, `gen` blocks, the Allocator API).

When findings cluster around a focused workflow, route to it. Pointing the developer at `/rust-harden` when there's a defensive-pass theme is more useful than listing every defensive finding individually. Use judgment about when routing actually helps.
