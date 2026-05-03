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
→ *Consult [false-positives reference](references/false-positives.md) before every removal in this phase — includes DI container resolution as category 11.*

### 3a. Unused Exported Functions
Functions exported/public in applications (not libraries) with zero external callers. Search thoroughly including templates, configs, DI container registrations, and dynamic references before removing.

### 3b. Orphaned Files
Files not imported by anything. Verify they're not entry points, config files, DI-discovered services, or framework-discovered modules before removing.

### 3c. Duplicate Implementations
Identify functions doing the same thing.
→ *Consult [duplicate code reference](references/duplicate-code.md) for clone types and consolidation patterns.*
Choose the canonical implementation, update callers, remove the duplicate.

### 3d. Speculative Generality
Interfaces with single implementation, unused parameters, wrapper functions adding no value. Inline or remove.

### 3e. AI Slop Cleanup
If the project uses AI coding tools, check for AI-specific dead code patterns.
→ *Consult [AI slop patterns](references/ai-slop-patterns.md) for the full catalog including split-brain across stacks.*
Target: copy-paste proliferation, wrapper functions adding nothing, reimplemented stdlib, excessive restating comments, phantom edge case handling, split-brain implementations.

### 3f. Stack-Specific Dead Code
Apply per-stack patterns from `references/stack-specific.md`. Examples:
- **SwiftUI:** Remove orphan `Assets.xcassets` entries, dead localization keys, unused `EnvironmentKey`s, unused `ViewModifier`s.
- **Rust:** Remove orphan workspace members, dead `examples/`, always-on/always-off `#[cfg(feature = "...")]` branches, over-broad `pub` visibility.
- **TS/React:** Remove `useState` whose setter is never called, dead `Context.Provider`s, dead reducer cases, unused Tailwind utilities, unused barrel re-exports.
- **.NET:** Remove DI registrations with no consumer, dead `DbSet`/navigation properties, `appsettings.json` keys nothing binds, no-op middleware.
- **Python:** Add missing `await`s for unawaited coroutines (or delete the call if effect is unwanted), remove stale `TYPE_CHECKING` imports, unused FastAPI `Depends`, unused fixtures.
- **C++:** Remove dead `#define`s, prove `#ifdef` branches with `unifdef` then collapse, remove dead CMake link libraries.

## Phase 4: Cross-Boundary / Split-Brain Cleanup (aggressive only)

Only execute if mode is 'aggressive' AND the project spans multiple stacks. **High-risk phase — every change crosses an architectural seam.**
→ *Consult [duplicate code reference](references/duplicate-code.md) "Cross-Boundary Duplication" section.*

### 4a. Identify Split-Brain Pairs
```bash
# Same enum-like declarations across languages
rg -o '(enum|class)\s+(\w+)' -t ts -t py -t cs -t swift --no-filename \
  | sort | uniq -c | sort -rn | awk '$1 >= 2 {print $1, $2}'

# Same field names across model/dto/schema/type directories
for d in models schemas dto types entities; do
  fd -t f . "$d" 2>/dev/null
done | xargs rg -o '^\s*(\w+):' --no-filename -r '$1' 2>/dev/null \
  | sort | uniq -c | sort -rn | awk '$1 >= 3'

# Two route handlers covering same path
rg -o "(get|post|put|delete|patch|MapGet|MapPost|app\.route)\(['\"]([^'\"]+)" \
  --no-filename -r '$2' | sort | uniq -c | sort -rn | awk '$1 >= 2'
```

### 4b. Choose a Canonical Source
For each split-brain pair, pick one canonical source per the rules below. **Do not delete either side without picking the canonical.**

| Pattern | Default canonical |
|---|---|
| Enum / status / type set | Protobuf / OpenAPI / shared schema package |
| Validation rules | Backend (defense-in-depth requires server-side; mirror to client via codegen) |
| DTO ↔ Entity mapping | Generate DTO from entity (or generate both from a schema) |
| Two services for same resource | Newer service (with explicit deprecation timer for old) |
| Read path vs write path | Add missing field to read model OR remove from write model — don't leave drift |
| Two config sources | Pick one; document precedence loudly if both are intentional |
| Old API + new API | Deprecate old; use access logs to confirm zero callers; then remove |

### 4c. Introduce Codegen / Shared Package
- Add codegen step to CI (`buf generate` for Protobuf, `openapi-typescript` for OpenAPI → TS, `quicktype` for JSON Schema → multi-lang).
- Or: extract shared logic into a package consumed by both halves (npm workspace package, Python package, shared Rust crate compiled to WASM for browser).

### 4d. Migrate Callers
1. Add the canonical source.
2. Migrate one consumer at a time to the canonical source.
3. Verify with build + tests after each migration.
4. Once all consumers migrated, delete the duplicate.

**Do not skip the migration step — deleting the duplicate first will break the consumer that still references it.**

## Phase 5: Deployment / Asset Cleanup (aggressive only, project-aware)

Only execute if mode is 'aggressive' AND the user has confirmed they want non-source-file cleanup. **Highest risk — operational artifacts can have non-obvious consumers.**
→ *Consult [deployment dead code reference](references/deployment-dead-code.md).*

### 5a. Dead Environment Variables
1. Inventory declared env vars (`.env*`, k8s `ConfigMap`/`Secret`, Helm values, Docker Compose).
2. Inventory read sites in source.
3. For each declared-but-unread var: confirm with the user / operator before removing (some may be runtime-conventional).

### 5b. Dead Configuration Keys
For `appsettings.json` / `config.yaml` / `pyproject.toml [tool.*]`: walk every leaf key, grep for it as a string literal in the appropriate language. Confirmed-unread keys can be removed.

### 5c. Dead Asset Files
For files in `public/` / `assets/` / `static/` / `Assets.xcassets`: grep the basename across source and templates. Confirmed-orphan assets can be removed (git history preserves them).

### 5d. Dead Localization Keys
For `.json` / `.po` / `.strings` / `.xcstrings`: walk every key, grep for it. Confirmed-unread keys can be removed.

### 5e. Dead Scripts
For files in `scripts/`: grep filename across pipelines, justfile, package.json, Makefile, README. Orphans can be removed.

### 5f. Stale Documentation Links
Run `lychee --no-progress './**/*.md'` (or equivalent) to find broken markdown links. Fix or remove.

**DO NOT** auto-remove without user confirmation:
- Database migrations (chain integrity).
- Helm/K8s/Terraform resources (require operational verification).
- Public HTTP routes (require traffic analysis from access logs).
- Feature flags (require flag-management-system check).
- Anything in `infra/` / `deploy/` / `terraform/` / `k8s/` directories without explicit user approval.

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
- X stack-specific items cleaned (DI registrations, dead assets, unused EnvironmentKeys, etc.)
- X split-brain pairs unified (with canonical source named)
- X dead env vars / config keys / asset files / scripts removed
- **Total: X lines / artifacts removed**

State any items skipped due to false-positive risk, with explanation. For split-brain unifications, document the canonical source chosen and the codegen / shared-package approach used.

**NEVER**:
- Remove public API in libraries without user confirmation
- Remove code with framework decorators / DI markers without understanding the framework
- Skip build + test verification between phases
- Make large bulk deletions -- work file by file so failures are easy to isolate
- Remove code flagged as medium confidence without checking the false-positives reference
- Mix dead code removal with feature changes in the same edit
- Remove split-brain duplicates without first migrating consumers to the canonical source
- Auto-remove deployment artifacts (env vars, K8s resources, migrations, public routes) without explicit user confirmation
- Delete database migrations (the chain breaks for fresh installs)
