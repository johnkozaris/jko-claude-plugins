---
description: Deep code critique — read the target Rust code and apply the full review process. Evaluates soundness, ownership, error handling, type design, async correctness, performance, and architecture. Think like a senior Rust engineer giving honest feedback.
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# Rust Critique

Conduct a thorough code critique. Think like a senior Rust engineer reviewing a PR — be direct, be specific, explain WHY each finding matters.

**First**: Use the rust-expert skill for review process and reference files.

## Preparation

1. Find the workspace root: `cargo locate-project --workspace --message-format plain 2>/dev/null | xargs dirname`.
2. Determine if this is a library or application crate.
3. Read the target files. If no target specified, scan `src/` starting with `lib.rs` or `main.rs`.
4. Check `Cargo.toml` for edition, dependencies, lint config, and features.

## Automated Scans

Run these to gather data before the review:

```bash
# Panic sources
rg --type rust '\.(unwrap|expect)\(' src/ --glob '!*test*' -c 2>/dev/null | awk -F: '{s+=$2} END {print "unwrap/expect:", s+0}'

# Unsafe without safety comments
rg --type rust -B1 'unsafe \{' src/ -n 2>/dev/null | rg -v 'SAFETY' | rg -c 'unsafe' 2>/dev/null | awk -F: '{s+=$2} END {print "unsafe without SAFETY:", s+0}'

# Clone frequency
rg --type rust '\.clone()' src/ -c 2>/dev/null | awk -F: '{s+=$2} END {print "clone calls:", s+0}'

# Arc<Mutex usage
rg --type rust 'Arc<Mutex<' src/ -c 2>/dev/null | awk -F: '{s+=$2} END {print "Arc<Mutex<>:", s+0}'

# Debug artifacts
rg --type rust '(println!|dbg!|todo!|unimplemented!)' src/ --glob '!*test*' -c 2>/dev/null | awk -F: '{s+=$2} END {print "debug artifacts:", s+0}'

# Clippy
cargo clippy --all-targets --all-features --message-format short 2>&1 | tail -5
```

## The Review

Work through the review process from the rust-expert skill, consulting reference files as needed. For each finding:

1. **File and line** — where exactly
2. **Severity** — blocking / important / nit / suggestion / praise
3. **What** — name the problem clearly
4. **Why it matters** — the concrete consequence (bug, crash, CVE, maintenance cost). If you cannot name a concrete consequence, demote to suggestion.
5. **Fix** — before/after code block when non-obvious
6. **Command** — which command to run if applicable (`/rust-harden`, `/rust-types`, `/rust-polish`)

## Generate Critique Report

### Quick Stats
Start with the automated scan numbers. These set context.

### What's Working
Highlight 2-3 things done well. Be specific about WHY they work. Use **praise** severity.

### Priority Issues
The 3-5 most impactful problems, ordered by severity:
- Blocking issues first (soundness, UB, panics)
- Then important (error handling, performance, API design)
- Then nits

### Minor Observations
Quick notes on smaller issues.

### Questions to Consider
Provocative questions that might unlock better design:
- "Who should own this data?"
- "Does this need to be this complex?"
- "What would happen if this input is malformed?"
- "Could the type system enforce this invariant?"

**Rules**:
- Be direct — vague feedback wastes time
- Be specific — "line 42 of parser.rs" not "some functions"
- Say what's wrong AND why it matters in production
- Give concrete fixes, not "consider exploring..."
- Prioritize ruthlessly — if everything is important, nothing is
- Don't soften criticism — developers need honest feedback to ship safe Rust
