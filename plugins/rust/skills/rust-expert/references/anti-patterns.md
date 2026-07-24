# Anti-Pattern Catalog

## High severity — classify by concrete consequence

Treat these as blocking only when they create a plausible correctness,
soundness, security, or availability failure. Otherwise classify them as
important and explain the actual cost.

### Clone to Satisfy Borrow Checker

Sprinkling `.clone()` on owned heap data (`Vec`, `String`, large structs) to silence the compiler creates independent copies and wastes allocations. It is blocking only when the copy breaks identity or state semantics; otherwise it is important. Fix: restructure ownership, use references, or use Rc/Arc for genuine sharing. **Not a smell:** `Arc::clone(&handle)` of long-lived service handles, DB pools, channels — that is the M-SERVICES-CLONE pattern. The author should be able to name in one sentence why each clone is correct; if not, it's the anti-pattern.

### `unwrap()` / `expect()` in Production Paths

Both panic on `None`/`Err`. This is blocking on plausible external input or a recoverable library path. A documented `expect("invariant because ...")` remains valid when the type system cannot express the invariant. Otherwise use `?` with informative context, `match`, `if let`, or a deliberate fallback.

### `From` for Fallible Conversions

`From` is a contract that conversion always succeeds. Using `unwrap()` inside `From` hides fallibility. Fix: use `TryFrom` which returns `Result`.

### Blocking I/O in Async

`std::fs`, `std::net`, `std::thread::sleep` inside async functions block the executor thread. Fix: use async equivalents or `spawn_blocking`.

### MutexGuard Held Across `.await`

Lock is held while task is suspended, starving other tasks. Fix: drop guard before `.await`, or use `tokio::sync::Mutex`.

### `&'a mut self` on Struct Methods

If struct is generic over `'a`, this borrows self for its entire lifetime. Fix: just write `&mut self`.

### Arc/Rc Reference Cycles

Strong references in cycles never reach zero — permanent memory leak. Fix: use `Weak<T>` for back-references.

### Panic Inside `Drop`

If `drop` panics during unwinding (double panic), the process aborts. Fix: never panic in Drop.

## Important — Should Fix

### `&String` / `&Vec<T>` Parameters

Unnecessarily restrictive. Fix: use `&str` / `&[T]` — accepts both owned and borrowed via deref coercion.

### Arc<Mutex<T>> Everywhere

Atomic operations are expensive and lock contention causes thread blocking. Overuse can dominate CPU time. Fix: `Rc<RefCell<T>>` for single-thread, channels for communication, `ArcSwap` for read-heavy config. Always profile before reaching for Arc<Mutex>.

### `..Default::default()` on Structs in Production Code

Silently uses wrong defaults when new fields are added — compiler won't warn. Fix: explicitly set all fields. **Exception:** acceptable in test fixtures and builder intermediaries where "I care about these fields, defaults for the rest" is the explicit intent.

### Catch-All `_` in Match on Owned Enums

Swallows newly-added variants with no compiler warning. Fix: match all variants explicitly.

### `#![deny(warnings)]` in Source Code

Builds fail when rustc introduces new lints. Fix: on Rust 1.97+, use `CARGO_BUILD_WARNINGS=deny` in CI so the policy stays outside source and does not invalidate the build cache.

### Ignoring `Result` Return Values

Silently discarding errors. Fix: use `let _ =` only with a comment explaining why, or handle the error.

### Rc<RefCell<T>> Overuse

Signal that ownership structure is wrong — bypasses borrow checker at runtime (panics on conflicts). Fix: restructure ownership.

### `Box<dyn Error>` in Library APIs

Callers can't match on specific errors. Fix: typed error enums with `thiserror`.

## Nit — Fix If Convenient

### Boolean Function Parameters

`process(data, true, false)` is unreadable. Fix: use enum types.

### Overusing `mut`

Rust defaults to immutability for a reason. Fix: only declare `mut` when mutation actually needed. Use shadowing for temporary mutability.

### `collect()` Then Immediately Iterate

Wastes an allocation. Fix: chain iterator adapters directly.

### Not Using `const fn` Where Applicable

Pure functions with compile-time-known inputs. Fix: add `const` to move work to compile time.

### String Concatenation with `+`

Creates intermediate allocations. Fix: `format!()` for small cases, `String::with_capacity()` + `push_str()` for large.

### Not Using Entry API for Maps

Manual get/check/insert is verbose and double-hashes. Fix: `map.entry(key).or_insert_with(|| value)`.

## Overboard Concurrency

### Async Where Sync Suffices

A CLI that awaits one HTTP call doesn't need `#[tokio::main]`. Use `reqwest::blocking` or `ureq`. Every `async` in a function signature infects all callers.

### Arc<Mutex<T>> for Private State

If only one task/thread ever touches the data, own it directly. Arc<Mutex> is for genuinely shared mutable state, not a default wrapper.

### Spawning Tasks for Sequential Work

`tokio::spawn` followed immediately by `.await` is just a function call with extra overhead. Only spawn when you need concurrent execution.

### par_iter() on Tiny Collections

Rayon's thread pool overhead dominates for collections under ~1000 items. Benchmark before parallelizing.

## Design Smells

### OOP Patterns in Rust

Deep inheritance, God objects, virtual dispatch everywhere. Fix: traits for polymorphism, enums for sum types, composition for shared state.

### Monolithic Structs

A struct with 20 fields should be 3-4 focused structs composed together.

### Implicit Invariants Not in Type System

"This should never happen" comments. Fix: encode invariant in the type (newtype, NonZero, slice patterns).

### dyn Trait in Hot Paths

vtable lookup prevents inlining. ~10x slower than generics in tight loops. Fix: use generics with trait bounds for performance-critical code.

### Weasel Word Names (Microsoft M-CONCISE-NAMES)

`BookingService`, `DataManager`, `RequestFactory` — these words carry no meaning. Fix: `Bookings`, `DataStore`, `Requests`. Name types after what they _are_, not their design pattern.

### `#[allow(lint)]` Instead of `#[expect(lint)]` (Rust 1.81+)

`#[allow]` silently suppresses a lint forever — even after you fix the issue. `#[expect]` warns when the suppression is no longer needed, preventing stale suppressions.

### Not Using `let...else` for Early Returns

```rust
// BAD: nested, indented
if let Some(user) = get_user(id) {
    if let Ok(perms) = user.permissions() {
        // main logic deeply indented
    }
}

// GOOD: flat, early exit
let Some(user) = get_user(id) else { return Err(NotFound) };
let Ok(perms) = user.permissions() else { return Err(Forbidden) };
// main logic at top indentation level
```

### Not Destructuring Structs Explicitly

```rust
// BAD: field access hides when struct changes
let x = point.x;
let y = point.y;

// GOOD: compiler errors when fields change
let Point { x, y } = point;
```

## Stdlib Pitfalls — Sharp Edges in `std`

The standard library is excellent but has known footguns. Experienced Rust developers route around these.

### `Path::join` Silently Discards Base on Absolute Argument

```rust
// SURPRISE: not "/usr/local/bin"
let p = Path::new("/usr").join("/local/bin");  // = "/local/bin"
```

If the argument is absolute, the base is dropped entirely. With user-supplied paths this is a footgun. Validate input or check `Path::is_absolute()` first. Lint: `clippy::join_absolute_paths`.

### `slice::split_at` Panics on Out-of-Bounds Index

```rust
// PANIC if mid > len
let (l, r) = arr.split_at(mid);

// SAFE — returns Option<(&[T], &[T])>
match arr.split_at_checked(mid) { Some((l, r)) => ..., None => ... }
```

Use `split_at_checked` (stable) whenever `mid` is computed or user-supplied.

### Indexing `vec[i]` Panics — Use `.get(i)` for Computed Indices

```rust
let elem = arr[3];          // panic if len <= 3
let elem = arr.get(3);      // Option<&T>
```

Lint: `clippy::indexing_slicing`. Prefer slice patterns when the shape is known:

```rust
match users.as_slice() {
    [] => Err(NotFound),
    [only] => Ok(only),
    [first, ..] => Ok(first),
}
```

### `as` for Numeric Conversions — Silent Truncation

```rust
let x: i32 = 1_000_000_000;
let y: i8 = x as i8;        // truncates silently
let y = i8::try_from(x)?;   // proper handling
```

Rule: `From::from` for lossless, `TryFrom` for fallible, `as` only when truncation is intended (and documented). Lints: `clippy::cast_possible_truncation`, `clippy::cast_sign_loss`, `clippy::cast_possible_wrap`.

### `std::collections::LinkedList` — Almost Never the Right Choice

Even its own docs recommend `Vec` or `VecDeque`. Worse cache locality, no O(1) splice/erase/insert in the middle, `remove` is still nightly-only. For genuine intrusive linked lists (kernel-style), use `intrusive-collections` — never `std::collections::LinkedList`.

### `std::thread::spawn` Without `join()` Skips Destructors

```rust
// BAD: cleanup in spawned thread may not run if main exits first
let h = thread::spawn(|| { let _r = Resource; do_work(); });
// forgot h.join() — Resource::drop may never run

// GOOD: scoped threads — auto-join, can borrow non-'static data
thread::scope(|s| { s.spawn(|| do_work_with(&data)); });
```

See [concurrency reference](concurrency.md#threading-prefer-threadscope-over-threadspawn).

### `SystemTime` and `Instant` Are Platform-Dependent

`SystemTime` precision and behavior near the epoch differ per OS. `SystemTime` does not account for leap seconds. For real date arithmetic, use `jiff`, `chrono`, or `time` — never `std::time` for anything beyond `sleep` and elapsed measurement.

### `Path` / `OsStr` Conversion Dance

`path.as_os_str().to_str()` is needed everywhere because not all OS paths are UTF-8. For application code where UTF-8 paths are a fair assumption, use `camino::Utf8Path` / `Utf8PathBuf` to skip the dance. Reserve `std::path::Path` for libraries that must handle every OS.

### `..Default::default()` Hides New Fields

Already covered above — silently uses defaults when the struct grows. Restated here because it is the single most common stdlib-adjacent footgun in app code.

## Clippy Configuration

### Recommended Cargo.toml

```toml
[lints.clippy]
all = "deny"
pedantic = { priority = -1, level = "warn" }
nursery = "warn"
# Cherry-pick restriction lints
unwrap_used = "warn"
expect_used = "warn"
todo = "warn"
```

### CI Command

```bash
CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features
```

### Scope Panic Lints to Non-Test Code

```rust
// Crate-level: warn in production code
#![warn(clippy::unwrap_used)]
#![warn(clippy::expect_used)]

// Test modules: allow unwrap/expect
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests { ... }
```

### Deny Unsafe at Crate Level

For application crates that don't need unsafe, deny it globally and surgically allow on the one module that needs it:

```rust
// Cargo.toml
[lints.rust]
unsafe_code = "deny"

// The one module that needs it:
#[allow(unsafe_code)]
mod secrets;
```
