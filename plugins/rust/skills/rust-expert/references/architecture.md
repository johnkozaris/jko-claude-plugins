# Architecture Patterns for Rust

This reference describes the architectures you'll see in real Rust codebases. It doesn't pick between them — that decision depends on goals, team, and roadmap that the plugin can't see. The job here is detection and consistency: figure out what a codebase is already using, help the developer stay consistent with it, and flag patterns that are bad regardless of context.

The biggest harm an architecture-prescribing plugin can do is suggest a new pattern for one part of a codebase that already uses a different pattern. Mixed architectures in one repo cost more to maintain than any single choice would.

---

## Detecting the architecture

Before suggesting anything, identify what's already there. The signatures are usually obvious from a few minutes of reading.

**Folder layout.** `domain/`, `inbound/`, `outbound/` (or `application/`) is hexagonal. `crates/*-core` and `crates/*-adapters` is hexagonal at the workspace level. Feature folders (`src/feature_a/`, `src/feature_b/`) with no layer separation is module-driven. A workspace member with a name third parties depend on (`*-tile`, `*-sdk`, `*-api`) signals a plugin SDK pattern.

**State management.** `Arc<Mutex<>>` shared between handlers is module-driven (or layered) with shared mutable state. State behind `Handle` types wrapping `mpsc::Sender<Msg>` is the actor pattern. A protocol layer with `handle_input(&[u8])` methods and no `await`s is sans-IO. Types carrying phantom parameters that change as methods run (`Connection<Disconnected>` → `Connection<Connected>`) is typestate.

**Tests.** Tests that spin up real Postgres via `testcontainers` are typical of layered or module-driven code. Tests that swap in an in-memory trait implementation suggest hexagonal. Tests that feed byte sequences into a state machine are sans-IO.

**Trait usage.** A small number of port traits with two-plus implementors each (real + fake) is hexagonal. Many traits with one implementor each is over-abstraction — flag it. No `pub trait` for business logic is module-driven.

Once you identify the dominant pattern, **stay consistent with it**. Suggestions inside a codebase should match that codebase's architecture. If it's hexagonal, new features go through the existing port/adapter split. If it's module-driven, refactor by splitting modules — don't suggest extracting port traits.

---

## Module-driven

Just write modules. Logic in `lib.rs`, thin `main.rs`, folders by feature. No layers, no ports — the structure emerges as the project grows.

```
src/
  lib.rs            # pub fn run(args: Args) -> Result<()>
  main.rs           # 10-30 lines: parse args, init logging, call run()
  error.rs          # thiserror::Error
  feature_a/
  feature_b/
```

ripgrep started this way; fd and bat have stayed this way. Cheap refactoring, no architectural commitments to break. The upgrade signal is usually one of: two binaries need to share non-trivial code, build times start hurting iteration, or `pub(crate)` is sprouting everywhere to enforce something the module system can't quite capture.

**Watch for**: business logic and I/O getting tangled into the same functions; tests slow because they need real infrastructure; a single `Arc<Mutex<AppState>>` accreting fields until it's a god-object; files past a thousand lines. None of these are reasons to bolt on a heavier architecture — they're reasons to refactor within the style: split files, extract pure helpers, scope shared state into smaller containers.

---

## Functional core, imperative shell

Pure logic at the center, async I/O at the edges. The core is `fn` (no side effects, no async), the shell is `async fn` (database calls, HTTP, timers). Rust's function coloring enforces the boundary: the core can't accidentally take an I/O dependency because the type system won't compile it.

```rust
// core — pure, returns descriptions of work
pub fn plan_invoice(state: &State, cmd: NewInvoice) -> Vec<Effect> { /* ... */ }

// shell — async, executes effects
pub async fn execute(effects: Vec<Effect>, services: &Services) -> Result<()> { /* ... */ }
```

rustc and rust-analyzer use a related discipline internally (their query systems are built on pure providers for cacheability), though their own docs call it "query-based incremental compilation" rather than FC/IS. `regex` keeps a pure regex engine separate from any I/O. comrak's three-stage shape (input → AST → render) is FC/IS-adjacent, but its parser uses interior mutation throughout — so use it as a structural example, not as "pure functional core."

**Watch for**: `async` creeping into the core (someone adds an `async fn` to a function in what's supposed to be the core, and async propagates upward through everything that calls it). Also: the core taking dependencies on services through traits that turn out to be async traits in disguise. When reviewing this style, ask every async function whether it's actually doing I/O or whether the logic should move to a pure function.

---

## Actor pattern

One task owns the state; everyone else holds a cheap-cloneable `Handle` that wraps an `mpsc::Sender<Msg>`. Messages can carry a `oneshot::Sender<Reply>` for request/reply.

```rust
#[derive(Clone)]
pub struct CounterHandle { tx: mpsc::Sender<Msg> }

enum Msg {
    Inc(oneshot::Sender<u64>),
}

impl CounterHandle {
    pub fn spawn() -> Self {
        let (tx, rx) = mpsc::channel(32);  // BOUNDED — backpressure
        tokio::spawn(run(rx));
        Self { tx }
    }
    pub async fn inc(&self) -> u64 {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx.send(Msg::Inc(reply_tx)).await.expect("actor down");
        reply_rx.await.expect("actor dropped reply")
    }
}
```

This is the canonical [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/) pattern. Used in most non-trivial Tokio codebases. Frameworks exist (Actix, ractor) but the hand-rolled version is short enough that most teams write it directly.

**Watch for**: `unbounded_channel` replacing the bounded one (someone got tired of senders blocking under load, and now the system has no backpressure — OOM under sustained overload). Actors that take too long to process a message and block subsequent messages (long-running work should move to a worker actor or `spawn_blocking`). `Handle` types accreting fields beyond the channel sender (they're turning back into the shared-state object the pattern avoids).

---

## Sans-IO

The protocol is a pure state machine that consumes bytes and produces bytes or events. No async runtime, no file I/O, no syscalls — just `fn handle_input(&mut self, bytes: &[u8])` style methods. The crate user wires the bytes to wherever they actually come from.

Used by `quinn-proto` (QUIC), `rustls` (TLS), and `httparse` (HTTP/1). The same crate works in sync code, async code, or embedded. (Note: `h2` is built on Tokio, not sans-IO; `hickory-proto` includes Tokio transports alongside its message types — only the encoder/decoder pieces of it could be used sans-IO.)

**Watch for**: any direct I/O call in the protocol crate. A single `tokio::spawn` or `std::fs::read` breaks the runtime-independence and users on other runtimes can no longer use the crate. The other smell is internal buffers growing unboundedly because the state machine has no backpressure signals for the wrapper to act on.

---

## Pipeline / dataflow

Data flows through distinct named stages, each with a clean input/output type. rustc (lex → parse → typecheck → codegen), ripgrep (walker → matcher → searcher → printer), swc/biome/oxc (parse → transform → emit), datafusion (logical plan → optimization → physical plan → execution).

**Watch for**: stages developing side channels back to earlier stages, breaking the linear flow. A stage that mutates global state, or calls back into an earlier stage's data structures, becomes hard to test in isolation. Also: stages that combine too many concerns. A stage that "parses, validates, and transforms" usually wants to be three stages.

---

## State machine / typestate

States encoded as type parameters. The compiler refuses to let you call methods that aren't valid in the current state.

```rust
struct Connection<S> { socket: TcpStream, _state: PhantomData<S> }
struct Disconnected; struct Connected;

impl Connection<Disconnected> {
    pub fn connect(self) -> Result<Connection<Connected>, Error> { /* ... */ }
}
impl Connection<Connected> {
    pub fn send(&self, msg: &[u8]) -> Result<(), Error> { /* ... */ }
}
```

`serde::Serializer` is a typestate machine — you can't call `serialize_field` before `serialize_struct`. The typestate pattern is also pervasive in embedded HAL implementations (stm32f4xx-hal, esp-hal, etc.) for GPIO pin modes (`Pin<Input<Floating>>` → `Pin<Output<PushPull>>` via `into_input`/`into_output`); the `embedded-hal` 1.0 traits themselves are plain behavior traits, not typestate. TLS and HTTP handshakes also use typestate at the protocol-state level. The `http` crate's `Request::builder()` is NOT typestate — it's a plain runtime builder that accumulates errors and surfaces them at `.body()`.

**Watch for**: over-application. A type with five-plus states is usually clearer as a runtime state machine (or `statig`). Also: escape hatches that defeat the purpose — a `fn force_send_anyway(self) -> Connection<Connected>` that bypasses the connect step means the compiler isn't enforcing the protocol anymore.

---

## Plugin registry

Define a trait (or, for WASM, a WIT interface). Plugins implement it. The host drives them. Three flavors:

- **In-process trait objects**: `Vec<Box<dyn Plugin>>`, linked at compile time, fast, no sandbox. Bevy's `Plugin` trait is the canonical example.
- **Dynamic loading**: `libloading` + C ABI (or `abi_stable` for checked Rust-to-Rust). No sandbox.
- **WASM Components**: `wasmtime` + `wit-bindgen`. Sandboxed by default. The right answer when third-party plugins matter.

Zed extensions use WASM Components (Wasmtime + `wit-bindgen`, with WIT interfaces). Zellij plugins are plain WebAssembly modules — `wasm32-wasip1` running on the `wasmi` interpreter (since v0.44.0), with Protobuf for the host-plugin interface, NOT the Component Model. Bevy uses in-process trait objects (`Plugin` trait, registered into a `PluginRegistry`). Tower's `Layer`/`Service` is a middleware-shaped variant — `Layer` decorates `Service`, `ServiceBuilder` stacks them.

**Watch for**: the host trait accreting methods only one or two plugins use, forcing every plugin to implement irrelevant stubs (give those plugins their own extension trait instead). Also: plugins acquiring privileged access — direct database, filesystem, or shared mutable state access — which defeats the sandbox.

---

## Compositional / combinator-based

This isn't really a top-level architecture; it's a way of writing that Rust rewards. Iterators, futures, parser combinators, Tower middleware — `filter_map` instead of `filter().map()`, `Service::layer(auth).layer(logging)` instead of bespoke middleware. It shows up inside almost every other pattern.

**Watch for**: chains long enough to be unreadable (seven combinators each doing real work needs intermediate `let` bindings). Also: reaching for combinators where a plain `for` loop or `match` would read more directly.

---

## Hexagonal / ports-and-adapters

The domain defines what it needs as trait interfaces (ports); infrastructure implements them (adapters). Infrastructure depends on the domain, never the reverse.

```rust
// domain — knows nothing about HTTP or DB
pub trait UserRepo: Send + Sync + 'static {
    async fn find(&self, id: UserId) -> Result<Option<User>, RepoError>;
    async fn save(&self, user: &User) -> Result<(), RepoError>;
}

pub struct AuthService<R: UserRepo> { repo: R }

// adapter — Postgres impl
pub struct PgUserRepo { pool: PgPool }
impl UserRepo for PgUserRepo { /* ... */ }

// composition root in main.rs picks the wiring
let svc = AuthService { repo: PgUserRepo::new(pool) };
```

What it gives you: substitutability (in-memory fake for tests, real database in prod). Domain testable without infrastructure. Multiple teams can work in parallel on domain vs adapters. What it costs: a trait per port, generic parameters or `Arc<dyn>` at every service, type mapping at adapter boundaries, complex composition root.

**Watch for**: the domain layer reaching out to infrastructure. Any infrastructure-specific import (`sqlx`, `reqwest`, `tokio::fs`) inside the domain is the layering slipping. Also: over-trait-ing — `Database`, `Logger`, single-API HTTP clients usually don't need ports because they're not going to be swapped. The cost of trait-per-port is paid every day; the supposed benefit pays off once if it ever pays at all.

---

## Patterns compose

Most real systems use more than one of these at different layers. A hexagonal microservice often has functional-core/imperative-shell inside each application service and an actor for any stateful long-running piece. rustc combines pipeline with functional core. The patterns answer different questions — "how do I wire adapters?" (hexagonal), "how do I separate effects?" (FC/IS), "how do I structure stateful concurrency?" (actor), "how do I avoid runtime lock-in?" (sans-IO) — so a codebase that uses several, each in the right place, is internally consistent.

For games (Bevy ECS), GUI apps (Iced MVU, egui immediate-mode), and embedded firmware (Embassy's async reactor), the framework picks the architecture. CQRS plus event sourcing shows up almost exclusively in finance, healthcare, and audit-heavy systems. None of these are patterns the plugin proposes for general use; they come along with their ecosystems.

---

## Patterns to flag regardless of context

Some shapes aren't "another architecture" — they're things that have gone wrong and should be flagged in any codebase.

**Mixed architectures in one repo.** Two services using completely different patterns — one hexagonal, another module-driven with `Arc<Mutex<>>` shared state — is itself a finding. The maintenance cost scales with the number of patterns the team holds in their heads. Align with whichever the majority uses.

**OOP inheritance via `Deref` chains.** A `BaseUser` struct with `Deref<Target = Account>`, `AdminUser` with `Deref<Target = BaseUser>` — someone is recreating class inheritance. Rust doesn't have inheritance for a reason. `Deref`-as-inheritance breaks `dyn Trait` dispatch and defeats safety newtypes. Flag on sight.

**God-object `Arc<Mutex<AppState>>`.** One mutex around the whole application state, locked from everywhere. Lock contention dominates, every method grabs the lock, `.await`-while-holding becomes a constant footgun. Split by concern: separate the cache, the counters, the session store, the metrics — each gets the appropriate primitive.

**Stringly-typed APIs in a domain that warrants types.** `String` for `user_id` where a `UserId(Uuid)` newtype would prevent confusion with `OrderId`. Three boolean parameters where an enum would name what they mean. The Rust type system is the strongest tool you have for preventing bugs; not using it on domain values leaves safety on the table.

**Premature workspace splits.** A workspace with eight crates for a three-thousand-line project, where the boundaries are speculative. Tokio's [issue #1318](https://github.com/tokio-rs/tokio/issues/1318) is the canonical "we tried this and merged it back" story. Split when there's a concrete consumer (compile-time pain, two binaries sharing code, published SDK, FFI shim, team ownership).

**One-implementor trait obsession.** Every type behind a trait that has exactly one implementor and no test fake. Sometimes this is groundwork for a planned second implementation — but often it's "I might want to swap this someday," which is just speculation tax. The cost is paid every day; the benefit pays off once if at all.

**`Box<dyn Error>` in library public APIs.** Consumers can't `match` on errors, can't handle different errors differently. Libraries should use `thiserror` for typed errors; applications can use `anyhow`. Mixing them (`anyhow::Error` in library signatures) is the worst of both worlds.

**Mock-only tests.** A suite where every collaborator is mocked, every dependency stubbed, and every assertion checks that the mocks were called correctly. This verifies that the test setup matches the test setup. Real fakes (in-memory trait implementations) beat mocks for most non-IO collaborators; `testcontainers` and `wiremock` cover the I/O boundary for real.

These are the things to push back on regardless of which architecture the codebase otherwise uses.

---

## Sources

- [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/)
- [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)
- Sans-IO examples: [quinn-proto](https://docs.rs/quinn-proto), [rustls](https://docs.rs/rustls), [httparse](https://docs.rs/httparse), [h2](https://docs.rs/h2)
- WASM Component Model: [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen)
- "Master Hexagonal Architecture in Rust" (howtocodeit) and "Hexagonal Architecture in Rust" (Cogs and Levers) — well-cited Rust hexagonal tutorials; search for current URLs
- r/rust discussions on hexagonal — useful pushback against dogmatic application of the pattern
