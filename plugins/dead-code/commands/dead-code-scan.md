---
description: Scan for dead code, unused imports, duplicates, and zombie code across the project
user-invocable: true
allowed-tools:
  - Read
  - Grep
  - Glob
  - Bash
argument-hint: "[target]"
---

# Dead Code Scan

Comprehensive scan for dead code across the project. Reports findings without making changes. Use `/dead-code-clean` to actively remove.

**CRITICAL**: This is a scan, not a fix. Document issues thoroughly with clear evidence and confidence levels. Use `/dead-code-clean` to remove issues after the scan.

**First**: Use the dead-code-expert skill for detection catalog, language tools, and false-positive awareness.

## Preparation

1. Detect the project language from file extensions and config files (package.json, Cargo.toml, pyproject.toml, go.mod, *.csproj, Package.swift).
2. Identify entry points (main, index, App, etc.).
3. Determine the target. If no target specified, scan from project root focusing on source directories.

## Automated Scans

Run all applicable scans based on detected language.

### Universal Scans (all languages)

```bash
# Debug artifacts
rg -c '(console\.log|print\(|dbg!\(|println!\(|debugger|breakpoint\(\)|System\.out\.print)' . --glob '!*test*' --glob '!*spec*' --glob '!node_modules*' --glob '!target*' --glob '!.git*' --glob '!__pycache__*' 2>/dev/null | awk -F: '{s+=$2} END {print "Debug artifacts:", s+0}'

# Lint suppressions hiding dead code
rg -c '(#\[allow\((dead_code|unused)|eslint-disable.*unused|# noqa: F4|@SuppressWarnings.*unused|#pragma warning disable)' . --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "Lint suppressions:", s+0}'

# Commented-out code (heuristic)
rg -c '^\s*(//|#)\s*(const|let|var|function|class|import|from|if|for|while|return|def |fn |pub |async |await )\b' . --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "Commented code:", s+0}'

# TODO/FIXME/HACK markers
rg -c '\b(TODO|FIXME|HACK|XXX|TEMP|TEMPORARY)\b' . --glob '!node_modules*' --glob '!target*' 2>/dev/null | awk -F: '{s+=$2} END {print "TODO/FIXME/HACK:", s+0}'

# Skipped tests
rg -c '(@skip|@ignore|xit\(|xdescribe\(|\.skip\(|#\[ignore\]|\[Ignore\]|@Disabled|@pytest\.mark\.skip)' . --glob '*test*' --glob '*spec*' 2>/dev/null | awk -F: '{s+=$2} END {print "Skipped tests:", s+0}'
```

### Language-Specific Scans

**JavaScript/TypeScript:**
```bash
npx knip --reporter compact 2>/dev/null || echo "knip not available -- install with: npm install -D knip"
```

**Python:**
```bash
ruff check --select F401,F841 2>/dev/null || echo "ruff not available"
vulture src/ --min-confidence 80 2>/dev/null || echo "vulture not available -- install with: uv add --dev vulture"
```

**Rust:**
```bash
cargo clippy --all-targets --all-features -- -W dead_code -W unused_imports -W unused_variables 2>&1 | head -40
```

**Go:**
```bash
deadcode ./... 2>/dev/null || echo "deadcode not available -- install with: go install golang.org/x/tools/cmd/deadcode@latest"
staticcheck ./... 2>/dev/null
```

## Manual Inspection

After automated scans, perform targeted manual review:

1. **Unused exports** -- For each exported symbol with zero external references, assess whether it's public API or dead code.
→ *Consult [false-positives reference](references/false-positives.md) before flagging.*
2. **Duplicate logic** -- Look for functions with similar names, matching parameter signatures, or overlapping purpose.
→ *Consult [duplicate code reference](references/duplicate-code.md) for clone type detection.*
3. **Speculative generality** -- Interfaces with one implementation, factory/strategy patterns with one variant, unused configuration options.
4. **Orphaned files** -- Files not imported by anything in the dependency chain.

## Generate Scan Report

### Quick Stats
Start with automated scan numbers. Set context for what follows.

### What's Working
Highlight 2-3 things the codebase does well. Be specific about WHY they work:
- Clean import hygiene in specific modules
- Good use of visibility modifiers (private/internal) limiting dead code surface
- Active lint enforcement that prevents accumulation
- Well-structured entry points making dependency tracing reliable

### Certain Dead Code (remove immediately)
- Unused imports (compiler/linter confirmed)
- Unreachable code after return/break/throw
- Debug artifacts in production paths
- Lint suppressions for dead code

### Probable Dead Code (verify then remove)
- Functions/classes with zero references project-wide
- Commented-out code blocks
- Skipped tests with no plan to re-enable
- Orphaned test/config files

### Suspicious (investigate first)
- Exported symbols with zero internal callers (may be public API)
- Code near framework decorators/conventions
- Code with serialization attributes

### Duplicate Implementations
- Functions doing the same thing differently
- Parallel type hierarchies
- Redundant validation layers

### AI Slop Verdict
**If the project uses AI coding tools**, run the AI slop scorecard.
→ *Consult [AI slop patterns](references/ai-slop-patterns.md) for the full 11-pattern catalog and scorecard.*
Check for: copy-paste proliferation, wrapper functions adding nothing, single-impl interfaces, commented-out "previous attempts", excessive comments restating code, phantom edge case handling, reimplemented stdlib, orphaned AI-generated files, refactoring avoidance (v2/old/legacy naming), security theater in tests, and unnecessary backward compatibility shims. Report the scorecard result (Clean / Moderate / Heavy).

### Questions to Consider
Provocative questions that might unlock deeper cleanup:
- "Is this abstraction layer earning its keep, or was it speculative?"
- "Why do two modules solve the same problem differently?"
- "Would removing this simplify the dependency graph?"
- "Is this tested because it's used, or tested because it exists?"

### Summary
Prioritized action list with estimated line counts. State total findings by confidence level. Map findings to `/dead-code-clean` modes: certain items → `certain` mode, high → `high` mode, medium → `aggressive` mode.

## Verify Scan Completeness

Before finalizing the report, check:
- All source directories were scanned (not just `src/`)
- Language-specific tools ran successfully (or failures were noted)
- Framework conventions were considered before flagging exports
- Side-effect imports were excluded
- `_`-prefixed intentionally-unused vars were excluded

**NEVER**:
- Flag code as dead without stating the evidence
- Mix confidence levels (be precise: certain vs high vs medium)
- Skip the "What's Working" section (celebrate good practices)
- Report false positives without checking the false-positives reference
- Forget to estimate line counts (quantify the cleanup opportunity)
