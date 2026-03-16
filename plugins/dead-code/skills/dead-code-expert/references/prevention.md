# Preventing Dead Code Accumulation

Practices and processes that stop dead code before it enters the codebase.

## Core Principles

### YAGNI -- You Aren't Going to Need It
Don't write code today for features needed tomorrow. If the feature never materializes, the code is dead from birth. Write the simplest thing that works. Refactor when actual needs emerge.

### Boy Scout Rule
Leave the code cleaner than you found it. Every PR should remove a small amount of dead code, not add to it. Over time, the codebase shrinks to its essential minimum.

### Delete Before You Write
Before adding new code, check if similar functionality already exists. Duplicate implementations are the most insidious form of dead code because both copies appear alive.

### Version Control Is Your Safety Net
Deleted code is never truly gone -- it lives in git history. This makes deletion risk-free. Stop commenting out code "just in case." Delete it. `git log -S 'function_name'` will find it if you ever need it again.

## CI/CD Prevention

### Lint Rules That Block Dead Code

**JavaScript/TypeScript:**
```json
{
  "rules": {
    "no-unused-vars": "error",
    "no-unreachable": "error",
    "no-unused-expressions": "error",
    "@typescript-eslint/no-unused-vars": "error"
  }
}
```

**Python (ruff):**
```toml
[tool.ruff.lint]
select = ["F401", "F811", "F841"]  # unused imports, redefined, unused variables
```

**Rust:**
```toml
[lints.rust]
dead_code = "deny"
unused_imports = "deny"
unused_variables = "warn"
```

**Go:** Already enforced by compiler.

### CI Pipeline Checks
```yaml
# Example GitHub Actions step
- name: Check for dead code
  run: |
    # JS/TS projects
    npx knip --reporter compact

    # Python projects
    vulture src/ --min-confidence 80
    ruff check --select F401,F841

    # Rust projects
    cargo clippy -- -D dead_code -D unused_imports
```

### PR Review Checklist
- [ ] No new unused imports
- [ ] No commented-out code
- [ ] No `TODO`/`FIXME` without linked issue
- [ ] No lint suppressions for unused code
- [ ] No new functions without callers
- [ ] Feature flag code has an expiration plan

## Code Review Practices

### What Reviewers Should Watch For

1. **Commented-out code in PRs** -- Never approve. Either the code is needed (uncomment it) or it isn't (delete it).
2. **New lint suppressions** -- `#[allow(dead_code)]` or `eslint-disable unused` in a PR means the developer knows it's dead and is shipping it anyway. Push back.
3. **Speculative generality** -- Interfaces with one implementation, factory classes with one product, strategy patterns with one strategy. Ask: "Where's the second implementation?"
4. **Copy-paste with modification** -- Blocks of similar code. Ask: "Can this be extracted into a shared function?"
5. **Unused parameters** -- Parameters accepted but never read. Ask: "Is this intentional? Should it be removed?"

### What PR Authors Should Do

1. **Run dead code detection before opening PR** -- Catch issues before review.
2. **Separate cleanup from feature work** -- Dead code removal gets its own commit/PR. Don't mix with features.
3. **Delete aggressively, trust version control** -- If git has the history, you don't need the commented code.

## Architectural Prevention

### Small Modules
Smaller modules make dead code more visible. A 50-line module with 10 unused lines is obvious. A 5,000-line module with 500 unused lines hides them.

### Clear Entry Points
Projects with well-defined entry points (main, index, routes) make it possible to trace the dependency graph and find disconnected subgraphs.

### Explicit Over Implicit
Frameworks that discover code by convention (magic strings, naming patterns) make dead code detection harder. When possible, prefer explicit registration over convention-based discovery.

### Feature Flag Hygiene
Every feature flag should have:
1. An owner
2. An expiration date
3. A plan to remove the flag (either roll out or kill)
4. CI that alerts when a flag is past its expiration

### Dependency Hygiene
Remove unused dependencies regularly. They are dead code at the package level.
- JS: `npx knip --include dependencies` or `npx depcheck`
- Python: `deptry`, or check imports vs. requirements/pyproject.toml manually
- Rust: `cargo +nightly udeps` or `cargo machete`
- Go: `go mod tidy`

## Culture

### Make Deletion Celebrated
Lines of code removed should be celebrated as much as lines added. A PR that removes 500 lines of dead code is more valuable than one that adds 500 lines of speculative features.

### Regular Cleanup Sprints
Dedicate time (e.g., one day per month) specifically to dead code removal. This prevents accumulation and keeps the codebase lean.

### Measure and Track
Track dead code metrics over time. If the ratio is increasing, the team needs to prioritize cleanup. If it's decreasing, celebrate.
