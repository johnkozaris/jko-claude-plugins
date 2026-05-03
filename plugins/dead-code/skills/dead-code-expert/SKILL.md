---
name: dead-code-expert
description: This skill should be used when the user wants to find, audit, or remove dead code, unused imports, unused functions, unused variables, duplicate implementations, parallel/split-brain implementations across architectural boundaries, dead assets/configuration/infrastructure, or simplify a codebase. Works across all programming languages and stacks (SwiftUI, Rust, TypeScript/React, .NET, Python, C++, Go, Java). Relevant when the user says "find dead code", "remove dead code", "remove unused imports", "find duplicate code", "split brain", "parallel implementations", "client server validation drift", "dead config", "orphan assets", "unused environment variables", "simplify this codebase", "find unused functions", "find unused code", "remove commented out code", "what code is unused", "find orphaned files", "detect duplicate implementations", "find unreachable code", "clean up this codebase", or "audit for unused code".
---

# Dead Code Expert

Find and eliminate dead code, duplicate implementations, parallel split-brain logic across architectural boundaries, dead deployment artifacts (env vars, configs, manifests, schemas, routes, assets), and unnecessary complexity across any programming language and stack. Every finding explains WHY the code is dead and the concrete cost of keeping it (cognitive overhead, build time, misleading developers, masking bugs, drift bugs).

## How to Think About Dead Code

Before flagging anything, identify which category it belongs to:

- **Layer 1 -- Certainly Dead:** Unreachable code after return/break/throw, unused private functions, unused local variables, imports with zero references. Safe to remove immediately.
- **Layer 2 -- Probably Dead:** Exported functions with no callers in the project, commented-out code blocks, `#[allow(dead_code)]` / `// eslint-disable unused` suppressions, permanently-off feature flags. Verify before removing.
- **Layer 3 -- Suspiciously Alive:** Code that LOOKS dead but may be used via reflection, serialization, framework magic, dynamic dispatch, DI container resolution, or public API surface. Investigate before touching.
- **Layer 4 -- Hidden by Spaghetti:** Dead code locked inside large structural smells (god classes, cyclic imports, divergent change). The structural smell makes the dead code invisible. Surface the smell first, then the dead code inside.
  → _Consult [false-positives reference](references/false-positives.md) for the full 11-category checklist and scoring framework._
  → _Consult [structural smells reference](references/structural-smells.md) when dead code is hidden inside spaghetti._

When dead code is found, reframe it as a design question:

| Dead Code Pattern               | Don't Just Say      | Ask Instead                                                    |
| ------------------------------- | ------------------- | -------------------------------------------------------------- |
| Unused function                 | "Delete it"         | Why was it written? Is there a missing caller?                 |
| Duplicate implementation        | "Remove one"        | Which is canonical? Why did duplication happen?                |
| Commented-out block             | "Delete it"         | Is there in-progress work? Check git blame.                    |
| Unused abstraction layer        | "Inline it"         | Was it speculative generality?                                 |
| Dead feature flag               | "Remove the branch" | Is there a deprecation process to follow?                      |
| Same logic in client and server | "Pick one"          | Where's the canonical source of truth? Should this be codegen? |
| Stale manifest entry / env var  | "Delete it"         | Is this read by a runtime convention or framework?             |

## Detection Process

When scanning for dead code, work through these categories in order.

1. **Unused Imports** -- Imports/includes/requires with no reference in file.
   → _Consult [grep patterns](references/grep-patterns.md) for per-language detection._

2. **Unused Variables & Parameters** -- Assigned but never read (dead stores).
3. **Unused Functions & Methods** -- Defined but never called.
4. **Unused Classes & Types** -- Defined but never instantiated or referenced.
   → _Consult [detection catalog](references/detection-catalog.md) for categories 2-6 with per-language tools._

5. **Unreachable Code** -- Code after return/break/throw, dead branches (always-true/false conditions).
6. **Commented-Out Code** -- Code blocks in comments (not documentation).

7. **Duplicate / Dual Implementations** -- Same logic implemented twice differently. Includes cross-boundary "split-brain" duplication (client↔server validation, DTO↔entity↔frontend type triplets, two services solving the same problem, duplicate enums across languages).
8. **Speculative Generality** -- Abstractions used in exactly one place, interfaces with single implementation, unused parameters kept "for future use".
   → _Consult [duplicate code reference](references/duplicate-code.md) for clone types, dual implementation patterns, cross-boundary split-brain catalog, and DRY escalation._

9. **Dead Test Code** -- Skipped tests, unused fixtures, orphaned test files.
10. **Debug Artifacts** -- `console.log`, `print()`, `dbg!()`, `TODO`/`FIXME` markers left in production code.

11. **Dead Assets, Configuration, & Infrastructure** -- Unused environment variables, config keys nothing reads, asset files / localization keys / images nothing references, Dockerfile stages whose output is discarded, Kubernetes resources nothing routes to, Terraform variables/modules with no consumer, CI/CD steps producing artifacts nothing uses, database tables/columns nothing queries, HTTP routes with no caller, frontend routes with no `<Link>`, scripts in `scripts/` no pipeline invokes.
    → _Consult [deployment dead code reference](references/deployment-dead-code.md) for per-artifact-type detection._

## Stack-Specific Patterns

Source-symbol detection misses framework- and build-system-level dead code: DI registrations with no consumers, IBOutlets bound to deleted views, API routes with no client, Helm values nothing templates, dead Tailwind utilities, unawaited coroutines, EF Core navigations never traversed.
→ _Consult [stack-specific patterns](references/stack-specific.md) for SwiftUI/iOS, Rust, TypeScript/React/Next.js, C#/.NET, Python, and C/C++ detection beyond standard linters._

## Thinking Prompts

Before removing code, work through:

1. **Is this genuinely dead?** Check for reflection, serialization, dynamic imports, framework conventions, DI container resolution, public API consumers.
   → _Consult [false-positives reference](references/false-positives.md) before every medium-confidence removal._
2. **Why does this exist?** Check `git log` / `git blame`. If someone wrote it recently, it might be in-progress work. If it's years old with no references, it's dead.
3. **What's the cost of keeping it?** Cognitive overhead for every developer who reads it. Misleading grep results. False confidence from tests that exercise dead paths. Build time for code nobody uses. **For split-brain duplication: the cost is silent drift bugs in production.**
4. **Is this hidden by a bigger smell?** A god class with 30 unused methods isn't 30 separate findings — it's one structural smell. Name the smell, then list the dead code inside.

## Confidence Levels

Label every finding:

- **certain** -- Compiler/linter confirms it (unused import, unreachable after return). Remove immediately.
- **high** -- No references found in project-wide search. Remove after quick verification.
- **medium** -- Might be used via dynamic means (reflection, templates, DI container, string-based lookup). Investigate first.
- **low** -- Potentially used by external consumers (library public API, plugin interface, external API caller). Do not remove without understanding consumers.

## Output Format

Group findings by file (or by artifact for deployment-level findings). For each finding:

1. File path and line number (or artifact path)
2. Confidence level
3. Category (from the 11 categories above)
4. **What is dead** -- name the specific symbol, block, artifact, or pattern
5. **Why it's dead** -- the evidence (zero references, unreachable, no consumer, etc.)
6. **Cost of keeping it** -- cognitive overhead, misleading results, build time, masking bugs, drift risk
7. Recommended action (delete, inline, consolidate, investigate, generate-from-source)

End with a prioritized summary: certain items first, then high confidence, then medium. For split-brain findings, recommend a canonical source and a codegen / shared-package strategy.

## The AI Slop Test

If a codebase uses AI coding tools, check for AI-specific dead code fingerprints: copy-pasted logic across services, wrapper functions adding nothing, single-impl interfaces, commented-out "previous attempts", excessive restating comments, phantom edge case handling, reimplemented stdlib, orphaned files, refactoring avoidance (`v2`/`_old`/`_legacy` naming), security theater in tests, unnecessary backward compatibility shims kept after a migration is complete, and **split-brain implementations** (same constant, enum, schema, or validation rule duplicated across stacks without codegen).
→ _Consult [AI slop patterns](references/ai-slop-patterns.md) for the full catalog with detection commands and scorecard._

## Structural Smells (When Spaghetti Hides Dead Code)

The dead-code skill stays focused on detection and removal — but structural smells (god class, cyclic dependency, divergent change, layer violation, anemic domain model) often **hide** dead code by making it unreadable. When you encounter these:

1. **Name the smell** in the report.
2. **Point at the right tool** (madge, lizard, import-linter, archunit, code-maat, dependency-cruiser).
3. **Surface the dead code inside** (god class → list its unused methods; cycle → list orphans the cycle hides).
4. **Defer the structural refactoring** to a language-specific critique skill (`/rust-critique`, `/dotnet-critique`, `/py-critique`, `/swift-critique`).

→ _Consult [structural smells reference](references/structural-smells.md) for the smell taxonomy and tool mapping._

## Safe Removal & Prevention

→ _Consult [safe removal strategies](references/safe-removal.md) for phased approach, library API considerations, and monorepo concerns._
→ _Consult [prevention practices](references/prevention.md) for CI rules, review checklists, and cultural norms._

## Language Detection

Detect the project language(s) from file extensions, config files, and directory structure. For multi-language projects, scan each language independently and **scan the seams between them for split-brain duplication.**
→ _Consult [language tools](references/language-tools.md) for per-language tool configs, compiler settings, and architecture-test libraries._

---

Approach every finding as a meticulous code archaeologist. Dead code is not just clutter -- it misleads, it hides bugs, it wastes every developer's time, and split-brain duplication causes silent drift in production. Hunt with precision, verify with evidence, remove with confidence. The best line of code is the one you delete.
