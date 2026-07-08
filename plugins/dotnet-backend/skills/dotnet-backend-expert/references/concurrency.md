# Concurrency & Async

The rules a reviewer needs are not "use async" — they are the specific shapes that
compile, pass tests, and then fail under production load. Each entry names the failure.

## Sync-over-async: why it actually breaks

`.Result` / `.Wait()` / `GetAwaiter().GetResult()` on a request path doesn't just
"block a thread" — under load it **starves the thread pool**: each blocked request
pins a pool thread while the continuation it's waiting for needs a pool thread to
run. Throughput collapses in a hill-shaped curve that looks fine in dev (few
concurrent requests) and falls over in production. The fix is never a bigger pool;
it's making the path async end-to-end.

`Task.Run(() => SomethingSync())` inside a request to "make it async" is the same
bug with extra steps — it burns two threads per request instead of one.

## `async void` crashes the process

An exception thrown in an `async void` method cannot be observed by any caller —
it is rethrown on the SynchronizationContext/thread pool and takes the process
down. `async void` is legal only as an event-handler signature. In review, every
`async void` outside an event handler is **blocking** severity, no discussion.

## DbContext is not thread-safe — the `Task.WhenAll` trap

The most common concurrency bug in EF Core code reviews:

```csharp
// BAD — two queries racing on ONE DbContext. Throws
// "A second operation was started on this context instance" — or worse,
// corrupts tracked state silently under just the wrong interleaving.
var orders  = db.Orders.Where(...).ToListAsync(ct);
var invoices = db.Invoices.Where(...).ToListAsync(ct);
await Task.WhenAll(orders, invoices);

// GOOD — sequential on one context (usually fine: same connection anyway)…
var orders   = await db.Orders.Where(...).ToListAsync(ct);
var invoices = await db.Invoices.Where(...).ToListAsync(ct);

// …or parallel with one context each, via IDbContextFactory<T>
await using var db1 = await factory.CreateDbContextAsync(ct);
await using var db2 = await factory.CreateDbContextAsync(ct);
```

## CancellationToken discipline

- **Accept and pass `ct` through every async signature.** A missing token on one
  layer severs cancellation for everything below it — the request aborts, the
  query keeps running.
- **Honor it in loops**: `while (!ct.IsCancellationRequested)` or
  `ct.ThrowIfCancellationRequested()` inside long CPU stretches.
- **Shutdown vs request tokens are different lifetimes.** In a hosted service,
  work triggered by a request but expected to survive it needs
  `CancellationTokenSource.CreateLinkedTokenSource` thinking — decide explicitly
  which lifetime owns the work, don't just forward whichever token was in scope.
- `OperationCanceledException` on shutdown is not an error — catch it at the
  `ExecuteAsync` boundary and exit quietly; logging it as `Error` trains people
  to ignore the error log.

## Locks

- .NET 9+: `private readonly System.Threading.Lock _lock = new();` with
  `lock (_lock)` — dedicated type, faster, can't be locked by strangers.
- Pre-9: `private readonly object _lock = new();`. Never `lock (this)`, a
  string, or a `Type` — all are reachable from other code; that's a deadlock at
  a distance.
- **You cannot `await` inside `lock`** (compiler error with `Lock`/`lock`, and
  hand-rolled Monitor equivalents deadlock). If the critical section must await,
  the tool is `SemaphoreSlim(1,1)` with `WaitAsync`/`Release` in `try/finally` —
  and that's also the signal to ask whether the design wants a `Channel<T>` and
  a single owner instead.

## Coordination tools, by job

| Job | Tool | The trap it replaces |
| --- | --- | --- |
| Producer/consumer pipeline | `Channel<T>` (bounded) | `List<T>` + `lock` + polling loop (DN-14) |
| At most K concurrent async ops over a set | `Parallel.ForEachAsync` with `MaxDegreeOfParallelism` | hand-rolled `SemaphoreSlim` + `Task.WhenAll` |
| Cap concurrency on a shared resource | `SemaphoreSlim` (release in `finally`) | uncapped fan-out that DDoSes your own dependency |
| Periodic background work | `PeriodicTimer` + `await timer.WaitForNextTickAsync(ct)` | `while (true) { …; await Task.Delay(...) }` — drifts, and swallows the tick on slow iterations |
| Single writer, many readers of a snapshot | immutable object swapped via `Volatile`/`Interlocked` | `ReaderWriterLockSlim` guarding a mutable graph |

**Bounded channels**: choose the `BoundedChannelFullMode` deliberately. `Wait` gives
backpressure (producer slows down); `DropOldest`/`DropNewest` silently lose data —
acceptable for telemetry, a production incident for work items. An unbounded channel
is a memory leak with a delay fuse: fine only when the producer is provably slower
than the consumer.

## Hosted services / background work

- `ExecuteAsync` runs **synchronously until its first await** — a hosted service
  that does heavy setup before the first `await` delays the entire host's startup.
  `await Task.Yield()` first if setup is nontrivial.
- Scoped dependencies (like `DbContext`) can't be constructor-injected into a
  singleton hosted service (DN-05). Inject `IServiceScopeFactory`, create a scope
  per unit of work, dispose it.
- **Fire-and-forget is a bug until proven otherwise** (DN-13). `_ = DoWorkAsync()`
  loses the exception and the shutdown story. Work that matters gets an owner: a
  channel + consumer service, tracked tasks drained on shutdown, or a real queue.

## `ValueTask` and `ConfigureAwait` — two things models over-apply

- `ValueTask` may be awaited **once**. Storing it, awaiting twice, or
  `Task.WhenAll`-ing it is undefined behavior, not a perf tweak. Use it only on
  measured hot paths that usually complete synchronously; `Task` is the default.
- `ConfigureAwait(false)` does nothing in ASP.NET Core application code (there is
  no SynchronizationContext) — it's required posture in general-purpose libraries
  only. Flag it as noise in app code, not as a missing best practice.

## Concurrency smells

| Smell | Signal | Fix |
| --- | --- | --- |
| Sync-over-async | `.Result`, `.Wait()`, blocking wrappers | make the whole path async (thread-pool starvation) |
| `async void` | anywhere outside event handlers | `async Task`; unobservable exceptions crash the process |
| Parallel queries, one DbContext | `Task.WhenAll` over queries on the same context | sequential, or `IDbContextFactory<T>` |
| Hidden shared state | static/singleton mutable collections | immutable snapshots, or explicit ownership + `Channel<T>` (DN-06) |
| Ad hoc queue | list + lock + polling loop | `Channel<T>` (DN-14) |
| Scoped-in-worker bug | hosted service uses scoped dependency directly | `IServiceScopeFactory` per work item (DN-05) |
| `Task.Delay` heartbeat loop | `while(true)` + delay | `PeriodicTimer` |
| Severed cancellation | a layer that drops the `ct` parameter | thread the token through every async signature |
