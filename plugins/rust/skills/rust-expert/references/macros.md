# Macros

## Decision Flowchart

```
Need code reuse?
├── Fixed types, fixed arity → Function
├── Multiple types, same logic → Generic Function / Trait
└── Neither works?
    ├── Variable arity or custom syntax → macro_rules!
    ├── Simple pattern, small input → macro_rules! with repetition
    ├── Complex parsing, large input → Proc macro
    ├── Auto-implement a trait → #[derive(...)]
    └── Transform annotated items → Attribute proc macro
```

**Default to functions, then generics. Macros are a last resort.**

## When Macros Are Anti-Patterns

| Anti-Pattern | Fix |
|---|---|
| Macro for trivial logic | Plain function |
| Hidden `return`/`continue` in macro | Return `Result`/`Option`, caller uses `?` |
| Multiple evaluation of arguments | Assign to temp var in macro body |
| Non-standard confusing syntax | Match normal Rust or look obviously different |
| Macro wrapping that hides references | Explicit `&` at call site |

## macro_rules! Best Practices

- **Fragment specifiers:** Only `tt`, `ident`, and `lifetime` remain transparent to downstream macros. All others (`expr`, `ty`, `pat`) become opaque.
- **Trailing commas:** Use `$(, $x:expr)* $(,)?` for natural comma-separated lists.
- **Internal rules:** Use `@internal_rule_name` arms for helper logic without separate macros.
- **TT munchers are O(n^2)** — prefer `*`/`+` repetition. TT muncher + push-down accumulator is doubly quadratic.

## Procedural Macro Best Practices

### Crate Structure
Proc macros must be in their own crate with `proc-macro = true`. Convention: `<crate>-derive` or `<crate>-macros`.

### The Standard Stack: syn + quote + proc-macro2

**dtolnay's primary rule:** Write all internal logic against `proc_macro2`, not `proc_macro`. The one exception is entry point signatures.

```rust
#[proc_macro_derive(MyTrait)]
pub fn my_trait_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);
    // logic using proc_macro2 / quote internally
    proc_macro::TokenStream::from(expanded)
}
```

### Error Reporting
```rust
// GOOD: preserves source spans, actionable error
syn::Error::new(span, "expected a struct, not an enum").to_compile_error()

// BAD: no span info, poor UX
panic!("expected a struct")
```

### Absolute Paths in Generated Code
Generated code runs in the user's crate. Always qualify: `::std::option::Option`, `::core::result::Result`.

### Testing
- `trybuild` — test that macros emit correct compile-time errors
- `cargo expand` — inspect expanded output during development

## Derive Macro Attributes

```rust
#[proc_macro_derive(Builder, attributes(builder))]
pub fn builder_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream { ... }
```

- Namespace with your crate/trait name: `#[serde(...)]`, `#[builder(...)]`
- Document all attributes in the derive macro's rustdoc
- Use `darling` crate for parsing structured attributes into typed structs

## Future Direction

As of the 2025H1 project goals, there is active work to extend `macro_rules!` to define attribute and derive macros declaratively — without writing a proc macro crate. Check the Rust blog for current status.
