# Structural Smells (Spaghetti Code)

Structural / architectural smells that _produce_ dead code as a downstream effect. The dead-code skill stays focused on detection and removal, but it should **recognize** these smells, name them precisely, and point at the right specialized tool. Many language-specific `*-critique` plugins (Rust, .NET, Python, SwiftUI) are better homes for the actual refactoring work — this reference is the bridge.

The taxonomy follows Fowler's _Refactoring_ and Suryanarayana's _Refactoring for Software Design Smells_. Refactoring.Guru groups smells into 5 buckets; the dead-code-expert skill itself only covers the **Dispensables** bucket. The other four are listed here so they're not invisible during a scan.

---

## Why this matters for dead-code work

A god class has dead methods nobody noticed because the class is too big to read. A cyclic import graph hides orphan files because grep doesn't see the cycle. A module with shotgun-surgery coupling has parallel implementations of the same logic split across files. **Spaghetti makes dead code invisible.** Naming the structural smell is the first step to surfacing the dead code inside it.

---

## Smell Categories (Fowler / Refactoring.Guru)

### Bloaters — code grew beyond reasonable size

| Smell                        | Signal                                                         | Detection                                                                                                                                    |
| ---------------------------- | -------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| **Long Method**              | Function > ~50 LOC                                             | `lizard --CCN 1 -L 50` (multi-lang); `eslint complexity max-lines-per-function`; `radon cc -a -s` (Python); `clippy::too_many_lines` (Rust). |
| **Large Class / God Object** | Class > ~500 LOC, > ~20 methods                                | `lizard --length 500`; `radon cc`; `tokei` per file; `pmd UnusedPrivateField` indirectly.                                                    |
| **Primitive Obsession**      | `string customer_id` everywhere instead of a `CustomerId` type | Hard to detect statically; manual review during dead-code audit.                                                                             |
| **Long Parameter List**      | Function takes > 5 params                                      | `eslint max-params`; `clippy::too_many_arguments`; `pylint too-many-arguments`.                                                              |
| **Data Clumps**              | Same group of params/fields appears in 3+ places               | Manual; sometimes flagged by IDE refactoring suggestions.                                                                                    |

### Object-Orientation Abusers

| Smell                                             | Signal                                                                                                                                                             |
| ------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Switch over type-tag**                          | `switch (x.kind)` repeated in many places — should be polymorphism. Often a precursor to dead branches when a kind is removed without removing all switches.       |
| **Refused Bequest**                               | Subclass overrides a base method to throw `NotSupportedException` / `unimplemented!()` — inheritance was wrong; the override is dead code from the contract's POV. |
| **Alternative Classes with Different Interfaces** | Two classes do the same thing with different APIs — the canonical "split-brain" smell. **Covered in `duplicate-code.md`.**                                         |
| **Temporary Field**                               | Object field used only by one method — should be a local variable; field outside that method's lifetime is dead.                                                   |

### Change Preventers — one change forces many edits

| Smell                                | Signal                                                | Detection                                                                                                                                         |
| ------------------------------------ | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Divergent Change**                 | Single class changes for multiple unrelated reasons   | Git history: `git log --name-only` shows file changes per commit; if file X co-changes with files in 3+ unrelated areas, it has divergent change. |
| **Shotgun Surgery**                  | Single conceptual change requires edits in many files | Inverse co-change: if file X always changes when files Y and Z change, they're coupled. Tools: `code-maat`, `codescene`.                          |
| **Parallel Inheritance Hierarchies** | Adding a subclass to A forces adding one to B         | Manual; flagged by reviewers. Reduce by collapsing hierarchies.                                                                                   |

### Dispensables ← _the dead-code-expert skill lives here_

Already covered:

- Comments (over-commenting / restating code) — `ai-slop-patterns.md` Pattern 5.
- Duplicate Code — `duplicate-code.md`.
- Data Class — class with only fields and getters/setters, no behavior — flag during audit.
- Dead Code — the rest of this skill.
- Lazy Class — class doing too little; merge into caller.
- Speculative Generality — `duplicate-code.md` and `ai-slop-patterns.md` Pattern 3.

### Couplers — excessive interaction between modules

| Smell                             | Signal                                              | Detection                                                                      |
| --------------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------ |
| **Feature Envy**                  | Method uses another class's data more than its own  | Manual review; some IDEs flag.                                                 |
| **Inappropriate Intimacy**        | Two classes reach into each other's internals       | Cyclic dependency analysis.                                                    |
| **Message Chains / Train Wrecks** | `a.getB().getC().getD()` — Law of Demeter violation | `eslint no-restricted-syntax` with custom rule; `pylint chained-method-calls`. |
| **Middle Man**                    | Class only delegates to another class               | Manual; sometimes flagged as dead wrapper layer.                               |

---

## Architectural Smells (beyond Fowler)

These are larger-scale patterns that source-symbol tools can't see.

### Cyclic Dependencies

**What:** Module A imports B which imports A (directly or transitively). Cycles defeat dependency-graph reasoning, prevent dead-code tools from finding orphans, and indicate poor layering.

**Detection per stack:**

```bash
# JavaScript / TypeScript
npx madge --circular --extensions ts,tsx src/

# Python
pydeps --show-cycles --max-bacon 0 src/
# or: pylint --disable=all --enable=cyclic-import

# Rust
cargo modules generate graph --package my_crate | grep -i cycle
# or use cargo-depgraph

# .NET
dotnet-graph or NDepend (CQLinq query for cycles)

# Go
go-callvis, then check graph for cycles

# C / C++
cinclude2dot . | dot -Tpng > deps.png   # visual; cycles are obvious
```

**Why for dead-code work:** A cycle means a module's "true" dependents are not just its direct importers — `knip`-style analysis may flag a file as orphan because the cycle hides its real caller, or vice versa miss truly-dead modules locked in a cycle nobody enters.

### Layer Violations

**What:** Domain layer importing infrastructure; UI importing repository directly; clean-architecture rings violated.

**Detection — architecture test libraries:**

| Stack      | Tool                                       |
| ---------- | ------------------------------------------ |
| JVM        | `archunit`                                 |
| TypeScript | `ts-arch`, `dependency-cruiser`            |
| Python     | `import-linter`, `pytestarch`              |
| .NET       | `NetArchTest`, `ArchUnitNET`               |
| PHP        | `deptrac`                                  |
| Rust       | `cargo-modules` (visualize), manual rules  |
| Go         | `go-cleanarch`                             |
| Multi      | `dependency-cruiser` (JS/TS), `Sonargraph` |

These tools encode rules like "domain must not depend on infrastructure" as tests that fail in CI. A violated rule often reveals dead infrastructure code: a domain class still calls into a repository because the migration to ports/adapters never finished — the repository call is in-flight dead code or the new port is an unused abstraction.

### Hexagonal / Ports & Adapters Mismatches

- **Port without adapter:** Interface defined in domain, no concrete implementation — speculative generality.
- **Adapter without port:** Infrastructure class no domain interface declares — bypassed abstraction; either the port is missing or the adapter is dead.

### Anemic Domain Model

Domain classes are bags of getters/setters; behavior lives in "service" classes. Often co-occurs with feature envy and middle-man smells. Service classes accumulate dead methods because nobody notices what's used.

### God Module / Mega-File

File > 1000 LOC, > 30 functions/classes. Dead code accumulates because the file is unreadable. Detection: `tokei`, `cloc`, `eslint max-lines`.

### Dead Layers

An entire architectural layer that exists but adds no value:

- DTO layer that's a 1:1 copy of the entity layer with field-by-field mapping.
- Repository pattern wrapping ORM that already provides repository semantics.
- Service classes that just call a single repository method (middle man at scale).

Detection: count layer-internal methods that are pure passthroughs (`return _repo.method(args)`).

---

## Complexity Metrics

Threshold-based metrics that flag spaghetti regions:

| Metric                               | Threshold (rule of thumb)   | Tools                                                                             |
| ------------------------------------ | --------------------------- | --------------------------------------------------------------------------------- |
| Cyclomatic complexity (per function) | > 10 risky, > 20 untestable | `lizard`, `radon`, `gocyclo`, `clippy::cognitive_complexity`, `eslint complexity` |
| Cognitive complexity                 | > 15                        | SonarQube, `clippy::cognitive_complexity`                                         |
| Nesting depth                        | > 4                         | `eslint max-depth`, `clippy::excessive_nesting`                                   |
| Maintainability index                | < 65                        | `radon mi`, Visual Studio metrics                                                 |
| Halstead metrics                     | High effort/volume          | `radon hal` (Python)                                                              |
| Lines per file                       | > 500                       | `tokei`, `cloc`                                                                   |
| Lines per function                   | > 50                        | `lizard`, `eslint max-lines-per-function`                                         |

These metrics don't directly find dead code, but a function with cyclomatic complexity 30 _probably_ has dead branches, dead variables, and impossible conditions. Flag high-complexity regions for manual dead-code review during audits.

---

## Co-Change Analysis (Behavioral Detection)

Source code reveals what's coupled by import graph. Git history reveals what's coupled in practice — files that **change together** are coupled even if they don't import each other.

**Tools:**

- **CodeScene** — commercial; produces co-change hotspots.
- **code-maat** — open-source; outputs CSVs for coupling analysis.
- **git-of-theseus** — visualizes file-age cohort survival.

**Quick command:**

```bash
# Files most frequently co-changed with file X
git log --format=format: --name-only --since=6.months -- path/to/file.ts \
  | sort | uniq -c | sort -rn | head -20
```

If file X always co-changes with files Y, Z, W from "unrelated" parts of the codebase, you have shotgun surgery — and likely duplicated logic / split-brain across those files.

---

## How this skill should respond to structural smells

The dead-code-expert skill is **not** a refactoring skill. When structural smells appear during a dead-code audit, the appropriate response is:

1. **Name the smell** in the report ("this file exhibits divergent change", "import cycle between A and B").
2. **Point at the right tool** (the table above).
3. **Surface the dead code that the smell hides** (god class → list its unused methods; cyclic dependency → list files at the cycle boundary that may actually be orphans).
4. **Defer the refactoring** to a language-specific critique skill (`/rust-critique`, `/dotnet-critique`, `/py-critique`, `/swift-critique`) or to the user's explicit follow-up.

This keeps the skill focused while still respecting that spaghetti and dead code are intertwined.
