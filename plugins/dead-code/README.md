# Dead Code Expert Plugin

Language-agnostic, stack-agnostic dead code detection. Finds unused symbols, parallel "split-brain" implementations across architectural seams, dead deployment artifacts (env vars, configs, K8s manifests, schemas, routes, assets), and surfaces structural smells that hide dead code.

## What It Does

A cleanup skill that identifies certainly dead code first, then escalates to higher-confidence duplicates, cross-boundary split-brain duplication, speculative abstractions, zombie test/debug artifacts, and deployment-level cruft. It emphasizes proof, false-positive awareness, and safe removal instead of blind deletion. When structural smells (god classes, cyclic dependencies, divergent change) hide dead code, the skill names the smell and surfaces the dead code inside — but defers structural refactoring to language-specific critique skills.

## Installation

```bash
# From the marketplace
claude plugin marketplace add /path/to/myClaudeSkills
claude plugin install dead-code@jko-claude-plugins

# Or load for one session
claude --plugin-dir /path/to/myClaudeSkills/plugins/dead-code
```

## Commands

| Command | Purpose |
|---|---|
| `/dead-code-scan` | Read-only scan grouped by confidence, category, stack, split-brain pairs, deployment artifacts, and structural smells |
| `/dead-code-clean` | Remove dead code, duplicates, zombie artifacts, split-brain pairs, and deployment cruft using configurable confidence modes |

## Skill

The `dead-code-expert` skill activates automatically when finding or removing dead code. It covers:

- **Source-symbol dead code:** unused imports, variables, functions, classes, types, unreachable code, dead branches, commented-out code, debug artifacts, lint suppressions
- **Duplicate / dual implementations** including cross-boundary "split-brain" (client↔server validation drift, DTO↔entity↔frontend type triplets, two services for the same resource, duplicate enums across languages)
- **Stack-specific dead code:** SwiftUI IBOutlet/asset/localization, Rust workspace/feature/visibility hygiene, React/Next.js dead routes/state/styles, .NET DI/EF Core/config/minimal-API, Python async/fixtures/Pydantic, C++ macros/CMake/preprocessor branches
- **Deployment / asset / infrastructure dead code:** env vars never read, dead Helm/K8s resources, Dockerfile cruft, dead Terraform variables, unused localization keys, orphan asset files, dead scripts, broken doc links, dead DB tables/columns/indexes
- **Structural smells (named, not refactored):** god classes, cyclic dependencies, layer violations, anemic domain, divergent change — defers to `/rust-critique`, `/dotnet-critique`, `/py-critique`, `/swift-critique`
- **False-positive awareness** for reflection, serialization, framework magic, DI containers, public API, lifecycle methods, FFI, declarative event handlers, code generation
- **AI slop catalog** including 12 patterns (copy-paste, single-impl interfaces, wrapper functions, reimplemented stdlib, security-theater tests, compat shims, **split-brain across stacks**)

## Supported Languages & Stacks

Python, JavaScript/TypeScript (incl. React, Next.js, Redux/Zustand/Jotai, Tailwind, GraphQL), Rust, Go, Swift/SwiftUI, C# / .NET (incl. EF Core, ASP.NET, gRPC, DI containers), Java, C/C++ (incl. CMake), and polyglot/microservice repos.

Per-stack tool integrations: knip, vulture, ruff, deptry, clippy, cargo-machete, cargo-modules, periphery, deadcode, staticcheck, Roslyn analyzers, IWYU, cppcheck, jscpd, lizard, semgrep, ast-grep, madge, pydeps, dependency-cruiser, import-linter, archunit, NetArchTest, code-maat, bloaty, dive, hadolint, lychee, and more.

## Hook

No active runtime hooks. `hooks/hooks.json` is reserved for future hook-based checks.

## References

10 reference files organized by domain:

- `detection-catalog.md` — 10-category catalog with per-language tools
- `false-positives.md` — 11-category checklist with scoring framework (incl. DI containers)
- `duplicate-code.md` — clone types, dual implementations, **cross-boundary split-brain catalog (8 patterns)**
- `stack-specific.md` — **NEW:** SwiftUI/iOS, Rust, TS/React/Next.js, .NET, Python, C++ patterns beyond standard linters
- `deployment-dead-code.md` — **NEW:** env vars, configs, Docker/K8s/Terraform, CI, DB, routes, assets, docs, scripts
- `structural-smells.md` — **NEW:** Fowler's 5 smell categories + architectural smells (cycles, layers, complexity, co-change)
- `ai-slop-patterns.md` — 12-pattern AI dead code catalog (incl. split-brain) with scorecard
- `grep-patterns.md` — ripgrep patterns for cycle detection, split-brain quick scan, deployment scan, complexity hot-spots
- `language-tools.md` — per-language tools, architecture-test libraries, cycle detection, co-change, bundle/binary inspection, deployment tools
- `safe-removal.md` — phased approach, library API considerations, monorepo concerns, Meta SCARF
- `prevention.md` — CI rules, review checklists, cultural norms

## License

MIT
