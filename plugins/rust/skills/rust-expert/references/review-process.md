# Evidence-Driven Rust Review Process

## High-signal judgments

- **A dropped `JoinHandle` is a detached task.** `tokio::spawn` keeps running
  after the handle is gone, and its panics vanish into the runtime. Hold the
  handle, use `JoinSet`, or explain why fire-and-forget is intentional.
- **`let _ = fallible();` discards the failure.** Handle the `Result`, assert a
  real invariant, or log it. If deliberate, make that decision visible with
  `.ok(); // deliberately ignored: <why>`.
- **Use `Instant` for durations and `SystemTime` for wall-clock time.**
  `SystemTime` can move backwards under clock correction; elapsed-time logic
  built on it is a deployment bug.
- **Floats have no total order.** `partial_cmp(...).unwrap()` panics on NaN; use
  `total_cmp`. Treat float `HashMap` keys as a domain-design problem.
- **Prefer named generics on public input APIs.** Callers can reference and
  turbofish `T`; argument-position `impl Trait` hides that control. Return
  position `impl Trait` remains idiomatic.
- **Reserve glob imports for preludes and test modules.** Elsewhere they hide
  name provenance and turn dependency upgrades into ambiguous failures.
- **Use file-named modules rather than `mod.rs`.** The 2018 module layout keeps
  editor tabs and paths distinguishable.
- **Derive `Clone` only when duplication is semantic.** Aggregate `Clone`
  provides a deep-copy escape hatch for ownership mistakes.
- **Default to `pub(crate)` and promote deliberately.** Every public library
  item is a semver contract.
- **Design error enums around caller actions.** A `#[from]` variant for every
  dependency mirrors the dependency graph instead of expressing retryable,
  invalid-input, or bug states.

## Zoom out before editing

Avoid split-brain implementations and orphaned code:

1. Search for an existing function, type, trait, or helper by concept before
   adding another one.
2. Read the complete module and its callers. Ownership symptoms often originate
   one layer above the compiler error.
3. After renaming or replacing code, search for old symbols and remove newly
   dead paths in the same change.
4. Identify the architectural layer or boundary that owns the change. If it
   cannot be named, investigate further before editing.
5. Verify with the repository's real build, tests, and Clippy configuration.
   If a command was not run, call the result unverified.

## Apply the scientific method

### Discover

Read before recommending. Identify state ownership, I/O boundaries,
dependencies, error flow, concurrency, tests, and existing architecture.
Findings that ignore actual code behavior are noise.

### Evaluate against evidence

Connect each observation to a named consequence. An unwrap on external input
can become an outage; an unmaintained runtime carries an advisory; mixed
architectures impose multiple mental models on maintainers. State the evidence
and failure mode rather than asserting taste.

### Distinguish evidence from uncertainty

An `Arc<Mutex<_>>` may be a god object or a correctly scoped cache. A
one-implementation trait may be needless indirection or a planned external
seam. When code cannot distinguish those cases, ask the one concrete question
that would.

### Propose a verifiable fix

Name the test, command, or runtime behavior that proves the change. Replacing an
unwrap with typed propagation needs an error-path test; enabling overflow
checks needs an arithmetic boundary case. A proposal without a verification
path remains a hypothesis.

### Use judgment

Do not manufacture findings to look thorough. Make strong calls when evidence
supports them, praise coherent design, and express uncertainty specifically.
Avoid empty hedges such as "consider whether this applies."

## Three questions before any finding

1. What concrete bug or maintenance failure does this prevent?
2. What happens in production if it remains?
3. Can the type system eliminate the runtime state or check?
