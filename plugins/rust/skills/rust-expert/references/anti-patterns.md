# Anti-Pattern Catalog

## Blocking — Must Fix Before Merge

### Clone to Satisfy Borrow Checker
Sprinkling `.clone()` to silence the compiler creates independent copies, breaking semantic intent. Fix: restructure ownership, use references, or use Rc/Arc for genuine sharing.

### `unwrap()` / `expect()` in Production Paths
Both panic on None/Err. Library code that panics forces crashes on consumers. Fix: use `?` with `.context()`, `match`, `if let`, or `unwrap_or_else`.

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
Builds fail when rustc introduces new lints. Fix: use `RUSTFLAGS="-D warnings"` in CI only.

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
`BookingService`, `DataManager`, `RequestFactory` — these words carry no meaning. Fix: `Bookings`, `DataStore`, `Requests`. Name types after what they *are*, not their design pattern.

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
cargo clippy --all-targets --all-features -- -D warnings
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
