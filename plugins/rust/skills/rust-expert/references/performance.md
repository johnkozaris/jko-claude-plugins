# Performance

## Priority Order

1. **Profile first** — use DHAT, samply, or cargo flamegraph to find the actual hotspot
2. **Algorithm/data structure** — this dwarfs all other optimizations
3. **Build config** — LTO, codegen-units, panic=abort for release
4. **Reduce allocations** — with_capacity, reuse with clear(), Cow, clone_from
5. **Stack allocation** — ArrayVec/SmallVec where appropriate (benchmark first)
6. **Iterator chains** — idiomatic, often marginally faster
7. **Alternative allocator** — jemalloc or mimalloc for allocation-heavy code
8. **target-cpu=native** — enable SIMD for non-distributed binaries
9. **PGO** — for mature codebases needing the last 10-15%

## Build Configuration

### Maximum Runtime Speed
```toml
[profile.release]
opt-level = 3
lto = "fat"            # whole-program optimization (10-20%+ gains)
codegen-units = 1      # more cross-function optimization
panic = "abort"        # eliminates unwinding machinery
overflow-checks = true # prevent silent wrapping (CVE-2018-1000810)
```

### Minimum Binary Size
```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

Try `lto = "thin"` first — often comparable to fat with faster compile.

## Heap Allocation Patterns

### Pre-allocate Collections
```rust
// BAD: triggers log2(n) reallocations
let mut v = Vec::new();
for item in items { v.push(item); }

// GOOD: single allocation
let mut v = Vec::with_capacity(items.len());
for item in items { v.push(item); }
```

Same for `String::with_capacity()`.

### Reuse Allocations in Loops
```rust
// BAD: new Vec each iteration
for row in rows {
    let mut buf = Vec::new();
    process(&mut buf, row);
}

// GOOD: reuse allocation
let mut buf = Vec::new();
for row in rows {
    buf.clear();  // retains capacity
    process(&mut buf, row);
}
```

### clone_from Reuses Allocation
```rust
a.clone_from(&b);  // reuses a's buffer if possible
// better than: a = b.clone();  // drops old allocation, creates new
```

## Stack Allocation Options

| Type | Storage | Heap fallback | Use case |
|---|---|---|---|
| `[T; N]` | Stack | None | Fixed size known at compile time |
| `Vec<T>` | Heap | Always | General-purpose dynamic |
| `SmallVec<[T; N]>` | Stack → Heap | Yes | Usually small, occasionally large |
| `ArrayVec<T, N>` | Stack | None | Bounded max size |

**Caveat:** SmallVec is not always faster than Vec — the extra branch on every access can hurt. Always benchmark.

## Iterators vs Loops

Iterators are zero-cost abstractions — the compiler generates the same machine code as hand-written loops. Often marginally faster because:
- Bounds check elimination for sequential access
- Better LLVM vectorizer confidence
- Values kept in registers across chains

Prefer explicit loops when: body is large/multifunctional, complex early-exit logic, or forced conversion hurts readability.

### High-Value Patterns
```rust
// Lazy chains — no intermediate Vec
let result: Vec<_> = data.iter()
    .filter(|x| x.is_valid())
    .map(|x| x.transform())
    .collect();

// zip instead of indexing (bounds-check-free)
for (a, b) in slice_a.iter().zip(slice_b) { ... }

// collect Result — propagate errors from transforms
let parsed: Result<Vec<u32>, _> = strings.iter().map(|s| s.parse()).collect();

// array_windows (Rust 1.94) — compile-time-sized sliding windows, bounds-check-free
for [a, b] in data.array_windows::<2>() {
    // a and b are &T, window size known at compile time
}

// Peekable::next_if (stable) — advance only if predicate matches
let mut iter = tokens.peekable();
if let Some(tok) = iter.next_if(|t| t.is_number()) { ... }
```

### Avoid Collect-Then-Iterate
```rust
// BAD: allocates a Vec just to iterate again
let items: Vec<_> = source.iter().filter(|x| x.valid).collect();
for item in items { process(item); }

// GOOD: chain directly
source.iter().filter(|x| x.valid).for_each(|item| process(item));
```

## Cache Locality

- `Vec<T>` (contiguous) over `LinkedList<T>` (pointer-chased) — always
- `Vec<(K, V)>` sorted + binary search over `HashMap` for small maps (<20 entries)
- Struct fields: frequently accessed first, rarely accessed large fields last

## Benchmarking with Criterion

```toml
[dev-dependencies]
criterion = { version = "0.8", features = ["html_reports"] }

[[bench]]
name = "my_benchmarks"
harness = false
```

**`black_box`** is mandatory — without it, the compiler optimizes away the benchmark body.

## Alternative Allocators

```rust
// In main.rs or lib.rs
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;
```

Can give significant gains for allocation-heavy workloads. Always benchmark.
