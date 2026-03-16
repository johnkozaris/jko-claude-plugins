# Documentation

## Doc Comment Rules

- `///` — documents the item following it (functions, structs, enums, traits)
- `//!` — documents the parent item (crate root, module)
- First line: single complete sentence, third-person present, ends with period
  - "Returns the length of the string." not "Return the length"
- Markdown is the format (CommonMark)

## Required Sections

| Section | When Required |
|---|---|
| `# Examples` | Always, for every public item. Always plural. |
| `# Panics` | Whenever the function can panic |
| `# Errors` | Whenever a function returns `Result` |
| `# Safety` | Required on every `unsafe fn` |

## API Guidelines Checklist

| Code | Rule |
|---|---|
| C-CRATE-DOC | Crate-level docs in `lib.rs` with `//!`, thorough, includes examples |
| C-EXAMPLE | All public items have at least one rustdoc example |
| C-QUESTION-MARK | Examples use `?`, never `unwrap()` or `try!` |
| C-FAILURE | Function docs include error, panic, safety considerations |
| C-LINK | Prose contains hyperlinks to relevant types |
| C-METADATA | Cargo.toml includes description, license, repository, keywords |

## Intra-Doc Links

```rust
/// See [`MyStruct`] or [`some_function`] for details.
/// Also see [`std::collections::HashMap`].
```

Enable the lint: `#![warn(rustdoc::broken_intra_doc_links)]`

## Doc Tests

Doc examples are compiled and run by `cargo test`. They:
- Auto-wrap in `fn main() {}`
- Compile against only the public API
- Support annotations: `no_run`, `should_panic`, `compile_fail`, `ignore`

```rust
/// # Examples
/// ```
/// # use my_crate::Foo;  // hidden setup line
/// let foo = Foo::new();
/// assert_eq!(foo.value(), 42);
/// ```
```

### Test README as Doctest
```rust
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
```

## What NOT to Document

- Don't repeat what the type signature communicates
- Don't expose implementation details as first-class entries
- Use `#[doc(hidden)]` on impl blocks users can't meaningfully use

## Enforcing Documentation

```rust
#![warn(missing_docs)]              // warn on undocumented public items
#![warn(rustdoc::broken_intra_doc_links)]  // catch broken links
```

Escalate to `#![deny(missing_docs)]` once coverage is solid.

## Tooling

| Tool | Purpose |
|---|---|
| `cargo doc --open` | Build and view docs locally |
| `cargo test --doc` | Run doc tests only |
| `cargo rdme --check` | CI check that README matches lib.rs docs |
