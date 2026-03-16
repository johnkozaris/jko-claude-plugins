# Unsafe Code

## The Five Unsafe Superpowers

`unsafe` grants exactly five capabilities:
1. Dereference a raw pointer
2. Call an unsafe function or method
3. Access or modify a mutable static variable
4. Implement an unsafe trait
5. Access fields of a union

`unsafe` does NOT disable the borrow checker. References inside unsafe blocks are still checked.

## The Tootsie Pop Model (Niko Matsakis)

The unsafe boundary is the **entire module**, not just the `unsafe {}` block. Even safe code within the same module can break invariants that unsafe code depends on. Use Rust's privacy system to strictly limit which code can access raw fields.

## Mandatory: SAFETY Comments

Every `unsafe` block must have a `// SAFETY:` comment explaining what invariant is being upheld:

```rust
// SAFETY: index was bounds-checked by the caller via the length assertion above.
unsafe { slice.get_unchecked(index) }
```

Clippy warns if this is missing. This is not optional — it is enforced in major projects including the Rust compiler and Linux kernel.

## When Unsafe Is Acceptable

- FFI (Foreign Function Interface) with C libraries
- Low-level systems programming (implementing Vec, Mutex, allocators)
- Bypassing conservative compiler checks when bounds are proven by other logic
- Directly interfacing with OS APIs or hardware

## When Unsafe Is Avoidable

~25% of unsafe blocks in real-world Rust could be eliminated. Common unnecessary uses:
- Sharing mutable state across threads (use Mutex, Arc, RwLock, atomics)
- Nullable/error-prone patterns (use Option/Result)
- Performance assumptions without profiling evidence

## Encapsulation Rules

1. **Wrap unsafe in safe public interfaces.** Callers should never need to write `unsafe`.
2. **Minimize scope** of unsafe blocks — cover only the single operation that requires it.
3. **Never expose `unsafe fn` in public APIs** without a documented, genuine reason.
4. **Document ownership** — who owns memory, who deallocates it.

```rust
// BAD: exposes unsafe in public API
pub unsafe fn read_memory(ptr: *const u8) -> u8 { *ptr }

// GOOD: safe wrapper handles invariants internally
pub fn read_at_offset(slice: &[u8], offset: usize) -> Option<u8> {
    if offset < slice.len() {
        // SAFETY: bounds checked above
        Some(unsafe { *slice.get_unchecked(offset) })
    } else {
        None
    }
}
```

## Common UB Patterns

| Pattern | Why UB |
|---|---|
| Two `&mut T` to same memory | Breaks aliasing — compiler assumes `&mut` is unique |
| `static mut` from multiple threads | Data race |
| Reading uninitialized memory | Reads `undef`, violates type invariants |
| Wrong `extern "C"` ABI | Calling convention mismatch |
| `transmute` between incompatible types | Produces invalid values |
| Panic inside unsafe leaving invariants broken | Panic safety violation |

## Miri — Dynamic UB Detection

```bash
rustup +nightly component add miri
cargo +nightly miri test
```

Miri detects: dangling pointers, misaligned access, data races, Stacked Borrows violations, invalid enum discriminants, overlapping `copy_nonoverlapping`. Run on every codebase with unsafe.

**Limitation:** Miri only finds UB in code paths that actually execute. Absence of errors does not prove soundness.

## Edition 2024 Changes

- `unsafe extern` blocks required for extern declarations
- `unsafe` on `export_name`, `link_section`, `no_mangle` attributes
- `unsafe_op_in_unsafe_fn` is denied by default
- `static mut` references now denied — use raw pointers
- `std::env::set_var` / `remove_var` now unsafe (Rust 1.83+, all editions)
