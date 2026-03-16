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
