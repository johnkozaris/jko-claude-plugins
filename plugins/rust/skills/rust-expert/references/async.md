# Async Rust

## When to Use Async

**Use async for:** many concurrent I/O-bound operations (hundreds/thousands of connections), complex concurrency composition (select!, timeouts, racing), constrained environments.

**Avoid async for:**
- CPU-bound work → use Rayon or `std::thread`
- Simple/low-concurrency apps → sync is simpler, faster to build, easier to debug
- File system I/O → most OSes lack true async FS; tokio::fs just wraps `spawn_blocking`
- Libraries → avoid forcing a runtime choice on consumers; offer sync APIs unless the library is inherently async (networking)
- Anything with <10 concurrent operations → threads are fine and simpler

**Strategy:** Keep domain/business logic synchronous. Use async only at I/O boundaries. Async is about concurrency, not performance — a single-threaded sync server handling 10 requests is simpler than an async one.

### Overboard Async Anti-Patterns

- **Async for a single HTTP call.** `reqwest::blocking::get()` exists. Don't pull in a full tokio runtime for one request.
- **`#[tokio::main]` on a CLI that does sequential work.** If you `await` one thing at a time, you don't need async.
- **Making every function async "just in case."** Async infects signatures upward. A function that never awaits should not be async.
- **`async fn` that immediately calls `spawn_blocking`.** If the entire body is blocking, just make it a sync function called from `spawn_blocking` at the call site.

## The Cardinal Rule

No task should spend more than **10-100 microseconds** between `.await` points.

## Blocking Taxonomy (Alice Ryhl)

| Blocking type | Fix |
|---|---|
| `std::fs` / `std::net` in async fn | `tokio::fs`, `tokio::net`, or `spawn_blocking` |
| `std::thread::sleep` | `tokio::time::sleep(...).await` |
| CPU-bound computation | `spawn_blocking` + `rayon` inside |
| `MutexGuard` held across `.await` | Drop guard before `.await`, or use `tokio::sync::Mutex` |
| `block_on` inside async context | Never — causes panic or deadlock |
| Long loop without yielding | `tokio::task::yield_now().await` every N iterations |

## Mutex Selection

- **`std::sync::Mutex`**: Use for short, non-async critical sections. Faster than tokio's Mutex.
- **`tokio::sync::Mutex`**: Only when the lock must be held across an `.await` point.
- **Prefer channels over shared state** when the design allows.

## Cancellation Safety

Futures are cancelled by **dropping** them. A future can be dropped at any `.await` point.

### The select! Loop Problem
```rust
// BAD: future recreated each iteration, losing partial state
loop {
    tokio::select! {
        result = some_operation() => { ... }
        _ = shutdown.recv() => break,
    }
}

// GOOD: create once, resume via mutable reference
let op = some_operation();
tokio::pin!(op);
loop {
    tokio::select! {
        result = &mut op => { ... }
        _ = shutdown.recv() => break,
    }
}
```

### Common Cancel-Unsafe Operations
- `tokio::sync::mpsc::Sender::send()` — value lost if cancelled mid-send
- `AsyncWrite::write_all()` — partial write, position lost

### The reserve() Pattern (Cancel-Safe Sends)
```rust
// BAD: send() is cancel-unsafe
sender.send(value).await?;

// GOOD: reserve() is cancel-safe, send() on permit is sync
let permit = sender.reserve().await?;
permit.send(value);
```

### Shutdown: CancellationToken > task.abort()
```rust
let token = CancellationToken::new();
tokio::spawn(async move {
    tokio::select! {
        _ = token.cancelled() => { /* clean shutdown */ }
        _ = do_work() => {}
    }
});
token.cancel();  // cooperative, clean
```

`task.abort()` cancels at arbitrary await points and is extremely hard to make safe.

## Bounded Concurrency

Never do unbounded `tokio::spawn` in accept loops:

```rust
let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));
let permit = sem.clone().acquire_owned().await?;
tokio::spawn(async move {
    let _permit = permit;
    handle(conn).await;
});
```

## async fn in Traits (Rust 1.75+)

| Use case | Approach |
|---|---|
| Static dispatch (most code) | Native `async fn` in trait — zero allocation, no crate |
| Need `dyn Trait` (plugins, DI) | `async-trait` crate — one `Box::pin` per call |
| Need `Send` guarantee | `trait-variant` crate or manual `-> impl Future + Send` |
| Single-threaded (WASM, LocalSet) | `#[async_trait(?Send)]` |

**Default to native async fn in trait.** Only reach for `async-trait` when you need `dyn Trait`.

## JoinSet — Structured Task Groups

`JoinSet` manages dynamic task groups with RAII cleanup — all tasks abort when the set is dropped:

```rust
let mut set = JoinSet::new();
for url in urls {
    set.spawn(async move { fetch(url).await });
}
while let Some(result) = set.join_next().await {
    handle(result??);
}
// All done. Or drop set early to cancel remaining.
```

For bounded concurrency, pair with a `Semaphore`:
```rust
let sem = Arc::new(Semaphore::new(16));
for item in items {
    let permit = sem.clone().acquire_owned().await?;
    set.spawn(async move { let _p = permit; process(item).await });
}
```

| Tool | Dynamic count | Auto-cancel on drop | Ordered results |
|---|---|---|---|
| `tokio::join!` | No (fixed) | N/A | Yes |
| `JoinSet` | Yes | Yes | No (completion order) |
| `FuturesUnordered` | Yes | No | No |

## Async Cleanup (No Async Drop Yet)

Rust has no `async Drop`. When a type needs async cleanup (close connection, flush buffer), provide an explicit async method:

```rust
impl MyService {
    /// Call before dropping to cleanly shut down.
    pub async fn shutdown(self) {
        self.connection.close().await;
        self.buffer.flush().await;
    }
}
```

If consumers forget `shutdown()`, the sync `Drop` impl should do best-effort cleanup (log a warning, signal a background task). Never block in `Drop`.

## "Future is not Send" — Quick Fix Guide

| Cause | Fix |
|---|---|
| `Rc<T>` held across `.await` | Replace with `Arc<T>` |
| `RefCell<T>` held across `.await` | Replace with `tokio::sync::Mutex<T>` |
| `MutexGuard` held across `.await` | Scope guard in a block, drop before `.await` |
| Non-Send type in async closure | Move the work to `spawn_blocking` |
| `#[async_trait]` without `Send` | Use `#[async_trait(?Send)]` or switch to native `async fn in trait` |
| `Cell<T>` in an async fn | Extract the value before `.await` |

## spawn_blocking Best Practices

```rust
// Use for: sync I/O, CPU work, calling blocking C libraries
let result = tokio::task::spawn_blocking(move || {
    expensive_sync_computation()
}).await?;

// For CPU parallelism: spawn_blocking + rayon inside
tokio::task::spawn_blocking(|| {
    rayon::scope(|s| { /* parallel CPU work */ });
}).await?;
```

**Do NOT:**
- Nest `block_on` inside `spawn_blocking` — creates a second runtime
- Use `spawn_blocking` for trivial operations (< 1μs) — the scheduling overhead exceeds the work
- Forget to `.await` the JoinHandle — the work runs but errors are lost
- Assume `abort()` stops a blocking task — **`spawn_blocking` tasks cannot be cancelled once running**. If you need cancellable blocking work, use a channel to signal the work to stop itself.

**Bound the blocking pool** under load:
```rust
tokio::runtime::Builder::new_multi_thread()
    .max_blocking_threads(32)  // default is ~512, too many for CPU work
    .build()
```

## Tokio Task Budget (Automatic Cooperative Yielding)

Tokio's built-in I/O primitives (sockets, channels, timers) automatically yield after a budget of operations per poll. This prevents one busy task from starving others — **as long as you use Tokio's own primitives.** Custom `Future` implementations or tight `loop {}` blocks bypass this budget and must yield manually with `tokio::task::yield_now().await`.

## Pin Practical Guide

| Scenario | Pattern |
|---|---|
| Awaiting directly | Nothing needed |
| `select!` over a stored future | `tokio::pin!(future)` then `&mut future` |
| Dynamic collection of futures | `Box::pin(future)` |
| Custom Future impl | `pin-project` crate |
| Stack pinning (no heap) | `std::pin::pin!` macro (stable since 1.68) |
