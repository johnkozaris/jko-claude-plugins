# Concurrency

## Do You Actually Need Concurrency?

Before reaching for threads, async, or channels — ask: **is sequential code fast enough?**

### Overboard Concurrency Anti-Patterns

**Async for CPU-bound work.** Async is for I/O concurrency. CPU-bound computation on a tokio worker starves other tasks. Use Rayon or `spawn_blocking` instead.

**Threads for a 10ms task.** Spawning a thread costs ~20μs on Linux. If the work itself is fast, the thread overhead dominates. Just run it inline.

**Arc<Mutex<T>> for data only one thread touches.** If ownership analysis shows only one thread ever accesses the data, plain `T` or `&mut T` suffices. Arc<Mutex> is for genuinely shared state, not a default.

**Channels between two functions in the same task.** Channels are for cross-thread communication. Within a single task or function, use a `Vec`, iterator, or direct calls.

**`par_iter()` on small collections.** Rayon's work-stealing overhead exceeds the parallelism gains for collections under ~1000 elements. Benchmark before parallelizing.

**Multiple tokio runtimes.** One runtime is almost always enough. Multiple runtimes fragment the thread pool, waste memory, and complicate shutdown. Exception: isolating blocking work in a dedicated runtime.

**Shared mutable state when message passing works.** If you can model the problem as "send a request, get a response," channels or actors are simpler and deadlock-free. Shared state is for when multiple threads need simultaneous read access.

### The Litmus Test

Ask: "If I removed all concurrency from this code and ran it sequentially, would it be too slow?" If you cannot point to a measured bottleneck, you don't need concurrency yet.

## Decision Tree

```
Do you actually need concurrency?
  ├── No measured bottleneck → stay sequential
  ├── I/O bound (network, DB, file) → async/await
  ├── CPU bound (computation, parsing) → Rayon or std::thread
  └── Both → async for I/O, spawn_blocking for CPU

Need to share data across threads?
  ├── Read-only, never changes → Arc<T> (no lock needed)
  ├── Read-heavy, rarely written → Arc<RwLock<T>> or ArcSwap
  ├── Write-heavy, contended → Arc<Mutex<T>> (profile first!)
  ├── Simple counter/flag → AtomicUsize / AtomicBool
  └── Complex ownership → channels (hand off, don't share)

Need a concurrent map?
  ├── Read-heavy → dashmap (sharded RwLock)
  ├── Many small writes → dashmap
  ├── Heavy contention → consider per-thread maps + merge
  └── Lock-free needed → papaya or flurry (advanced)

Need thread communication?
  ├── Single consumer, sync → std::mpsc
  ├── Multi-consumer, sync → crossbeam-channel
  ├── Mixed sync+async → flume
  └── Throughput critical → benchmark kanal

Need to parallelize a collection?
  ├── Large collection (1000+ items) → Rayon par_iter()
  └── Small collection → stay sequential
```

## Structured Concurrency with JoinSet

`tokio::task::JoinSet` manages groups of spawned tasks with structured lifetimes — tasks are cancelled when the JoinSet is dropped:

```rust
use tokio::task::JoinSet;

let mut set = JoinSet::new();

for url in urls {
    set.spawn(async move { fetch(url).await });
}

// Collect results as tasks complete (unordered)
while let Some(result) = set.join_next().await {
    match result {
        Ok(Ok(response)) => process(response),
        Ok(Err(app_err)) => log_error(app_err),
        Err(join_err) => log_panic(join_err),  // task panicked
    }
}
// All tasks complete when loop ends. Or drop `set` to cancel remaining.
```

**Key patterns:**
- `set.spawn()` for fire-and-track
- `set.join_next().await` to collect results
- `set.abort_all()` for forced cancellation
- Dropping `JoinSet` cancels all remaining tasks — structured cleanup
- Bounded concurrency: limit `set.len()` and await before spawning more

## Actor Pattern

Encapsulate state behind a dedicated task. Communicate via channels:

```rust
struct DbActor {
    db: SqlitePool,
    rx: mpsc::Receiver<DbCommand>,
}

enum DbCommand {
    Get { key: String, reply: oneshot::Sender<Option<Value>> },
    Set { key: String, value: Value },
}

impl DbActor {
    async fn run(mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                DbCommand::Get { key, reply } => {
                    let val = self.db.get(&key).await;
                    let _ = reply.send(val);  // ignore if caller dropped
                }
                DbCommand::Set { key, value } => {
                    self.db.set(&key, &value).await;
                }
            }
        }
        // Channel closed = all handles dropped = clean shutdown
    }
}
```

**Why actors beat Arc<Mutex>:**
- No deadlocks — single-owner, sequential processing
- Backpressure — bounded channel blocks senders
- Clean shutdown — drop all handles, actor drains and exits
- Testable — inject a mock channel

## Arc<Mutex<T>> — When It's Wrong and When It's Right

### When It's Wrong

**Config that changes once at startup.** Use `ArcSwap` or `OnceCell`/`OnceLock`:
```rust
// BAD: mutex for read-only-after-init config
let config = Arc::new(Mutex::new(load_config()));

// GOOD: ArcSwap for hot-swappable, OnceLock for set-once
static CONFIG: OnceLock<Config> = OnceLock::new();
CONFIG.set(load_config()).ok();
```

**Data only one task touches.** Just own it:
```rust
// BAD: Arc<Mutex> for private task state
let state = Arc::new(Mutex::new(MyState::new()));
tokio::spawn({
    let state = state.clone();
    async move { /* only this task uses state */ }
});

// GOOD: move ownership into the task
tokio::spawn(async move {
    let mut state = MyState::new();
    // state is owned, no synchronization needed
});
```

**Counters and flags.** Use atomics:
```rust
// BAD
let count = Arc::new(Mutex::new(0u64));

// GOOD
let count = Arc::new(AtomicU64::new(0));
count.fetch_add(1, Ordering::Relaxed);
```

### When It's Right

- Genuinely shared mutable state accessed by multiple threads
- Short critical sections (lock, update, unlock immediately)
- State that doesn't fit the actor or channel model
- Always profile first — is contention actually a problem?

## Deadlock Prevention

1. **Consistent lock ordering.** Define global acquisition order — always lock A before B.
2. **Never hold a lock across a library boundary** that may itself take locks.
3. **Use `try_lock()` with fallback** for non-critical paths.
4. **Avoid holding multiple locks simultaneously.** Reorganize into one structure or use channels.
5. **Never hold a lock across `.await`** — use `tokio::sync::Mutex` only if required.
6. **Prefer ownership handoff over shared locks.** Move data to a dedicated worker via channels.

### Rayon + Mutex = Deadlock

```rust
// BAD: Rayon's work-stealing can re-enter on the same thread
let guard = mutex.lock().unwrap();
data.par_iter().for_each(|item| { ... });  // deadlock!

// GOOD: clone data, release lock, then parallelize
let snapshot = {
    let guard = mutex.lock().unwrap();
    guard.clone()
};
snapshot.par_iter().for_each(|item| { ... });
```

## Backpressure

**Always use bounded channels in production.** Unbounded channels hide load problems until OOM.

```rust
// BAD: unbounded — memory grows without limit under load
let (tx, rx) = mpsc::unbounded_channel();

// GOOD: bounded — senders block when buffer is full
let (tx, rx) = mpsc::channel(100);  // backpressure at 100 pending

// ALSO GOOD: semaphore for bounding task spawning
let sem = Arc::new(Semaphore::new(MAX_CONCURRENT));
loop {
    let permit = sem.clone().acquire_owned().await?;
    let conn = listener.accept().await?;
    tokio::spawn(async move {
        let _permit = permit;  // released on drop
        handle(conn).await;
    });
}
```

**Size your bounds intentionally.** A channel bound of 100 means "I can tolerate 100 messages of lag." Document why.

## Atomics (Mara Bos)

Use atomics for simple shared values instead of Mutex:

### Memory Ordering

| Ordering | Use |
|---|---|
| `Relaxed` | Isolated counters, statistics — no inter-variable sync |
| `Release` / `Acquire` | Producer publishes, consumer reads — the primary tool |
| `SeqCst` | Rarely needed — carries unnecessary performance cost |

**Do not use `SeqCst` as a lazy default.** Understand your actual synchronization needs.

## Modern Concurrency APIs

- **`RwLockWriteGuard::downgrade()`** (1.92) — atomically convert write guard to read guard. Prevents other writers sneaking in.
- **`strict_add`, `strict_sub`, `strict_mul`** (1.91) — always panics on overflow, even in release.
- **`File::lock()`, `lock_shared()`, `try_lock()`** (1.89) — native file locking. Replaces `fd-lock`, `fs2` crates.

## ArcSwap — Lock-Free Hot-Swapping

When you need `Arc<T>` that can be atomically replaced at runtime (config reload, provider swap):

```rust
use arc_swap::ArcSwap;
use std::sync::Arc;

let config = Arc::new(ArcSwap::from_pointee(initial_config));

// Readers (lock-free, nanosecond cost):
let current = config.load();

// Writer (atomic swap):
config.store(Arc::new(new_config));
```

Use `ArcSwapOption` when the value can start as `None`. `load_full()` returns an owned `Arc`, releasing the epoch guard — important for long-lived async operations.

## Send and Sync

| Type | `Send` | `Sync` | Notes |
|---|---|---|---|
| Primitives | Yes | Yes | Trivially safe |
| `Arc<T>` (T: Send+Sync) | Yes | Yes | Thread-safe shared ownership |
| `Mutex<T>` (T: Send) | Yes | Yes | Synchronized access |
| `Rc<T>` | No | No | Single-threaded only |
| `RefCell<T>` | Yes | No | Runtime borrow check not thread-safe |

In async: futures held across `.await` must be `Send` for multi-threaded executors. Replace `Rc` → `Arc`, `RefCell` → `Mutex/RwLock`.

For "Future is not Send" causes and fixes, see the [async reference](async.md#future-is-not-send--quick-fix-guide).

## Concurrent Data Structures

| Crate | Type | Use when |
|---|---|---|
| `dashmap` | Sharded `RwLock<HashMap>` | General concurrent map, good read/write balance |
| `moka` | Concurrent cache with TTL/size eviction | Caching with expiration |
| `papaya` | Lock-free hash map | Extreme read throughput, rarely written |
| `arc-swap` | Atomic `Arc` swap | Config/provider hot-reload |
| `crossbeam::SegQueue` | Lock-free unbounded queue | Work queues |

## Rayon — Data Parallelism

```rust
use rayon::prelude::*;
let sum: i32 = data.par_iter().map(|&x| x * x).sum();
```

**Rules:**
- Never lock a Mutex inside `par_iter()` — use `fold` + `reduce` for accumulation
- `RAYON_NUM_THREADS=1` for debugging to isolate concurrency bugs
- Benchmark: if sequential is within 2x, the complexity isn't worth it
- Minimum collection size ~1000 items before parallelism helps

## Channel Selection Guide

| Library | MPMC | Async | `select!` | Notes |
|---|---|---|---|---|
| `std::mpsc` | No | No | No | Zero deps, single consumer |
| `crossbeam-channel` | Yes | No | Yes | Mature, fast, feature-rich |
| `flume` | Yes | Yes | No | Best sync+async bridge, no unsafe |
| `kanal` | Yes | Yes | No | Highest raw throughput |
| `tokio::sync::mpsc` | No | Yes | Yes (via `select!`) | Native tokio, bounded |
| `tokio::sync::broadcast` | Yes | Yes | Yes | Fan-out to multiple receivers |
| `tokio::sync::watch` | No | Yes | No | Latest-value broadcast (config changes) |
| `tokio::sync::oneshot` | No | Yes | No | Single response (request/reply) |

## Alternative Mutex: parking_lot

`parking_lot::Mutex` is faster than `std::sync::Mutex` in the uncontended case (no syscall), smaller (1 byte vs 40+ bytes on Linux), and never poisons. Drop-in replacement for many use cases:

```rust
use parking_lot::Mutex;
let data = Mutex::new(vec![]);
data.lock().push(42);  // no .unwrap() needed — no poisoning
```

Also provides `parking_lot::RwLock` (writer-biased, faster than std), `FairMutex`, and `ReentrantMutex`.

## Testing Concurrent Code: loom

`loom` exhaustively tests all possible thread interleavings. Essential for verifying lock-free data structures and custom synchronization:

```rust
#[test]
fn test_concurrent_access() {
    loom::model(|| {
        let data = loom::sync::Arc::new(loom::sync::Mutex::new(0));
        // loom explores all possible interleavings
    });
}
```

## Crossbeam Utilities

- `CachePadded<T>` — prevents false sharing between cache lines
- `Backoff` — adaptive spin-wait (spin → yield → park)
- `scope` — borrow stack data in spawned threads (also in `std::thread::scope` since 1.63)
