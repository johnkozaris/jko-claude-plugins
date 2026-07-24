---
name: rust-expert
description: This skill should be used when the user is writing, reviewing, debugging, architecting, or upgrading Rust code. Detects edition, toolchain, and MSRV from the project. Provides expert critique covering ownership, errors, unsafe, async, traits, types, performance, architecture, Cargo, and release compatibility. Use when the user asks "critique my Rust code", "fix borrow checker error", "is this unsafe sound", "make this idiomatic", "review my Cargo.toml", "upgrade to Rust 1.97", "what changed in Rust 1.97", "is this API stable on my MSRV", or "check Rust release compatibility".
---

This skill guides expert Rust development. Detect the project's edition and toolchain from `Cargo.toml` (`edition`, `rust-version`) and adapt guidance accordingly. Every finding explains WHY it matters — what bug it prevents, what production incident it avoids, what design problem it reveals. Do not invent APIs — verify any method or type exists in stable Rust before suggesting it.

## How to Think About Rust Problems

Before fixing any issue, trace through the layers:

- **Layer 3 — Domain (WHY)**: Business rules, performance constraints, deployment context. These constrain everything below.
- **Layer 2 — Design (WHAT)**: Error strategy, type design, API surface, module structure. Check against SOLID and API Guidelines.
- **Layer 1 — Mechanics (HOW)**: Compiler errors, ownership, lifetimes, trait bounds. Fix the immediate issue, but always trace UP.

When a compiler error appears, reframe it as a design question:

| Error                            | Don't Just Say   | Ask Instead                            |
| -------------------------------- | ---------------- | -------------------------------------- |
| E0382 (value moved)              | "Clone it"       | Who should own this data?              |
| E0597 (doesn't live long enough) | "Add a lifetime" | Is the scope boundary correct?         |
| E0277 (trait not satisfied)      | "Add the bound"  | Is this the right abstraction?         |
| E0499 (two mutable borrows)      | "Use RefCell"    | Should this be two separate resources? |
| "future is not Send"             | "Wrap in Arc"    | Does this state need to cross threads? |

## Ownership & Borrowing

→ _Consult [ownership reference](references/ownership.md) for borrowing rules, Cow, smart pointers._

**DO**: Default to borrowing (`&T`). Move to owned only when the callee must store the value.
**DO**: Use `&str` and `&[T]` in function parameters — not `&String` or `&Vec<T>`.
**DO**: Use `Cow<'_, str>` when a function conditionally allocates.
**DO**: Treat `Arc::clone(&handle)` of service handles, DB pools, and channels as idiomatic — that is what M-SERVICES-CLONE is for.
**DON'T**: Clone owned heap data (`Vec`, `String`, large structs) to silence the borrow checker — restructure ownership instead.
**DON'T**: Over-annotate lifetimes — elision covers 95% of cases.
**DON'T**: Write `&'a mut self` on methods — borrows self for its entire lifetime.

## Error Handling

→ _Consult [error-handling reference](references/error-handling.md) for thiserror/anyhow/snafu decision matrix._

**DO**: Use `?` to propagate, and `.context()` **where the frame adds information** (the file path, the query, the record id) — not mechanically at every layer. Context at every propagation point produces russian-doll messages ("failed to handle request: failed to process order: failed to query db: …") that bury the one frame that knew something.
**DO**: Use `thiserror` v2 for typed library or domain errors and `anyhow` v1 at application boundaries where callers do not branch on error variants. Applications that need programmatic recovery should keep typed errors too.
**DO**: Use `#[non_exhaustive]` on public error enums.
**DO**: Use `.expect("invariant X holds because Y")` to assert what the type system cannot express — invariant assertion is fine; lazy error handling is not.
**DON'T**: Use `.unwrap()` or `.expect()` on Results from outside the program (parse, IO, env, deserialize, network) — this is what took down [Cloudflare on Nov 18, 2025](https://blog.cloudflare.com/18-november-2025-outage/): a hard-coded 200-feature limit hit unexpected input, `.unwrap()` on the `Err` panicked in `fl2_worker_thread`, 5xx globally for hours.
**DON'T**: Implement `From` for fallible conversions — use `TryFrom`.
**DON'T**: Both log AND propagate an error — pick one.

## Type Design

→ _Consult [type-patterns reference](references/type-patterns.md) for newtype, typestate, builder patterns._

**DO**: Parse, don't validate — convert raw inputs into types that carry their validity.
**DO**: Replace boolean parameters with enums — `process(data, true, false)` is unreadable.
**DO**: Use `core::range::Range` (and `core::range::RangeFrom` / `RangeToInclusive`) when you need a `Copy`-able range — e.g., storing slice indices in a `Copy` newtype like `Span(core::range::Range<usize>)`. Legacy `core::ops::Range` is not `Copy` because it implements `Iterator` directly. The `0..n` syntax still produces legacy types; convert with `.into()` until a future edition flips the default.
**DO**: Use `#[must_use]` on functions returning values callers must handle.
**DON'T**: Use `..Default::default()` — silently wrong when fields are added.
**DON'T**: Use catch-all `_` in match on owned enums — swallows new variants.

## Design Principles

→ _Consult [design-principles reference](references/design-principles.md) for SOLID, Microsoft `M-*` rules, and the modern Rust table._

**DO**: Apply Single Responsibility — one struct per concept, one domain per module.
**DO**: Introduce a trait at a real substitution boundary — multiple implementations, caller-supplied behavior, runtime polymorphism, or an external I/O seam that benefits from a test double. Prefer a concrete type when there is one owned implementation.
**DO**: Use `#[expect(lint)]` instead of `#[allow(lint)]` (warns when stale, Rust 1.81+).
**DON'T**: Use weasel word names — `BookingService`, `DataManager`. Name types after what they ARE.
**DON'T**: Expose `Arc`, `Rc`, `Box` in public API signatures — hide implementation details.

## Traits & API Surface

→ _Consult [traits reference](references/traits.md) for generics vs dyn, standard traits, sealed patterns._

**DO**: Implement standard traits deliberately. `Debug` is usually valuable; add `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, and `Default` only when their semantics are valid and callers genuinely benefit.
**DO**: Default to generics. Use `dyn Trait` only for genuine runtime polymorphism.
**DON'T**: Violate Hash/Eq consistency — the most dangerous silent bug in Rust's stdlib.

## Async

→ _Consult [async reference](references/async.md) for blocking taxonomy, cancellation safety, JoinSet._

**DO**: Keep business logic synchronous. Use async only at I/O boundaries.
**DO**: Use `CancellationToken` when work needs cooperative shutdown and cleanup. Use `JoinHandle::abort()` only when the future is cancellation-safe and immediate forced cancellation is intentional; await the handle so cancellation and panics are observed.
**DON'T**: Monopolize an async runtime worker with blocking I/O, long CPU work, or unbounded loops. Move blocking/CPU work to the appropriate pool and measure against the application's latency budget instead of applying a universal microsecond cutoff.
**DON'T**: Hold a `MutexGuard` across `.await` — drop it first.
**DON'T**: Make every function async "just in case" — async infects signatures upward.

## Concurrency

→ _Consult [concurrency reference](references/concurrency.md) for decision tree, actor pattern, channels, atomic orderings._

**DO**: Ask first: "Do I actually need concurrency?" If no measured bottleneck, stay sequential.
**DO**: Pick the primitive by access pattern — see Concurrency Pattern Triage below.
**DO**: Use bounded channels in production for backpressure.
**DON'T**: Default to `Arc<Mutex<T>>` for everything — it's the right tool for short critical sections, not for global app state.
**DON'T**: Use async for CPU-bound work — use Rayon or `spawn_blocking`.

### When to reach for which concurrency primitive

Pick by access pattern, not reflex. Channels (`tokio::sync::mpsc` for async, `crossbeam-channel` for sync) for ownership transfer between tasks. The actor pattern (one task owns state, handle wraps an `mpsc::Sender<Msg>`) for long-lived stateful concurrent components — this replaces most uses of `Arc<Mutex<BigStruct>>`. `Arc<Mutex<T>>` for short critical sections on shared data; std::sync::Mutex became futex-based on Linux in 1.62 and the gap to parking_lot closed dramatically (parking_lot still has an edge under heavy contention), so reach for parking_lot only for its specific features (fairness, reentrant, RwLock downgrade, deadlock detection). `Arc<RwLock<T>>` for read-heavy with rare writes (watch for writer starvation). `ArcSwap<T>` for snapshot reload (lock-free reads, atomic swap). `DashMap` for partition-keyed concurrent access — never call another DashMap method while holding a `Ref`/`RefMut` to the same map, that's a shard self-deadlock. Atomics for single primitives, with the right ordering (Relaxed for independent counters, Release/Acquire for publishing data).

The anti-pattern is `Arc<Mutex<WholeAppState>>` as a god-object. Split by concern, let each piece pick its own primitive.

## Unsafe

→ _Consult [unsafe reference](references/unsafe.md) for SAFETY comments, Miri, UB patterns._

**DO**: Every `unsafe` block needs a `// SAFETY:` comment explaining the invariant.
**DO**: Run `cargo +nightly miri test` on code with unsafe.
**DO**: Deny unsafe at crate level (`unsafe_code = "deny"`) with surgical allows.
**DON'T**: Use unsafe when safe alternatives exist — all memory-safety CVEs in Rust trace to unsafe code.

## Performance

→ _Consult [performance reference](references/performance.md) for build config, allocation patterns, benchmarking._

**DO**: Profile before optimizing — `cargo flamegraph`, DHAT, samply.
**DO**: Use `overflow-checks = true` in release profiles (CVE-2018-1000810).
**DO**: Use `strict_add` / `strict_sub` instead of `checked_add().unwrap()` — they always panic on overflow, even in release.
**DO**: Use `core::hint::cold_path()` inside rare branches when `#[cold]` on the whole function is too coarse.
**DON'T**: Optimize without a measured bottleneck.

## Security & Robustness

→ _Consult [security reference](references/security.md) for OWASP-class issues that safe Rust does not catch._

**DO**: Enable `overflow-checks = true` in release; use `strict_*` for must-not-overflow business arithmetic.
**DO**: Open-then-check filesystem paths (avoid TOCTOU); use `O_NOFOLLOW | O_DIRECTORY`.
**DO**: Use constant-time comparison (`subtle::ConstantTimeEq`) for passwords, MACs, tokens.
**DO**: Cap input sizes at every boundary (HTTP body, decompression, deserialization).
**DO**: Wrap secrets in a redacting newtype (or `secrecy` crate) — never derive `Debug` on a struct containing a password.
**DO**: Validate at the deserialization boundary via `#[serde(try_from = "...")]`, and use `#[serde(deny_unknown_fields)]`.
**DO**: Run `cargo audit` (and `cargo deny` / `cargo geiger` where useful) in CI.
**DON'T**: Use `==` on secret bytes, `Path::join` user input without checking absolute paths, or `as` to narrow integers.

## Testing & Documentation

→ _Consult [testing](references/testing.md) and [documentation](references/documentation.md) references for test frameworks, property testing, and API docs guidelines._

**DO**: Prefer `assert_matches!` / `debug_assert_matches!` over `assert!(matches!(...))` when the project's MSRV supports them; the testing reference covers imports and compatibility.

## Modules, Macros & Serde

→ _Consult [modules-cargo](references/modules-cargo.md), [macros](references/macros.md), and [serde](references/serde.md) references for workspace setup, macro decision flowchart, and serialization patterns._

## FFI & Cross-Language Boundaries

→ _Consult [ffi reference](references/ffi.md) when the code touches `extern "C"`, cdylib/staticlib targets, wasm, PyO3, napi-rs, UniFFI, or any cross-language boundary — panic/unwind rules, Edition 2024 `unsafe extern`, per-ecosystem crate guidance._

## Cross-Cutting Decision Rules

→ _Consult [decision-rules reference](references/decision-rules.md) for the ten numbered judgment calls other references delegate to: unwrap/expect policy, shared-state primitive, newtype-or-not, builder crate, date/time crate, workspace split, Edition 2024 migration, parking_lot vs std Mutex, bounded vs unbounded channels, test ratios._

## Architecture Patterns

→ _See [architecture reference](references/architecture.md) for pattern descriptions, detection signatures, smells per pattern, and bad-pattern flags. See [workspace-organization](references/workspace-organization.md) for sub-crate decomposition. For architecture conversations, invoke `/rust-architect`._

The posture: **detect what the codebase is using, stay consistent with it, flag bad patterns**. Don't pick between healthy architectures — module-driven, hexagonal, actor, functional-core/imperative-shell, sans-IO, pipeline, typestate, plugin registry all work for different shapes. The harm of mixed patterns in one codebase is worse than picking "wrong" between healthy options. For game engines (Bevy ECS), GUI apps (Iced MVU, egui immediate-mode), and embedded firmware (Embassy's async reactor), the framework decides — don't argue.

**Bad patterns to flag regardless of architecture**: mixed architectures in one repo, OOP inheritance via `Deref` chains, god-object `Arc<Mutex<AppState>>`, stringly-typed domains, premature workspace splits with no consumer, one-implementor trait obsession, `Box<dyn Error>` in library APIs, mock-only tests. Full descriptions in the architecture reference.

**Workspace split** is a separate decision with objective signals: compile-time pain, two binaries sharing code, a published SDK crate that third parties consume, an FFI shim, or team ownership. [Tokio #1318](https://github.com/tokio-rs/tokio/issues/1318) is the "don't split for tidiness" warning. Flat layout when warranted; centralize with `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`.

## 2026 Deprecation Watchlist

→ _See [2026-currency reference](references/2026-currency.md) for the current toolchain, RUSTSEC details, migration paths, and per-release feature index. Consult [Rust 1.96.1-1.97.1 release notes](references/rust-1.97-release-notes.md) when upgrading from the previous 1.96 anchor or explaining every 1.97 compatibility change._

Headline Cargo.toml flags: unmaintained `async-std` and `bincode`, crates superseded by std, deprecated `actions-rs/*`, and `sled` in new projects. Use the currency reference for evidence and migration targets; migrate working code deliberately rather than mechanically.

## Still in Motion (July 2026)

Do not recommend unstable features as production defaults. Consult the currency reference's "Still nightly" table for current status and stable workarounds.

## Anti-Patterns

→ _Consult [anti-patterns reference](references/anti-patterns.md) for the full severity-labeled catalog._

## Review Process

→ _Consult [review-process reference](references/review-process.md) before reviewing or editing: search-before-change workflow, evidence standard, verification loop, and high-signal Rust judgments._

## The Rust AI Slop Test

→ _Consult [ai-slop reference](references/ai-slop.md) for the fingerprint catalog. Flag clone-heavy ownership avoidance, default `Arc<Mutex<_>>`, one-implementation traits, indiscriminate unwraps, and comments that narrate syntax rather than intent._

## Severity Levels

Label every finding:

- **blocking** — Soundness bug, UB, data race, guaranteed panic on plausible input, security flaw, RUSTSEC-flagged dependency. Must fix before merge.
- **important** — Wrong error handling on external input, performance cliff in a measured hot path, design pain causing future churn, missing tests for non-trivial logic, deprecated dependencies needing a migration plan.
- **architecture** — Misfit pattern, premature abstraction, missing seam, workspace-split signal. Used in `/rust-critique`; route to `/rust-architect` for design-level work.
- **nit** — Style, naming, minor idiom. Fix if convenient.
- **polish** — Pre-merge cleanup: clippy warnings, formatting drift, dead code, debug artifacts, doc coverage on public items.
- **suggestion** — Alternative worth considering. No action required.
- **praise** — Highlight well-written code. Reinforce good patterns.

## Output Format

Group findings by file. For each finding:

1. File path and line number
2. Severity label
3. Rule name
4. **WHY it matters** — the concrete consequence
5. Before/after code block when the fix is non-obvious

Skip files with no findings. End with a prioritized summary.

**CRITICAL**: Be direct — vague feedback wastes time. Be specific — "line 42 of parser.rs" not "some functions". Say what's wrong AND why it matters. Prioritize ruthlessly — if everything is important, nothing is.
