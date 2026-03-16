# Dead Code Detection Catalog

Complete catalog of dead code categories with detection techniques. Each entry includes what to look for, why it matters, and how to detect it.

## 1. Unused Imports / Includes / Requires

**What:** Import statements that bring in modules, functions, types, or packages that are never referenced in the file.

**Why it matters:** Unused imports increase startup time (Python loads modules eagerly), bundle size (JS without tree-shaking), compile time (C++/Rust), and mislead developers about dependencies.

**Detection by language:**

| Language | Pattern | Tool |
|---|---|---|
| Python | `import X` / `from X import Y` where X/Y unused | ruff F401, autoflake, vulture |
| JS/TS | `import { X }` / `import X` where X unused | ESLint no-unused-imports, knip |
| Rust | `use crate::X` where X unused | rustc (built-in warning) |
| Go | `import "X"` where X unused | Go compiler (build error) |
| Java | `import X.Y.Z` where Z unused | IntelliJ, checkstyle |
| C# | `using X` where X unused | Roslyn IDE0005 |
| Swift | `import X` where X unused | Xcode warning, periphery |
| C/C++ | `#include "X.h"` where nothing from X used | include-what-you-use (IWYU) |

**Side-effect imports (DO NOT flag):**
- Python: `from __future__ import annotations`, `import encodings`, gevent monkey-patching
- JS: `import './polyfill'`, `import './styles.css'`, `import 'reflect-metadata'`
- Rust: `use trait_crate::MyTrait` (may enable methods via trait)

## 2. Unused Variables & Dead Stores

**What:** Variables that are assigned a value but never read. Includes:
- Variables assigned and never referenced
- Variables assigned, then immediately reassigned before reading
- Function return values captured but never used
- Accumulator variables (e.g., list built up but never consumed)

**Why it matters:** Dead stores waste computation, mislead about data flow, and can mask bugs (variable name typo creates a new dead variable while the intended one stays stale).

**Detection patterns:**
- Variable declared/assigned with no subsequent read before end of scope or reassignment
- `let _ = expression` / `_ = expression` (explicit discard -- may be intentional if documented)
- Write-only struct fields (set in constructor, never read)

**Language-specific:**
- Python: ruff F841 (local variable assigned but never used)
- JS/TS: ESLint no-unused-vars
- Rust: rustc warns on unused variables (prefixed `_` to suppress)
- Go: compiler error on unused variables
- C#: Roslyn CS0168 (unused variable), IDE0059 (unnecessary assignment)

## 3. Unused Functions & Methods

**What:** Functions, methods, or procedures that are defined but never called from any code path.

**Why it matters:** Each unused function is code that must be read, understood, maintained, and tested -- for zero benefit. It clutters search results and IDE suggestions.

**Detection approach:**
1. Find all function definitions
2. For each, search the entire project for references
3. A function with references only at its definition is a candidate
4. Private/internal functions with zero callers are certainly dead
5. Public/exported functions require broader analysis (may have external consumers)

**Caveats:**
- Framework callbacks (event handlers, lifecycle hooks, decorators) may not have explicit callers
- Reflection-invoked methods won't appear in static search
- Test helper functions may only be called from test files
- Trait/interface implementations may be required even if not directly called

**Tools:**
- Python: vulture (project-wide)
- JS/TS: knip (unused exports)
- Rust: rustc dead_code warning
- Swift: periphery
- C#: NDepend, ReSharper
- Go: deadcode command

## 4. Unused Classes, Types, Structs, Enums

**What:** Type definitions that are never instantiated, extended, implemented, or referenced.

**Why it matters:** Unused types are the most expensive dead code -- they often come with constructors, methods, trait implementations, tests, and documentation, all of which are also dead.

**Detection:** Same as unused functions but searching for type references (instantiation, type annotations, inheritance, generic parameters).

**Special cases:**
- Unused enum variants (variant defined but never constructed or matched)
- Unused struct fields (field exists but never read -- only written)
- Interfaces/traits with zero implementations (speculative generality)
- Interfaces/traits with exactly one implementation (may be premature abstraction)

## 5. Unreachable Code

**What:** Code that can never execute regardless of input.

**Patterns:**
- Code after unconditional `return`, `break`, `continue`, `throw`, `exit`, `panic!`
- Dead branches: `if (false) { ... }`, `if (true) { ... } else { DEAD }`
- Conditions that are always true/false due to type constraints
- Unreachable `catch`/`except` blocks (exception type can never be thrown)
- Unreachable `match`/`switch` arms (all cases already covered)
- Code after infinite loops (`while(true)` / `loop {}`) without break

**Why it matters:** Unreachable code misleads about possible execution paths. It can mask logic errors (the developer may have intended the code to be reachable).

**Tools:** Most compilers and linters detect basic unreachable code. ESLint no-unreachable, rustc unreachable_code, mypy/pyright unreachable warnings.

## 6. Commented-Out Code

**What:** Source code that has been commented out rather than deleted. NOT documentation comments, NOT `TODO`/`FIXME` annotations (those are separate).

**Why it matters:**
- "Version control is your backup, delete commented code" (Kent Dodds, Uncle Bob)
- Commented code gets stale immediately -- surrounding code changes but the comment doesn't
- It's visual noise that slows reading (less production code visible per screen)
- Developers are afraid to delete it, so it accumulates ("zombie code")
- It clutters search results

**Detection heuristics:**
- Multi-line comments containing syntactically valid code (assignments, function calls, control flow)
- Single-line comments that look like code: `// const x = ...`, `# import ...`, `// if (...)`
- Comments containing common code patterns: `=`, `()`, `{}`, `[]`, `->`, `=>`, `::`, `;`
- Large blocks of `//` comments (5+ consecutive lines) that aren't documentation

**Regex patterns (heuristic, cross-language):**
```
# Consecutive commented lines (likely code, not docs)
^\s*(\/\/|#)\s*(const|let|var|function|class|import|from|if|for|while|return|def|fn|pub|private|protected)\b

# Commented-out function calls
^\s*(\/\/|#)\s*\w+\.\w+\(

# Commented-out assignments
^\s*(\/\/|#)\s*\w+\s*=\s*
```

## 7. Dead Test Code

**What:** Test-related code that provides no value.

**Categories:**
- **Skipped/disabled tests**: `@skip`, `xit`, `xdescribe`, `#[ignore]`, `[Ignore]`, `@pytest.mark.skip`
- **Unused test fixtures/helpers**: Setup functions, factory functions, mock definitions never used
- **Orphaned test files**: Test files whose corresponding source was deleted
- **Dead assertions**: Assertions that can never fail (e.g., `assert True`, `expect(true).toBe(true)`)
- **Tests for deleted features**: Tests that exercise code paths that no longer exist

**Tools:**
- Python: pytest-deadfixtures, pytest-unused-fixtures, test-linter
- JS: knip can detect unused test utilities
- All: Search for test files with no corresponding source file

## 8. Debug Artifacts

**What:** Debugging code left in production paths.

**Patterns by language:**

| Language | Debug Artifacts |
|---|---|
| Python | `print()`, `breakpoint()`, `pdb.set_trace()`, `import pdb` |
| JS/TS | `console.log()`, `console.debug()`, `debugger;` |
| Rust | `dbg!()`, `println!()` in non-test code, `todo!()`, `unimplemented!()` |
| Swift | `print()`, `dump()`, `#if DEBUG` blocks in release |
| C# | `Console.WriteLine()`, `Debug.Log()`, `System.Diagnostics.Debug.Write()` |
| Java | `System.out.println()`, `e.printStackTrace()` |
| Go | `fmt.Println()` for debugging (should use structured logging) |

**Also flag:**
- `TODO`, `FIXME`, `HACK`, `XXX`, `TEMP`, `TEMPORARY` comments (track count, not necessarily dead but indicate incomplete work)
- `#[allow(dead_code)]`, `#[allow(unused)]`, `// eslint-disable no-unused-vars` -- lint suppressions that hide dead code

## 9. Dead Configuration & Orphaned Files

**What:** Files that exist in the project but are never referenced by anything.

**Patterns:**
- Source files not imported by any other file
- Config files for removed tools/features
- Migration scripts that have been superseded
- Asset files (images, fonts) not referenced in code or templates
- Script files not referenced in package.json, Makefile, justfile, etc.

**Detection:** Start from entry points (main, index, package.json scripts) and trace the dependency graph. Any file not reachable is a candidate. Tools like knip do this automatically for JS/TS.

## 10. Lint Suppressions Hiding Dead Code

**What:** Compiler/linter warnings that have been suppressed rather than addressed.

**Patterns:**
```
# Rust
#[allow(dead_code)]
#[allow(unused_imports)]
#[allow(unused_variables)]

# Python
# noqa: F401 (unused import)

# JS/TS
// eslint-disable-next-line no-unused-vars
/* eslint-disable no-unused-vars */

# C#
#pragma warning disable CS0168
```

**Why it matters:** Every suppression should be investigated. The code is dead, and the developer knew it -- they just chose to hide the warning instead of fixing it. In Rust 1.81+, prefer `#[expect(lint)]` over `#[allow(lint)]` so the suppression warns when it becomes stale.
