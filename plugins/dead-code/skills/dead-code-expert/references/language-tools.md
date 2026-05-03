# Language-Specific Dead Code Tools

Per-language tool recommendations and compiler/linter configurations for dead code detection.

## JavaScript / TypeScript

### Primary: knip

The most comprehensive dead code detector for JS/TS projects. Finds unused files, exports, dependencies, types, duplicates, and class members.

```bash
npx knip                    # Full scan
npx knip --fix              # Auto-remove unused exports
npx knip --include files    # Only report unused files
npx knip --include exports  # Only report unused exports
```

Knip uses TypeScript's compiler API for import resolution. It understands 100+ frameworks/tools (Jest, Storybook, Next.js, Vite, etc.) so it won't flag framework-required files as unused.

### Complementary tools

- **ESLint**: `no-unused-vars`, `no-unused-imports` (per-file analysis)
- **ts-prune**: Find unused exports (superseded by knip)

### Tree-shaking

Bundlers (webpack, rollup, esbuild) eliminate dead code during builds. But tree-shaking only works with ES modules and side-effect-free code. Check `"sideEffects": false` in package.json for libraries.

### Configuration for maximum detection

```json
// .eslintrc
{
  "rules": {
    "no-unused-vars": ["error", { "argsIgnorePattern": "^_" }],
    "no-unreachable": "error",
    "no-unused-expressions": "error"
  }
}
```

### React / Next.js / state management

React-specific dead code (useState with no setter, useEffect with no effect, dead Context providers, dead reducer cases, dead Next.js routes/server actions, unused Tailwind utilities, dead GraphQL queries) is covered in `stack-specific.md`.

## Python

### Primary: vulture (project-wide dead code)

```bash
vulture src/ tests/ --min-confidence 80
```

Vulture performs whole-project static analysis. It finds unused functions, classes, variables, imports, and unreachable code. Set `--min-confidence` to reduce false positives.

### Complementary tools

- **ruff**: `ruff check --select F401,F841` for unused imports (F401) and unused variables (F841). Replaces autoflake.
- **pyflakes**: Lightweight, finds unused imports and variables

### Framework-specific challenges

- Django: views referenced in urls.py via strings, models used by ORM, management commands loaded by name
- Flask: routes registered via decorators, not import chains
- SQLAlchemy: models may only be referenced by migration files

### Whitelist pattern for vulture

Create a `whitelist.py` with false positives:

```python
# whitelist.py - tell vulture these are used
from myapp.models import User  # Used by Django ORM
User.objects  # Accessed dynamically
```

### Async / framework / dependency drift

Unawaited coroutines, stale `TYPE_CHECKING` imports, unused FastAPI `Depends`, unused pytest fixtures, Celery tasks never invoked, `__all__` mismatches, and `requirements.txt`/`pyproject.toml` drift are covered in `stack-specific.md`. Use `deptry` for dependency drift, `pytest-deadfixtures` for unused fixtures.

## Rust

### Built-in: rustc dead code warnings

Rust has the most aggressive built-in dead code detection of any mainstream language.

```toml
# Cargo.toml - recommended lint config
[lints.rust]
dead_code = "warn"
unused_imports = "warn"
unused_variables = "warn"
unused_mut = "warn"
unreachable_code = "warn"

[lints.clippy]
unused_self = "warn"
```

### Complementary tools

- **clippy**: `cargo clippy` catches unused_self, redundant closures, unnecessary wraps
- **cargo-udeps**: Find unused dependencies (`cargo +nightly udeps`)
- **cargo-machete**: Faster alternative to cargo-udeps (no nightly required)

### Key Rust patterns

- `#[allow(dead_code)]` hides dead code -- replace with `#[expect(dead_code)]` in Rust 1.81+
- Unused trait imports: `use MyTrait;` where none of the trait's methods are called on any type

### Workspace / feature / public-API hygiene

Always-on/always-off feature flags, unused `target_os` branches, orphan workspace members, dead `examples/`, unused trait bounds, lifetime-only generics, over-broad visibility, and `build.rs` cfg flags with no consumer are covered in `stack-specific.md`. Use `cargo public-api` to track public surface and `cargo modules` to visualize crate graph.

## Go

### Built-in: compiler enforced

Go is unique -- unused imports and unused variables are **compile errors**, not warnings. This prevents accumulation of the most common dead code patterns.

### Tools for deeper analysis

- **deadcode**: `go install golang.org/x/tools/cmd/deadcode@latest && deadcode ./...` Reports unreachable functions via whole-program reachability analysis.
- **staticcheck**: `staticcheck ./...` finds unused struct fields, parameters, results
- **golangci-lint**: Meta-linter aggregating multiple tools

## Swift

### Primary: periphery

```bash
periphery scan --project MyApp.xcodeproj --schemes MyApp
```

Periphery finds unused declarations across entire Swift projects. It understands SwiftUI, UIKit, and Objective-C bridging.

### Xcode built-in

Xcode reports unused variables and some unused functions via compiler warnings. Enable "Treat Warnings as Errors" in CI.

### SwiftUI-specific dead code

- Unused `@State`, `@Binding`, `@Published` properties
- Views defined but never used in any navigation hierarchy
- Unused `Environment` values

### Asset / resource / IB hygiene

Dead IBOutlets / IBActions referenced from `.storyboard`/`.xib` by string, orphan `Assets.xcassets` entries, unused localization keys (`.strings`/`.xcstrings`), stale `Info.plist` entries, unused `ViewModifier`/`ButtonStyle`, dead Combine subscriptions, and unused `EnvironmentKey`s are covered in `stack-specific.md`.

## C# / .NET

### Primary: Roslyn analyzers

```xml
<!-- .editorconfig -->
dotnet_diagnostic.IDE0051.severity = warning  # Unused private members
dotnet_diagnostic.IDE0052.severity = warning  # Unread private members
dotnet_diagnostic.CS0168.severity = warning   # Unused variables
dotnet_diagnostic.IDE0005.severity = warning  # Unused using directives
```

### Complementary tools

- **NDepend**: Enterprise-grade, CQLinq queries for dead code patterns
- **ReSharper/Rider**: "Solution-Wide Analysis" detects unused types and members
- **dotnet-format**: Can remove unused usings

### ASP.NET challenges

- Controllers discovered by convention (not explicit import)
- Razor views referenced by string name
- Dependency injection resolves types at runtime

### DI / EF Core / configuration / minimal API hygiene

DI registrations with no consumer (and the inverse), dead EF Core navigations / `DbSet`s / shadow properties, `appsettings.json` keys nothing binds, `IOptions<T>` with no JSON, minimal API endpoints with no client caller, gRPC RPCs unused, no-op `BackgroundService`s, passthrough middleware, dead `.resx` entries, unused `<PackageReference>`s, and Razor partials/components never invoked are covered in `stack-specific.md`. Use `PublishTrimmed`/`PublishAot` to surface trim warnings (`IL2026`, `IL3050`).

## Java

### Tools

- **IntelliJ IDEA**: "Unused declaration" inspection (surprisingly thorough)
- **SpotBugs**: Bytecode-level dead code detection
- **PMD**: `UnusedPrivateField`, `UnusedLocalVariable`, `UnusedFormalParameter`
- **ProGuard/R8**: Dead code elimination for Android

## C / C++

### Tools

- **include-what-you-use (IWYU)**: Unused `#include` detection
- **cppcheck**: Unused functions, variables, struct members
- **clang-tidy**: `misc-unused-*` checks
- **PVS-Studio**: Deep unused code analysis
- **unifdef**: Statically resolve `#ifdef` branches when macro values are known
- **bloaty**: Per-symbol binary size — finds link-time dead code

### Macro / preprocessor / build-system hygiene

Unused `#define` macros, always-true/always-false `#ifdef` branches, dead CMake `target_link_libraries`, orphan CMake targets, generated-code masking (Qt MOC, protobuf), pimpl-with-single-impl, dead `friend` declarations, unused operator overloads, and header-include leak (forward-declaration sufficient) are covered in `stack-specific.md`. Use `--gc-sections` + `-Wl,--print-gc-sections` at link time to surface unused sections.

## Cross-Language / Polyglot

### Semgrep

Write rules that work across languages using Semgrep's generic AST:

```yaml
rules:
  - id: commented-out-code
    pattern: |
      // $CODE = $EXPR
    message: "Possible commented-out code"
    languages: [javascript, typescript, java, c, cpp]
    severity: WARNING
```

### ast-grep

Structural search/replace using tree-sitter. Can write language-specific rules for dead code patterns in YAML config.

### jscpd (token-based clone detection)

```bash
npx jscpd --min-lines 5 --threshold 5 --reporters html,console .
```

Multi-language. Detects Type 1 and Type 2 clones across `.ts`, `.py`, `.cs`, `.rs`, `.swift`, `.cpp`, etc.

### lizard (complexity)

```bash
pip install lizard
lizard -C 10 -L 50 -a 5 src/
```

Multi-language complexity / function-length / parameter-count scanner. Use to find god files where dead code hides.

---

## Architecture Test Libraries

These libraries encode rules like "domain layer must not import infrastructure" as tests that fail in CI. They surface **layer violations** that often hide dead abstraction layers (ports without adapters, adapters without ports).

| Stack                   | Library                                                                                                             | Notes                                            |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------ |
| JVM (Java/Kotlin/Scala) | [archunit](https://www.archunit.org/)                                                                               | The canonical arch-test library                  |
| .NET                    | [NetArchTest](https://github.com/BenMorris/NetArchTest), [ArchUnitNET](https://archunitnet.readthedocs.io/)         | xUnit / NUnit compatible                         |
| TypeScript / JavaScript | [ts-arch](https://github.com/ts-arch/ts-arch), [dependency-cruiser](https://github.com/sverweij/dependency-cruiser) | dependency-cruiser also detects orphans + cycles |
| Python                  | [import-linter](https://import-linter.readthedocs.io/), [pytestarch](https://github.com/zyskarch/pytestarch)        | import-linter has layered architecture contracts |
| PHP                     | [deptrac](https://github.com/qossmic/deptrac)                                                                       | YAML-defined layer contracts                     |
| Go                      | [go-cleanarch](https://github.com/roblaszczak/go-cleanarch)                                                         | Clean-architecture enforcer                      |
| Rust                    | [cargo-modules](https://github.com/regexident/cargo-modules) (visualize) + manual `pub(crate)` discipline           | No mainstream arch-test; rely on visibility      |
| Multi                   | [Sonargraph](https://www.hello2morrow.com/products/sonargraph)                                                      | Commercial; many languages                       |

---

## Cycle Detection (Specialized)

| Stack  | Tool                                                                 |
| ------ | -------------------------------------------------------------------- |
| JS/TS  | [madge](https://github.com/pahen/madge), dependency-cruiser          |
| Python | [pydeps](https://github.com/thebjorn/pydeps), pylint `cyclic-import` |
| Rust   | [cargo-modules](https://github.com/regexident/cargo-modules)         |
| Go     | [goda](https://github.com/loov/goda)                                 |
| C/C++  | `cinclude2dot` + `dot`                                               |
| .NET   | NDepend (commercial), `dotnet-graph`                                 |

---

## Co-Change / Behavioral Coupling

Detects shotgun surgery and divergent change from VCS history:

- [code-maat](https://github.com/adamtornhill/code-maat) — open source, CSV outputs
- [CodeScene](https://codescene.com/) — commercial, hotspot analysis
- Quick command:
  ```bash
  git log --format=format: --name-only --since=6.months -- path/to/file \
    | sort | uniq -c | sort -rn | head -20
  ```

---

## Database / Schema Drift

| Stack      | Tool                                                                       |
| ---------- | -------------------------------------------------------------------------- |
| PostgreSQL | `pg_stat_user_tables`, `pg_stat_user_indexes` for unused tables/indexes    |
| MySQL      | `pt-index-usage`, `sys.schema_unused_indexes`                              |
| Multi      | [SchemaCrawler](https://www.schemacrawler.com/) for static schema analysis |

---

## Bundle / Binary Inspection

Reveal what survived linking / tree-shaking — anything in the output that nobody calls is link-time dead code.

| Stack               | Tool                                                                                         |
| ------------------- | -------------------------------------------------------------------------------------------- |
| JS/TS               | `webpack-bundle-analyzer`, `source-map-explorer`, `rollup-plugin-visualizer`, `bundlephobia` |
| Native (C/C++/Rust) | [bloaty](https://github.com/google/bloaty) — per-symbol size breakdown                       |
| iOS                 | Xcode `LinkMap.txt`, `link-map-parser`                                                       |
| .NET                | `ILSpy`, `dotPeek`; `PublishTrimmed` warnings                                                |
| Docker              | [dive](https://github.com/wagoodman/dive) — per-layer file analysis                          |

---

## Deployment / Infrastructure

| Stack          | Tool                                                                         |
| -------------- | ---------------------------------------------------------------------------- |
| Dockerfile     | `hadolint`, `dive`, `docker-slim`                                            |
| Kubernetes     | `kube-score`, `kubeval`, `polaris`, `datree`                                 |
| Terraform      | `tflint` (built-in unused-variable check), `checkov`, `terraform-compliance` |
| Helm           | `helm template` + value reachability grep (see `deployment-dead-code.md`)    |
| GitHub Actions | `actionlint` for syntax, manual graph for unused workflows                   |
| Localization   | `i18next-scanner`, `formatjs cli` for unused keys                            |
| Markdown links | `lychee`, `markdown-link-check`                                              |
