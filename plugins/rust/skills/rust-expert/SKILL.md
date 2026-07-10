---
name: rust-expert
description: This skill should be used when the user is writing, reviewing, debugging, or architecting Rust code. Detects edition and toolchain from the project. Provides expert critique covering ownership, error handling, unsafe review, async correctness, trait design, type system patterns, performance, SOLID principles, and Cargo/workspace practices. Use when the user asks "critique my Rust code", "review this module", "fix borrow checker error", "is this unsafe sound", "design error types", "optimize this function", "review async code", "structure my workspace", "why is the borrow checker complaining", "help with lifetimes", "make this more idiomatic", or "review my Cargo.toml".
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

→ _Consult [design-principles reference](references/design-principles.md) for SOLID, Microsoft M-_ rules, modern Rust table.\*

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

**DO**: Use `assert_matches!` / `debug_assert_matches!` for pattern assertions in tests — they print the actual `Debug` repr of the failing value, unlike `assert!(matches!(..))`. Not in the prelude (collides with `mockall` / `claims`); import explicitly via `use std::assert_matches::assert_matches;` (it lives in a module of the same name — verify it is stable on the project's toolchain before recommending; on older stable use the `assert_matches` crate).

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

→ _See [2026-currency reference](references/2026-currency.md) for the full table with RUSTSEC details, migration paths, and per-release feature index._

Headline deprecations to flag in Cargo.toml scans: `async-std` ([RUSTSEC-2025-0052](https://rustsec.org/advisories/RUSTSEC-2025-0052.html)) and `bincode` ([RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141)) are unmaintained; `once_cell`/`lazy_static!`/`if_chain!`/`cfg-if`/`addr_of!` are superseded by std equivalents; `async-trait` for static dispatch is replaced by native AFIT (1.75); `#[bench]` is a hard error on stable since 1.88; `sled` is best avoided for new projects (use `redb` or `fjall`); `actions-rs/*` GitHub Actions are deprecated.

Existing code mostly still works — migrate when natural, not urgently. **`jiff` is the new datetime canon for IANA tz correctness but still pre-1.0**, so libraries with strict public-API stability should stay on `chrono` until jiff 1.0 ships.

## Still in Motion (June 2026)

These remain unstable or actively-debated. Use the workaround; don't pick winners.

- **AsyncDrop** — nightly, tracking #126482. Workaround: explicit `async fn close(self)` with `DropBomb` to catch missed calls.
- **`AsyncIterator` / `Stream` in std** — nightly. Use `futures::Stream` + `async-stream::stream!` macro.
- **`gen` blocks / coroutines** — nightly. `gen` is reserved in Edition 2024.
- **`Allocator` trait per-collection** — 6+ years nightly. `allocator_api2` polyfill.
- **Pin language support** — library-only. `pin-project-lite`.
- **Polonius borrow checker** — nightly, 2026 stabilization goal for alpha. Workaround: restructure or use `entry()`-style APIs.
- **Parallel rustc frontend** (`-Z threads=N`) — nightly, 15-50% wallclock improvement.
- **Cranelift codegen backend** (`rustc_codegen_cranelift`) — nightly-only as `rustc-codegen-cranelift-preview`. Active 2025H2 project goal to ship stable. ~20% dev-build speedup when used.

## Anti-Patterns

→ _Consult [anti-patterns reference](references/anti-patterns.md) for the full severity-labeled catalog._

## Hard-won opinions the catalog doesn't cover

- **A dropped `JoinHandle` is a detached task.** `tokio::spawn` keeps running after the handle is gone, and its panics vanish into the runtime. Hold the handle, use `JoinSet`, or write a comment saying why fire-and-forget is intended. Silent orphan tasks are the async equivalent of a leaked thread.
- **`let _ = fallible();` is error-swallowing with extra steps.** Either handle the `Result`, `.expect("invariant …")` it, or log it — an underscore is a decision to lose the failure, and it should look like one (`.ok(); // deliberately ignored: <why>`).
- **`Instant` for durations, `SystemTime` for wall-clock, never mixed.** `SystemTime` can go backwards (NTP); subtracting two of them returns a `Result` for a reason. Elapsed-time logic on `SystemTime` is a clock-skew bug waiting for deployment.
- **Floats have no total order.** `sort_by(|a, b| a.partial_cmp(b).unwrap())` panics on the first NaN that arrives from real data — use `total_cmp`. A float as a `HashMap` key is a design error, not a style issue.
- **On public APIs, prefer named generics over `impl Trait` in argument position.** `fn f(x: impl Iterator<Item = u8>)` cannot be turbofished by callers and can't be referenced in `where` clauses. `impl Trait` in *return* position is fine and idiomatic.
- **`use x::*` is for preludes and test modules only.** Anywhere else it hides where names come from and turns dependency bumps into mystery compile errors.
- **File-named modules over `mod.rs`.** Eight `mod.rs` files in an editor tab bar is self-inflicted pain; the 2018-edition style (`foo.rs` + `foo/` directory) exists — use it consistently.
- **`#[derive(Clone)]` is not free on aggregate types.** A reflexive `Clone` on a struct holding `Vec`s and `String`s hands every future caller a deep-copy escape hatch for borrow-checker friction — exactly the AI-slop clone pattern, but blessed at the type. Derive it when the type is *meant* to be duplicated, not by habit.
- **Default to `pub(crate)`; promote to `pub` deliberately.** Every `pub` item in a library is a semver contract. A module tree where everything is `pub` has no interior — nothing can be refactored without a major version.
- **Exhaustive error enums beat `#[from]` soup.** `thiserror` with a `#[from]` for every dependency error type turns your error into a mirror of your dependency graph; callers can't tell which variants are actionable. Group by what the *caller* can do (retryable / bad-input / bug), not by what library failed.

## Zoom out before you edit

Sessions that skip this produce split-brain code (a second implementation of an existing helper) and orphans. Non-negotiable sequence for any change:

1. **Before adding a function, type, or trait: search for an existing one** — `rg -i` the concept, not just the name you were about to pick, and check the crate's existing `util`/`common` modules. A second implementation is a drift bug on a timer.
2. **Read the whole module and its callers before editing**, not just the flagged lines — in Rust especially, ownership decisions upstream are usually the cause of the symptom downstream.
3. **After the change, grep the old symbol names**; delete anything now unreferenced in the same change (the compiler's `dead_code` lint will only catch the private cases).
4. **Say in one sentence where the change sits in the crate's architecture.** If you can't name the layer or boundary, you don't understand the change yet.
5. **Verification is output, not assertion.** `cargo build`/`cargo test`/`cargo clippy` results pasted into the report are verification; the word "verified" without them is not. If you didn't run it, write "unverified".

## The Rust AI Slop Test

→ _Consult [ai-slop reference](references/ai-slop.md) for the complete fingerprint catalog._

**Critical quality check**: If a senior Rust engineer reviewed this code, would they immediately suspect AI generated it? If yes, that's the problem.

The most common AI tells in Rust:

- `.clone()` everywhere to silence the borrow checker
- `Arc<Mutex<T>>` as default concurrency for everything
- `.unwrap()` on every `Result` and `Option`
- Traits with exactly one implementation
- Over-annotated lifetimes where elision works
- `async` on functions that never `.await`
- Verbose comments explaining WHAT, never WHY
- Generic variable names (`data`, `result`, `item` instead of domain names)
- Premature generalization with unused generic parameters
- No refactoring — duplicated blocks with minor variations
- `#[allow(...)]` to suppress warnings instead of fixing root cause

## Thinking Prompts

Before suggesting any fix, work through:

1. **What bug does this prevent?** If you cannot name a concrete bug, the fix may not be worth the complexity.
2. **What would happen in production?** Think in terms of incidents, not style.
3. **Is the type system doing enough work?** Every runtime `assert!` is a type waiting to be born.

## How to reason: the scientific method, applied to code review

Approach every finding the same way you'd approach a scientific hypothesis. The steps are straightforward and the plugin should run through them in order.

Start by discovering what the code actually does. Read it before suggesting anything — scan for patterns, dependencies, the shape of state, the I/O boundaries, the test setup. A finding that ignores what the code is doing is just noise.

Then evaluate what you found against the evidence. Compare to known patterns and named consequences. When you see `.unwrap()` on external input, that's the Cloudflare class of bug — name the incident, explain the failure mode, propose the fix. When the dependency is `async-std`, cite RUSTSEC-2025-0052. When the codebase is a mix of architectures, the harm is concrete (the team has to keep multiple mental models in their heads at once) — say so. Findings backed by evidence get stated directly and confidently.

Make sure you actually understand the code before recommending changes. An `Arc<Mutex<>>` that looks like a god-object might turn out to be a correctly-scoped cache with short critical sections. A trait with one implementor might be the deliberate seam for a planned second implementation. If you can't tell which case you're looking at, that's the time to ask the developer — not the time to invent a confident-sounding wrong answer.

Every fix proposal should come with a way to verify it worked. If you suggest replacing `.unwrap()` with `?` and `.context(...)`, also tell the developer which test should now pass that didn't before — or that this is the time to write that test. If you suggest adding `overflow-checks = true`, tell them which arithmetic-heavy code path will now panic where it silently wrapped. A fix without a verification plan is a guess that hasn't been tested yet.

Through all of this, use your judgment. The scientific method is a discipline, not a substitute for thinking. When the evidence supports a strong call, make it. When the codebase looks coherent and well-considered, say so and don't manufacture findings to look thorough. When the right answer genuinely depends on something the code can't tell you, ask the specific question that would clarify it — but ask it once, not as a tic.

The thing to avoid is hedging that masquerades as humility. "Consider whether X applies; this depends on your specific context" is not honest uncertainty — it's a sentence with no information in it. The honest version of uncertainty is specific: "I can't tell from the code whether the `OrderRepository` trait is positioned for a planned second implementation — if it is, it's justified; if not, it's overhead. Which is it?" That kind of question helps the developer. Vague hedging just wastes their time.

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
