---
description: One-time setup that scans your Rust project, understands its patterns and conventions, and writes a Rust-specific context section to your CLAUDE.md. Run once per project to establish persistent Rust guidelines.
argument-hint: "[project-root]"
allowed-tools:
  - Read
  - Edit
  - Write
  - Grep
  - Glob
  - Bash
  - AskUserQuestion
---

# Rust Teach

Scan this Rust project, understand its patterns, and persist the findings so all future sessions start with the right context.

## Step 1: Explore the Codebase

Before asking questions, thoroughly scan the project:

- **Cargo.toml**: Edition, rust-version, dependencies, features, lint config, workspace structure
- **src/lib.rs or src/main.rs**: Module structure, re-exports, visibility patterns
- **Error types**: `rg --type rust 'enum.*Error|struct.*Error' src/ -l` — what error handling strategy is used?
- **Error crate**: Is it thiserror, anyhow, snafu, or hand-rolled?
- **Logging**: tracing, log, or println?
- **Async runtime**: tokio, async-std, smol, or sync-only?
- **Testing**: What test frameworks? proptest, rstest, mockall, insta?
- **Clippy config**: Check `Cargo.toml` `[lints.clippy]` and any `clippy.toml`
- **Unsafe usage**: `rg --type rust 'unsafe' src/ -c` — how much, where, documented?
- **Build profiles**: LTO, overflow-checks, codegen-units settings
- **CI config**: `.github/workflows/`, `justfile`, `Makefile` — what checks run?
- **Existing CLAUDE.md**: Any Rust conventions already documented?

Note what you've learned and what remains unclear.

## Step 2: Ask Clarifying Questions

Ask the user only what you couldn't infer from the codebase:

### Project Context
- Is this a library, application, or both?
- What's the target audience? (internal team, open-source consumers, embedded, web services)
- Any performance constraints? (latency SLAs, memory limits, throughput targets)

### Conventions
- Any strong opinions on iterator chains vs for loops?
- Preferred error handling approach if not clear from code?
- Any crates that should always/never be used?
- Minimum supported Rust version (MSRV) if different from Cargo.toml?

### Safety & Quality
- How strict should unsafe review be? (zero tolerance, case-by-case, embedded-pragmatic)
- What clippy lint level does the team prefer?
- Any domain-specific rules? (e.g., "all network calls must be retried", "all DB queries go through the repository layer")

Skip questions where the answer is already clear from the codebase.

## Step 3: Write Rust Context

Synthesize findings into a `## Rust Conventions` section:

```markdown
## Rust Conventions

### Project Type
[Library / Application / Both — target audience, deployment context]

### Error Handling
[Strategy: thiserror/anyhow/snafu — pattern used, crate-level Error type location]

### Patterns
[Iterator style, module organization, visibility conventions, naming patterns]

### Dependencies
[Key crates, async runtime, logging crate, test frameworks]

### Quality Gates
[Clippy config, required CI checks, unsafe policy, doc requirements]

### Domain Rules
[Any project-specific conventions that apply to all Rust code in this project]
```

Write this section to the project's `CLAUDE.md` file. If the file exists, append or update the Rust Conventions section — do not overwrite existing content.

## Step 4: Confirm

Summarize the key conventions that will now guide all future Rust work in this project. Tell the user they can run `/rust-teach` again if conventions change.
