# rust

Rust code critique, architecture guidance, and idiomatic pattern enforcement. Detects edition and toolchain from the project.

## What It Does

An expert Rust skill that reviews code for correctness, idiomatic patterns, safety, performance, and design quality. Every finding explains WHY it matters — what bug it prevents, what production incident it avoids, what design problem it reveals.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install rust@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/rust
```

## Commands

| Command          | Purpose                                                                            |
| ---------------- | ---------------------------------------------------------------------------------- |
| `/rust-critique` | Full code review with automated scans, severity-labeled findings, and routing to focused commands. Supports `--pre-merge` and `--harden` modes. |
| `/rust-architect` | Detect the existing architecture, compare healthy options without imposing one, and assess workspace shape |
| `/rust-harden`   | Defensive pass, applies fixes: replace unwrap, SAFETY comments, overflow-checks, input validation, and type strengthening (newtypes, enums over bools, illegal states unrepresentable) |
| `/rust-polish`   | 10-dimension pre-merge checklist (clippy, fmt, dead code, docs, deps)              |
| `/rust-teach`    | One-time: scan your project, write Rust conventions to CLAUDE.md                   |

## Skill

The `rust-expert` skill activates automatically when working with Rust code. It provides:

- 3-layer thinking model (Language Mechanics / Design Choices / Domain Constraints)
- Evidence-driven review process with search-before-change and explicit verification
- Error-to-design-question reframing (E0382 → "Who should own this data?")
- 7 severity levels (blocking / important / architecture / nit / polish / suggestion / praise)
- 24 reference files covering every major Rust domain

## Hook

No active runtime hooks.

## References

24 reference files organized by domain:

**Code-level**: ownership, error-handling, traits, async, unsafe, performance, type-patterns, concurrency, testing, modules-cargo, macros, documentation, serde, anti-patterns, ai-slop, security

**Architecture-level**: design-principles, **architecture** (8 patterns including hexagonal with decision criteria), **workspace-organization** (Zellij case study, sub-crate decomposition)

**Interop**: **ffi** (UniFFI, napi-rs, PyO3, cxx, WASM Components, Edition 2024 unsafe attribute syntax)

**Lookup tables**: **decision-rules** (10 unified rules for optional choices), **2026-currency** (feature index, RUSTSEC deprecations, LTS lines, default crate set), **rust-1.97-release-notes** (complete 1.96.1-1.97.1 migration digest)

**Review workflow**: **review-process** (search-before-change, evidence standard, verification loop, high-signal judgments)
