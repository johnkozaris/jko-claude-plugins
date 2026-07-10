# Trait & API Design

## Generics vs Trait Objects

| Feature | Generics `T: Trait` | Trait Objects `dyn Trait` |
|---|---|---|
| Dispatch | Static (compile-time) | Dynamic (runtime vtable) |
| Performance | Faster (no vtable, inlinable) | Slower (vtable indirection, no inlining) |
| Binary size | Larger (monomorphized) | Smaller (one copy) |
| Heterogeneous collections | No | Yes |

First decide whether a trait is needed. Prefer a concrete type for one owned implementation; introduce a trait for genuine substitution, caller-provided behavior, or a stable external boundary. Once a trait is justified, default to generics for static dispatch and use `dyn Trait` for runtime polymorphism or heterogeneous collections. In microbenchmarks with trivial function bodies, dynamic dispatch can be an order of magnitude slower due to vtable indirection preventing inlining — but in real workloads with substantial function bodies, the difference is often negligible. Profile before optimizing. For the common case where you own all variant types, consider enum-based dispatch.

## Standard Traits to Implement

Implement standard traits deliberately on public types. The orphan rule means downstream crates cannot add them, but each implementation is also an API contract and should exist only when its semantics are sound:

| Trait | When | Notes |
|---|---|---|
| `Debug` | When diagnostics are useful and output does not expose secrets | Required for `assert_eq!`, often useful for logging |
| `Display` | When human-readable representation exists | Required for error types |
| `Clone` | When duplication makes sense | Beware: `Arc<Mutex<T>>` clones share state |
| `Copy` | Cheap, bitwise-copyable, no Drop | Small types only |
| `PartialEq` / `Eq` | When `==` comparisons make sense | `Eq` needed for `HashMap` keys |
| `Hash` | When used as map/set key | **Must be consistent with Eq** |
| `Default` | When a sensible zero/empty value exists | Enables `..Default::default()` |
| `From` / `Into` | For value conversions | Implement `From`, get `Into` free |
| `Send` / `Sync` | Auto-derived | Verify with `static_assertions` crate |

### Critical: Hash/Eq Consistency
If `a == b` then `hash(a)` MUST equal `hash(b)`. Violating this causes silent `HashMap` bugs. If you implement `PartialEq` manually (e.g., ignoring cache fields), implement `Hash` manually to match.

## API Design Properties (Jon Gjengset)

**Unsurprising**: Follow naming conventions. Implement standard traits. Use `From`/`Into` for ergonomic conversions.

**Flexible**: Accept the most general type in parameters (`&str` not `String`, `impl AsRef<Path>` not `&Path`). Return the most specific type.

**Obvious**: Use `#[must_use]` on types/functions callers must handle. Use newtypes over booleans. Documentation examples are executable tests.

**Constrained**: Minimize public API surface. Every public item is a semver commitment. Use `pub(crate)` for internal sharing.

## Sealed Traits

Prevent downstream implementations for traits you control:

```rust
pub trait MyTrait: private::Sealed {
    fn method(&self);
}

mod private {
    pub trait Sealed {}
    impl Sealed for crate::TypeA {}
    impl Sealed for crate::TypeB {}
}
```

Use sealed traits when: you want to add methods with defaults without breaking downstream, or the trait is an internal abstraction.

## Object Safety (Dyn-Compatibility)

A trait is usable as `dyn Trait` only if:
- No methods with generic type parameters
- No methods returning `Self`
- No `async fn` (use `async-trait` crate for `dyn` dispatch)
- All methods have a receiver (`&self`, `&mut self`, `Box<Self>`)

Add `where Self: Sized` to individual methods to exclude them from dyn dispatch while keeping the rest dyn-compatible.

## Trait Upcasting (Rust 1.86+)

`&dyn SubTrait` can now be coerced to `&dyn SuperTrait` automatically — no manual workaround needed.

## Associated Types vs Generic Parameters

- **Associated type**: trait is implemented once per type (`Iterator::Item`)
- **Generic parameter**: trait can be implemented multiple times with different types (`From<T>`)

## Extension Traits

Add methods to foreign types without violating the orphan rule:

```rust
trait StrExt {
    fn is_blank(&self) -> bool;
}
impl StrExt for str {
    fn is_blank(&self) -> bool { self.trim().is_empty() }
}
```

Convention: name `FooExt`. Export in your prelude for convenient glob importing.

## Derive Best Practices

- Derive `Debug` for diagnosable non-secret types.
- Derive `Clone` or `Copy` only when duplication is part of the type's intended semantics, not as a borrow-checker escape hatch.
- Derive `PartialEq`/`Eq` and `Hash` only when equality and key identity are meaningful; `Hash` must match `Eq`.
- Derive `Default` only when a valid, unsurprising default exists.
- Write a manual implementation when field-by-field behavior is semantically wrong (custom equality, clone-with-reset, redacted debug output).
