# Decision Rules

Some choices in Rust come up over and over: when to use `unwrap`, what shared-state primitive to reach for, when to newtype, which builder crate, jiff or chrono, single crate or workspace. This file collects those decisions in one place so the plugin doesn't have to relitigate them every time.

The shape of each rule is the same. There's a default — what to do if no special condition applies — and then a list of situations where the default doesn't fit and what to do instead. The point of writing it this way is to avoid two failure modes that often show up in architectural advice. One is hedging on everything ("consider whether X applies, this depends on your context"), which gives the developer no guidance and produces codebases where every contributor picks differently. The other is bulldozing with a single rule that has no exceptions, which forces legitimate working code into the same shape as broken code.

The rules below tell you what to do, and they're honest about when the default doesn't fit. If you find a situation that doesn't match any of the rows, the default in the first row is usually right; if you're not sure why your situation doesn't match, that's a sign to think about it more rather than guess.

---

## Rule 1 — `.unwrap()` and `.expect()`

| Situation | Answer |
|---|---|
| **DEFAULT** | Use `?` to propagate. `.expect("invariant X holds because Y")` ONLY when asserting a programmer-guaranteed condition the type system can't express. |
| Result/Option from outside the program (parse, IO, env, deserialize, network, user input) | NEVER `.unwrap()` or `.expect()` — propagate with `?` and `.context()`. This is what took down Cloudflare on Nov 18 2025 ([postmortem](https://blog.cloudflare.com/18-november-2025-outage/)). |
| Result/Option that the program JUST checked (e.g., `if v.is_empty() { return; }` then `v[0]`) | `.expect("non-empty by check above")` is fine — but slice patterns or `let Some(x) = v.first() else { return; }` is cleaner. |
| Mutex::lock() | `.expect("mutex poisoned — programmer error elsewhere")` is fine; or use `parking_lot` which doesn't poison; or restructure if poisoning is a real concern. |
| Tests, benchmarks, doc examples | `.unwrap()` is fine — tests should panic on unexpected state. |
| Main function's startup initialization that MUST succeed | `.expect("could not load config")` is acceptable — the program can't proceed without it. |

**Source**: [BurntSushi — Using unwrap() in Rust is Okay](https://burntsushi.net/unwrap/). The rule isn't "no unwrap"; it's "unwrap as documented assertion of invariant, never as error handling on external input."

---

## Rule 2 — Shared state primitive

Pick by access pattern, never by reflex.

| Access pattern | Use |
|---|---|
| Ownership transfer between tasks | `tokio::sync::mpsc` (bounded), or `crossbeam-channel` for sync threads |
| Long-lived stateful concurrent component | **Actor pattern**: struct + run loop + bounded `mpsc` + `Handle { tx }`. See [architecture.md](architecture.md#3--actor-pattern-ryhl). |
| Short critical section on shared data | `Arc<Mutex<T>>` (`std::sync` is futex-based and close to parking_lot since 1.62 — see Rule 10) |
| Read-heavy with rare writes | `Arc<RwLock<T>>` — watch for writer starvation under sustained read pressure |
| Snapshot reload (config, routing table) | `ArcSwap<T>` — lock-free reads, atomic swap on write |
| Partition-keyed concurrent access | `DashMap<K, V>` — sharded locks. **Never call a method while holding a `Ref`/`RefMut` to the same map** — shard deadlock. |
| Single primitive (counter, flag, pointer) | `Atomic*` (`Relaxed` for counters, `Release`/`Acquire` for handoff) |
| Single primitive needing transform | `Atomic*::update` (stable 1.95) — replaces hand-rolled `compare_exchange` CAS loops |

**Anti-pattern (regardless of which primitive)**: `Arc<Mutex<WholeAppState>>` as a god-object. Split by concern; each piece picks its own primitive from the table above.

---

## Rule 3 — Newtype or not?

| Situation | Answer |
|---|---|
| **DEFAULT** | Don't newtype — use the underlying primitive. |
| Two IDs of different concepts at the same call site (UserId vs OrderId vs ProductId) | Newtype each one. Compiler refuses to swap them. |
| A primitive needs validation (Email, URL, NonEmptyString, BoundedInt) | Newtype with a private inner field and validated constructor. |
| You need to add traits to a foreign type (orphan rule) | Newtype the foreign type. |
| You're shipping a library — public API uses primitive types | Newtype anything user code might confuse — IDs especially. Makes refactors safe. |
| Internal CRUD app with one ID concept, no validation rules | Skip the newtype — friction without benefit. |

A newtype with no methods and no validation invariants is pure noise. The question to ask: "What invariant does this newtype protect, or what confusion does it prevent?"

---

## Rule 4 — Builder crate

| Situation | Answer |
|---|---|
| **DEFAULT (new project)** | `bon` — compile-time typestate, works on functions not just structs. |
| Existing `derive_builder` code that works | Keep it. Don't migrate without measured reason. |
| Need runtime validation hooks (fields validated at `.build()` time, not before) | `derive_builder` — its hooks are richer than bon's. |
| Want minimal macro footprint, no proc-macro at all | `typed-builder` — declarative-macro-only. |

---

## Rule 5 — Date/time crate

| Situation | Answer |
|---|---|
| New application code touching timezones / DST, can tolerate minor API churn | `jiff` (BurntSushi). Best DST/timezone handling. Pre-1.0 in May 2026; maintainer committed to indefinite API stability post-1.0. |
| Library with strict public API stability | `chrono` (still releasing — 0.4.44 in Feb 2026 — but maintainer Dirkjan Ochtman signaled wind-down intent in his Jan 2026 year-in-review and now recommends jiff). Re-evaluate when `jiff` 1.0 ships. |
| `no_std` / embedded | `time` crate. |
| Existing chrono code | Don't migrate without reason. chrono is still being released and compiles fine; the wind-down is signaled, not done. |

The jiff caveat is real: 1.0 was originally targeted for Summer 2025, then Spring/Summer 2026; the README has since removed the timeline, so there's no committed date. Don't expose `jiff::Zoned` from a library API yet unless you're prepared for the migration when 1.0 lands.

---

## Rule 6 — Workspace split: single crate or multi-crate?

| Situation | Answer |
|---|---|
| **DEFAULT** | Single crate. Add structure within `src/` (folders, modules). |
| Save a file triggers a 30s+ rebuild | Split. Compile-time benefit will pay back. |
| Two binaries share non-trivial code (e.g. `server` and `cli`) | Split: shared `core` library + thin binaries. |
| You're publishing a library AND keeping a private CLI/daemon around it | Split: published library + private application. |
| You have a plugin SDK (like Zellij's `zellij-tile`) | Split: the SDK is its own published crate; the host is not. |
| FFI: pure Rust `-core` + C-ABI `-ffi` shim | Split: clean separation between logic and FFI surface. |
| Multiple teams own different subsystems with frequent merge conflicts | Split: crate boundaries enforce ownership. |
| You think "splitting will be cleaner" without one of the above triggers | DON'T split. [Tokio merged sub-crates back in 2019 (#1318)](https://github.com/tokio-rs/tokio/issues/1318) for exactly this reason — splitting for tidiness added maintenance and user-confusion overhead. |

See [workspace-organization.md](workspace-organization.md) for the Zellij case study, matklad's flat layout, and the feature-unification trap.

---

## Rule 7 — Edition 2024 migration

| Situation | Answer |
|---|---|
| New binary / application / service | Edition 2024 (Rust 1.85+). Run `cargo fix --edition`. |
| New library with no MSRV obligations | Edition 2024. |
| Library with broad MSRV policy (e.g., "1.70+") and downstream users on older toolchains | Stay on Edition 2021 until you can audit downstream impact. Edition is per-crate, so this isn't blocking your binaries. |
| Embedded / kernel work pinning a specific toolchain | Match the toolchain. |

Edition matters because Edition 2024 requires Rust 1.85+. If your `rust-version` is older, bumping the edition raises your MSRV.

---

## Rule 8 — `parking_lot::Mutex` or `std::sync::Mutex`?

| Situation | Answer |
|---|---|
| **DEFAULT (new code)** | `std::sync::Mutex`. Since 1.62, futex-based on Linux — the gap to `parking_lot` closed dramatically (the old pthread implementation was significantly slower). parking_lot still has a measurable edge under heavy contention, but for typical uncontended use the std mutex is the simpler default. |
| Heavy bursty contention with starvation risk | `parking_lot::Mutex` — eventual fairness mechanism prevents complete starvation. |
| Need `RwLock` atomic downgrade (write → read without releasing) | `parking_lot::RwLock` — std doesn't support this. |
| Need upgradable read locks | `parking_lot`. |
| Need reentrant locks | `parking_lot::ReentrantMutex`. |
| Need deadlock detection in dev/staging | `parking_lot` with the `deadlock_detection` feature. |
| Memory-tight fine-grained locking on macOS (need 1-byte mutex size) | `parking_lot`. |
| Holding a guard across `.await` | NEITHER — restructure or use `tokio::sync::Mutex` (and audit every operation inside the critical section). |
| Existing `parking_lot` code that works | Keep it. Don't migrate without measured reason. |

**Don't migrate working `parking_lot` code based on "std caught up."** The features above are real reasons people picked `parking_lot`; the std vs parking_lot performance gap is small for typical uncontended cases.

---

## Rule 9 — Bounded channel or unbounded?

| Situation | Answer |
|---|---|
| **DEFAULT** | Bounded. Capacity is your backpressure. |
| Standard producer/consumer code | `mpsc::channel(N)` where N is intentional (8-64 for command channels; 1024+ for high-throughput data pipelines). |
| Signal handler, interrupt handler, sync code calling into async (producer cannot block) | Unbounded is acceptable — but pair with explicit drop-on-overflow logic or a periodic drain step. |
| Ring-buffer pattern (drop oldest) | Use `tokio::sync::broadcast` (lossy on slow consumers) or a custom bounded structure with drop-oldest semantics. |
| You think "I don't know the right capacity, so I'll use unbounded" | NO — pick a capacity. Unbounded is how production services OOM under load. |

Document the unbounded choice with a comment naming the reason. Unexplained `mpsc::unbounded_channel()` in production code paths is `important` severity in code review.

---

## Rule 10 — Test ratios

Drop the "60/30/10 unit/integration/E2E" hedge. matklad's framing replaces it:

| Concept | Rule |
|---|---|
| **Purity** (the test's environment cost) | Optimize ruthlessly: pure compute > threads > disk > multi-process > distributed. Push as much logic as possible into pure functions and test those. |
| **Extent** (how much code the test exercises) | Don't artificially constrain. Big-extent tests via integration are fine if fast and reliable. |
| **Metric that matters** | Wall-clock time to feedback. Cargo's test suite is seven minutes; rust-analyzer's is thirty seconds. The difference is purity discipline, not test count. |
| Every public item | Gets a doc test (documentation that compiles). |
| Behavioral invariants | Property tests (`arbtest`, `proptest`). |
| Structured output (errors, JSON, AST) | Snapshot tests (`insta`). Review like code, never `--accept` in bulk. |
| Time-dependent async code | `#[tokio::test(start_paused = true)]` + `tokio::time::advance`. |
| HTTP integration | `wiremock` (a real local server tests serialization, headers, errors). |
| Stateful service E2E | `testcontainers` for real backends. |
| Non-IO collaborator | Real fakes (in-memory struct implementing the trait), not `mockall`. |
| True IO boundary (HTTP client, time, randomness) | `mockall` is acceptable here. |
| Coverage tool | `cargo-llvm-cov`, treated as signal not target. |

See [testing reference](testing.md) for the full toolkit.

---

## How to know if you're applying these correctly

When you're about to apply one of these rules, it's worth checking yourself. The rule should be tight — you should be able to say "do X" or "don't Y" without qualifying it with "perhaps" or "might." The exception clauses should be specific enough that you couldn't always find a reason to skip the rule. And the reason behind the rule should be a concrete consequence: a named incident, a class of bug, a measurable performance cost. If you can't name the consequence, the rule isn't really doing any work — fall back to the default.

The thing to avoid is hedging. Compare these two ways of giving the same advice:

The hedging version says: "Consider whether `Arc<Mutex<T>>` is appropriate here. Both channels and mutexes are valid choices, and the right one depends on your specific context and requirements." That sentence has no information in it. The developer learns nothing and has to make the same decision from scratch.

The honest version says: "Use `Arc<Mutex<T>>` for short critical sections on shared data. Use the actor pattern for long-lived stateful concurrent components. Use channels when you're transferring ownership between tasks. Don't put your whole application state in one `Arc<Mutex<>>` — that's a god-object, not a tool. Split by concern instead." This tells the developer what to do at each branch and what the failure mode looks like.

The first version produces codebases where every contributor picks differently. The second produces codebases where the team can recognize and discuss the patterns. The point of this file is to keep the plugin on the second side.
