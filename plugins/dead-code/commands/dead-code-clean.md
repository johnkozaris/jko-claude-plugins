---
description: Actively find and remove dead code, unused imports, duplicates, and zombie code
user-invocable: true
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
  - Bash
argument-hint: "[target] [mode: certain|high|aggressive]"
---

# Dead Code Clean

Actively find and remove dead code from the target. Makes changes directly. Defaults to 'high' confidence mode.

**CRITICAL**: This command modifies files. Ensure the working tree is clean before starting. Each phase is verified with build + tests before proceeding to the next.

**First**: Use the dead-code-expert skill for the full detection catalog, false-positive awareness, and safe removal strategies.

## Preparation

1. Detect project language from config files.
2. Determine the target and confidence mode ($2 or default 'high').
3. Ensure the working tree is clean:
```bash
git status --porcelain | head -5
```
If dirty, warn the user but proceed if they confirm.

## Phase 1: Certain Dead Code (always remove)

### 1a. Unused Imports
Run the language-appropriate tool and fix:

**Python:**
```bash
ruff check --select F401 --fix
```

**JS/TS:** Read each file, identify imports not referenced in the file body, remove them with Edit.

**Rust:** Run `cargo clippy` and fix unused import warnings.

**Go:** Already a compile error -- nothing to do.

### 1b. Unused Local Variables
**Python:**
```bash
ruff check --select F841 --fix
```

**Other languages:** Read flagged files from linter output and remove unused variables with Edit.

### 1c. Debug Artifacts
Find and remove in production code (not tests):
```bash
rg -n '(console\.log|print\(|dbg!\(|println!\(|debugger|breakpoint\(\))' . --glob '!*test*' --glob '!*spec*' --glob '!node_modules*' --glob '!target*'
```
Remove each artifact. Replace `println!`/`print()` with structured logging if the project uses a logging framework.

### 1d. Unreachable Code
Find code after return/break/throw and remove it.

### 1e. Commented-Out Code
Find blocks of commented-out code:
```bash
rg -n '^\s*(//|#)\s*(const|let|var|function|class|import|from|if|for|while|return|def |fn |pub )\b' . --glob '!*test*' --glob '!node_modules*' --glob '!target*'
```
Read context around each match. If it's genuinely commented-out code (not documentation), delete it. Version control has the history.

## Phase 2: High Confidence Dead Code (verify then remove)

### 2a. Unused Private Functions/Methods
For private/internal functions with zero callers in the project:
1. Find all private function definitions
2. Search project-wide for each name
3. If only found at definition, remove the function

### 2b. Lint Suppressions
Find and resolve:
```bash
rg -n '#\[allow\((dead_code|unused)|eslint-disable.*unused|# noqa: F4' .
```
For each suppression: check if the underlying code is actually used. If not, remove both the suppression and the dead code. In Rust, convert remaining `#[allow(lint)]` to `#[expect(lint)]`.

### 2c. Unused Dependencies
**JS:** `npx knip --include dependencies`
**Python:** Check imports vs. requirements/pyproject.toml
**Rust:** `cargo machete` or `cargo +nightly udeps`

Remove unused dependencies from the manifest file.

### 2d. Skipped Tests
Find permanently skipped tests:
```bash
rg -n '(@skip|@ignore|xit\(|xdescribe\(|\.skip\(|#\[ignore\]|\[Ignore\]|@Disabled|@pytest\.mark\.skip)' . --glob '*test*'
```
For each: check if there's a linked issue or reason. If it's been skipped >6 months with no plan, delete the test.

## Phase 3: Aggressive Mode Only (medium confidence)

Only execute if mode is 'aggressive'.
→ *Consult [false-positives reference](references/false-positives.md) before every removal in this phase.*

### 3a. Unused Exported Functions
Functions exported/public in applications (not libraries) with zero external callers. Search thoroughly including templates, configs, and dynamic references before removing.

### 3b. Orphaned Files
Files not imported by anything. Verify they're not entry points, config files, or framework-discovered modules before removing.

### 3c. Duplicate Implementations
Identify functions doing the same thing.
→ *Consult [duplicate code reference](references/duplicate-code.md) for clone types and consolidation patterns.*
Choose the canonical implementation, update callers, remove the duplicate.

### 3d. Speculative Generality
Interfaces with single implementation, unused parameters, wrapper functions adding no value. Inline or remove.

### 3e. AI Slop Cleanup
If the project uses AI coding tools, check for AI-specific dead code patterns.
→ *Consult [AI slop patterns](references/ai-slop-patterns.md) for the full catalog.*
Target: copy-paste proliferation, wrapper functions adding nothing, reimplemented stdlib, excessive restating comments, phantom edge case handling.

## Verify After Each Phase

After each phase, confirm nothing broke:
1. Run the project's build/compile command
2. Run the test suite
3. If anything fails, revert the last change and investigate

```bash
# Build check (run the appropriate one)
# JS/TS: pnpm build / npm run build
# Python: python -m py_compile src/**/*.py
# Rust: cargo check
# Go: go build ./...
```

## Output

Report what was cleaned:
- X unused imports removed
- X unused variables removed
- X debug artifacts removed
- X lines of commented-out code deleted
- X unused functions/methods removed
- X lint suppressions resolved
- X unused dependencies removed
- X skipped tests removed
- X duplicate implementations consolidated
- **Total: X lines removed**

State any items skipped due to false-positive risk, with explanation.

**NEVER**:
- Remove public API in libraries without user confirmation
- Remove code with framework decorators without understanding the framework
- Skip build + test verification between phases
- Make large bulk deletions -- work file by file so failures are easy to isolate
- Remove code flagged as medium confidence without checking the false-positives reference
- Mix dead code removal with feature changes in the same edit
