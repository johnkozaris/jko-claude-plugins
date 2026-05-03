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

1. Detect the project language from file extensions and config files (package.json, Cargo.toml, pyproject.toml, go.mod, \*.csproj, Package.swift).
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
   → _Consult [false-positives reference](references/false-positives.md) before flagging — includes DI container resolution as category 11._
2. **Duplicate logic** -- Look for functions with similar names, matching parameter signatures, or overlapping purpose.
   → _Consult [duplicate code reference](references/duplicate-code.md) for clone type detection AND cross-boundary "split-brain" duplication patterns._
3. **Speculative generality** -- Interfaces with one implementation, factory/strategy patterns with one variant, unused configuration options.
4. **Orphaned files** -- Files not imported by anything in the dependency chain.
5. **Stack-specific dead code** -- DI registrations with no consumer, IBOutlets bound to deleted views, API routes with no client, dead Tailwind utilities, unawaited coroutines, EF Core navigations never traversed, etc.
   → _Consult [stack-specific patterns](references/stack-specific.md) for SwiftUI/iOS, Rust, TS/React/Next.js, .NET, Python, C/C++._
6. **Deployment / asset / infrastructure dead code** -- Env vars never read, Helm values never templated, Dockerfile stages never copied, K8s resources nothing routes to, dead localization keys, orphan asset files, dead scripts.
   → _Consult [deployment dead code reference](references/deployment-dead-code.md)._
7. **Structural smells hiding dead code** -- God classes, cyclic dependencies, divergent change. Name the smell; surface the dead code inside; defer refactoring to a critique skill.
   → _Consult [structural smells reference](references/structural-smells.md)._

## Cross-Boundary / Split-Brain Scan

Run if the project spans multiple stacks (frontend + backend, mobile + web, monolith + microservices):

```bash
# Same enum-like declarations across languages
rg -o '(enum|class)\s+(\w+)' -t ts -t py -t cs -t swift --no-filename \
  | sort | uniq -c | sort -rn | awk '$1 >= 2 {print $1, $2}' | head -20

# Same field names across model/dto/schema/type directories
for d in models schemas dto types entities; do
  fd -t f . "$d" 2>/dev/null
done | xargs rg -o '^\s*(\w+):' --no-filename -r '$1' 2>/dev/null \
  | sort | uniq -c | sort -rn | awk '$1 >= 3' | head -20

# Two route handlers covering same path
rg -o "(get|post|put|delete|patch|MapGet|MapPost|app\.route)\(['\"]([^'\"]+)" \
  --no-filename -r '$2' | sort | uniq -c | sort -rn | awk '$1 >= 2'

# Hardcoded URLs duplicated
rg -o 'https?://[a-zA-Z0-9./?=&_-]+' --no-filename | sort | uniq -c | sort -rn | head -10
```

## Cyclic Dependency / Complexity Scan

Cycles and god files hide dead code from standard tools.

```bash
# Cycles (run the matching one)
npx madge --circular --extensions ts,tsx,js,jsx src/ 2>/dev/null
pydeps --show-cycles --max-bacon 0 src/ 2>/dev/null
cargo modules generate graph --package $(basename $PWD) 2>&1 | grep -i cycle

# God files (>500 lines)
fd -t f -e py -e ts -e tsx -e js -e jsx -e rs -e cs -e swift -e go src/ \
  | xargs wc -l 2>/dev/null | awk '$1 > 500' | sort -rn | head -20

# High complexity (lizard, multi-language)
lizard -C 10 -L 50 -a 5 src/ 2>/dev/null | head -50
```

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
- Types only constructed by DI container (or registered in DI but with no consumer)

### Duplicate Implementations

- Functions doing the same thing differently
- Parallel type hierarchies
- Redundant validation layers

### Split-Brain / Cross-Boundary Duplication

**If the project spans multiple stacks** (frontend + backend, mobile + web, monolith + microservices):

- Same enum / status / type declared in multiple languages
- Validation rules in client AND server (drift risk)
- DTO ↔ Entity ↔ Frontend type triplet
- Two services covering the same resource
- Hardcoded URLs / magic strings / status codes in both halves
- Two configuration sources for the same setting (env vs file)
- Old API + new API both live with no deprecation timer
- Frontend reimplementing backend computation (drift risk)

For each: name the canonical source and recommend codegen or shared package strategy.
→ _Consult [duplicate code reference](references/duplicate-code.md) "Cross-Boundary Duplication" section for the full 8-pattern catalog._

### Stack-Specific Findings

Group findings by stack-specific category when applicable: DI registrations, IBOutlet bindings, asset catalog entries, API routes, Tailwind utilities, EF Core navigations, unawaited coroutines, dead Helm values, etc.
→ _Consult [stack-specific patterns](references/stack-specific.md)._

### Deployment / Asset Dead Code

- Env vars declared but never read (`.env`, CI secrets, k8s ConfigMap/Secret)
- Config keys / Helm values / Terraform variables nothing consumes
- Dockerfile stages / K8s resources / CI jobs producing dead artifacts
- Database tables/columns/indexes nothing queries (combine static + production stats)
- HTTP routes (server-side) with no client caller
- Frontend routes with no `<Link>` / `navigate()` reaching them
- Localization keys / asset files (images, fonts, sounds) nothing references
- Shell scripts in `scripts/` no pipeline invokes
- Broken markdown links / stale documentation referencing removed features
→ _Consult [deployment dead code reference](references/deployment-dead-code.md)._

### Structural Smells (Spaghetti)

**Name structural smells that hide dead code; do not refactor here.** A god class with 30 unused methods isn't 30 findings — it's one structural smell containing dead code:

- God files / classes (>500 LOC, >20 methods)
- Cyclic dependencies between modules
- High cyclomatic complexity hot spots (CCN > 10)
- Layer violations (domain importing infrastructure)
- Anemic domain / dead architectural layers (DTO mirror of entity, repository wrapping ORM with no value-add)
- Divergent change / shotgun surgery (from git co-change analysis)

For each: point at the right tool (madge, lizard, import-linter, archunit, code-maat, dependency-cruiser) and surface the dead code inside the smell. **Defer the structural refactor to `/rust-critique`, `/dotnet-critique`, `/py-critique`, `/swift-critique`.**
→ _Consult [structural smells reference](references/structural-smells.md)._

### AI Slop Verdict

**If the project uses AI coding tools**, run the AI slop scorecard.
→ _Consult [AI slop patterns](references/ai-slop-patterns.md) for the full 12-pattern catalog and scorecard (now includes split-brain across stacks)._
Check for: copy-paste proliferation, wrapper functions adding nothing, single-impl interfaces, commented-out "previous attempts", excessive comments restating code, phantom edge case handling, reimplemented stdlib, orphaned AI-generated files, refactoring avoidance (v2/old/legacy naming), security theater in tests, unnecessary backward compatibility shims, and **split-brain implementations across stacks**. Report the scorecard result (Clean / Moderate / Heavy).

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
