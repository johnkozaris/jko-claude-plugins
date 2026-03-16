# Trait & API Design

## Generics vs Trait Objects

| Feature | Generics `T: Trait` | Trait Objects `dyn Trait` |
|---|---|---|
| Dispatch | Static (compile-time) | Dynamic (runtime vtable) |
| Performance | Faster (no vtable, inlinable) | Slower (vtable indirection, no inlining) |
| Binary size | Larger (monomorphized) | Smaller (one copy) |
| Heterogeneous collections | No | Yes |

**Default to generics.** Use `dyn Trait` only when you genuinely need runtime polymorphism or heterogeneous collections. In microbenchmarks with trivial function bodies, dynamic dispatch can be an order of magnitude slower due to vtable indirection preventing inlining — but in real workloads with substantial function bodies, the difference is often negligible. Profile before optimizing. For the common case where you own all variant types, consider `enum_dispatch` for enum-based dispatch with trait ergonomics.

## Standard Traits to Implement

Implement these eagerly on all public types — the orphan rule prevents downstream crates from adding them:

| Trait | When | Notes |
|---|---|---|
| `Debug` | Always | Required for `assert_eq!`, logging |
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

- `#[derive(Debug, Clone, PartialEq, Eq, Hash)]` — value types for maps/sets
- `#[derive(Debug, Clone, PartialEq)]` — general structs
- `#[derive(Debug, Default)]` — config/builder types
- When NOT to derive: when field-by-field behavior is semantically wrong (custom equality, clone-with-reset, hash must match custom PartialEq)
