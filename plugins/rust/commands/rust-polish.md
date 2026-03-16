---
description: Final pre-merge polish — remove dead code, verify doc comments, clean clippy, check all Result paths, remove debug artifacts. The last 5% that takes 50% of the effort.
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# Rust Polish

The final pass before merge. Polish is the discipline of caring about what most people skip. Do not polish work that is not functionally complete.

## The 10-Dimension Checklist

Work through each dimension in order.

### 1. Clippy Clean
```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | head -40
```
Fix every warning. No suppressions without `#[expect(lint, reason = "...")]`.

### 2. Formatting
```bash
cargo fmt --check
```
If any diffs, run `cargo fmt`.

### 3. Dead Code
```bash
rg --type rust '(todo!|unimplemented!|#\[allow\(dead_code|#\[allow\(unused)' src/ -n
```
Remove all `todo!()`, `unimplemented!()`, `#[allow(dead_code)]`, `#[allow(unused)]`. Either use it or delete it.

### 4. Debug Artifacts
```bash
rg --type rust '(println!|dbg!|eprintln!)' src/ --glob '!*test*' -n
```
Remove or replace with structured logging (`tracing`/`log`).

### 5. Documentation
```bash
cargo doc --no-deps 2>&1 | rg 'warning' | head -20
```
Every `pub` item needs a doc comment. First line: complete sentence, third person, ends with period. Add `# Examples` for key types and functions.

### 6. Error Handling Completeness
Scan for:
- `let _ = fallible_call()` — is the error intentionally discarded? Add a comment if so.
- Bare `?` without `.context()` — add context at every propagation point.
- `unwrap()` / `expect()` outside tests — replace with `?`.

### 7. Test Coverage
```bash
cargo test 2>&1 | tail -5
cargo test --doc 2>&1 | tail -5
```
Verify tests pass. Check that doc examples compile. Key public functions should have at least one test.

### 8. Dependency Hygiene
```bash
cargo audit 2>&1 | head -20
```
No known vulnerabilities. Remove unused dependencies.

### 9. Public API Surface
Check that:
- No internal types are accidentally `pub` (should be `pub(crate)`)
- `#[non_exhaustive]` on public enums
- No `Arc`, `Mutex`, `Box` in public function signatures (hide implementation details)

### 10. Consistency
- Naming follows Rust conventions (`snake_case` functions, `CamelCase` types)
- Error types follow the project's established pattern
- Module structure reflects domain, not technical layers
- Import style is consistent (use the project's convention)

## Output

Report what was polished as a checklist with pass/fail/fixed status for each dimension.
