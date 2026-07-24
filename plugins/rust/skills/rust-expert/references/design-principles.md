# Design Principles for Rust

## SOLID Adapted for Rust

Rust is not OOP but every SOLID principle translates — using traits, modules, and ownership instead of classes and inheritance.

### Single Responsibility (SRP)

Each struct, module, and function has exactly one reason to change.

- Data structs carry no behavior beyond derived traits. Business logic lives in separate `impl` blocks or service types.
- One type per file. One domain per module.
- If a module exceeds ~300 lines covering more than one concept, split it.

**Signal you are violating SRP:** an `impl` block mixes domain logic with I/O, serialization, or formatting.

### Open/Closed (OCP)

Types are open for extension, closed for modification.

- Define behavior as a trait. Add new types via new `impl` blocks.
- Extension traits add methods to foreign types without editing them.
- Blanket implementations (`impl<T: Display> Loggable for T`) extend behavior across all qualifying types.
- Sealed traits prevent extension when you need stability.

### Liskov Substitution (LSP)

Every trait implementor must fully honor the trait's contract.

- Never `panic!` or `unimplemented!()` in a trait implementation for a method callers expect to succeed.
- Use `Option` or `Result` in the trait signature if the operation can fail.
- Push invariants into types (newtypes with validated constructors) so the compiler rejects violations before they reach trait implementations.

### Interface Segregation (ISP)

No type should be forced to implement methods it does not use.

- Prefer many small traits over one god trait.
- Follow std's model: `Read`, `Write`, `Seek`, `BufRead` are separate — not one `FileOperations`.
- Compose at the call site: `fn process<T: Read + Seek>(input: T)`.

### Dependency Inversion (DIP)

High-level modules should depend on stable capability boundaries when substitution is real. A concrete dependency is the right starting point for one owned implementation; introduce a trait when callers provide implementations, multiple implementations exist, or an external I/O seam needs isolation.

```rust
// Concrete dependency (correct while EmailSender is the one owned implementation)
struct NotificationService { sender: EmailSender }

// Trait boundary (use when substitution is a real requirement)
struct NotificationService<S: MessageSender> { sender: S }
```

The escalation ladder (Microsoft M-DI-HIERARCHY):

1. **Concrete type** — when only one implementation exists
2. **Generics** — when users provide implementations at compile time
3. **`dyn Trait`** — last resort, wrapped in a custom struct

Rust's type system makes constructor injection straightforward; traits are one abstraction tool, not a requirement for every dependency.

## DRY — Don't Repeat Yourself

Use each layer in order, escalating only when the previous is insufficient:

1. **Functions** — extract repeated logic
2. **Generics** — eliminate per-type duplication (zero-cost monomorphization)
3. **Trait default methods** — provide standard behavior types inherit
4. **Blanket implementations** — one `impl` covers all qualifying types
5. **Macros** — last resort, when generics and traits cannot express the pattern

Three similar lines of code are better than a premature abstraction.

## Microsoft Pragmatic Rust Rules

### M-STRONG-TYPES — Use the Proper Type Family

Use the strongest type available as early as possible. `PathBuf`/`Path` for file paths, not `String`/`&str`. Domain types (newtypes) for IDs, emails, amounts.

### M-AVOID-WRAPPERS — Hide Smart Pointers from APIs

`Arc`, `Rc`, `Box`, `RefCell` must not appear in public API signatures. They are implementation details. Accept `&T`, `&mut T`, or `T`.

### M-CONCISE-NAMES — No Weasel Words

Avoid `Service`, `Manager`, `Factory`, `Handler`, `Processor`. Use `Bookings` not `BookingService`. Name types after what they ARE.

### M-SERVICES-CLONE — Services Are Clone

Long-lived service types implement `Clone` via internal `Arc<Inner>`. Cloning produces a cheap handle, not a copy.

```rust
#[derive(Clone)]
pub struct Database { inner: Arc<DatabaseInner> }
```

### M-MOCKABLE-SYSCALLS — I/O Is Mockable

Accept mockable I/O as parameters or provide `Library::new_mocked() -> (Self, MockCtrl)`. Never do ad-hoc I/O internally.

### M-INIT-CASCADED — Group Construction Parameters

Types requiring 4+ parameters cascade via semantic helper types:

```rust
// Bad: fn new(bank: &str, customer: &str, currency: &str, amount: u64)
// Good: fn new(account: Account, amount: Currency)
```

### M-LINT-OVERRIDE-EXPECT — Use #[expect] Not #[allow]

`#[expect(lint)]` warns when the suppression becomes stale. `#[allow(lint)]` silently suppresses forever.

## Use Modern Rust

Adopt features from recent stable releases. Check the project's `rust-version` in `Cargo.toml` and only suggest features available at that MSRV. Flag deprecated equivalents during review.

### Language Features

| Feature                                   | Since                        | Replaces                                                            |
| ----------------------------------------- | ---------------------------- | ------------------------------------------------------------------- |
| `let` chains in `if`/`while`              | 1.88 (Edition 2024 required) | Nested `if let`                                                     |
| `if let` guards in `match` arms           | 1.95                         | Nested `match` inside arm bodies — `Some(x) if let Ok(y) = f(x) =>` |
| `#[expect(lint)]`                         | 1.81                         | `#[allow(lint)]` (stale suppression detection)                      |
| Trait upcasting `&dyn Sub` → `&dyn Super` | 1.86                         | Manual `as_supertrait()` methods                                    |
| Async closures `async \|\| {}`            | 1.85                         | `\|\| async {}` workarounds                                         |
| `&raw const` / `&raw mut`                 | 1.82                         | `addr_of!` / `addr_of_mut!` macros                                  |
| Safe `#[target_feature]` functions        | 1.86                         | `unsafe` target_feature functions                                   |
| `core::error::Error` (no_std)             | 1.81                         | std-only error traits                                               |
| `cfg` on individual `asm!` statements     | 1.93                         | Duplicating entire asm blocks per platform                          |
| `#[repr(u128)]` / `#[repr(i128)]`         | 1.89                         | Workarounds for large discriminants                                 |

### New APIs That Replace Old Patterns

| New API                                                                                         | Since | Replaces                                                                               |
| ----------------------------------------------------------------------------------------------- | ----- | -------------------------------------------------------------------------------------- |
| `cfg_select!` macro                                                                             | 1.95  | `cfg-if` crate dependency — compile-time `match` on `cfg` predicates                   |
| `Atomic{Bool,Ptr,Usize,Isize,...}::update` / `try_update`                                       | 1.95  | Hand-rolled `compare_exchange` CAS loops                                               |
| `Vec::push_mut` / `insert_mut`, `VecDeque::push_*_mut` / `insert_mut`, `LinkedList::push_*_mut` | 1.95  | `vec.push(x); vec.last_mut().unwrap()` — returns `&mut T` to inserted element directly |
| `bool: TryFrom<{integer}>`                                                                      | 1.95  | Manual `n == 0/1` checks for int → bool conversion                                     |
| `core::range::{Range, RangeFrom, RangeInclusive, RangeToInclusive}` + iterators | 1.95 (`RangeInclusive`) / 1.96 (`Range`, `RangeFrom`, `RangeToInclusive`) | Legacy `core::ops` ranges. New types implement `IntoIterator` (not `Iterator`) and are `Copy` when bounds are `Copy`. Syntax `0..n` still produces legacy types; staged migration |
| `assert_matches!` / `debug_assert_matches!` | 1.96 | `assert!(matches!(value, pat))` — prints the actual `Debug` repr of the failing value; not in prelude (collides with `mockall` / `claims`) |
| `From<T> for AssertUnwindSafe<T>`, `LazyCell<T,F>`, `LazyLock<T,F>` | 1.96 | `AssertUnwindSafe(x)` tuple-struct calls and manually pre-initialized `LazyLock`s at API boundaries |
| `{integer}::bit_width`, `highest_one`, `lowest_one` | 1.97 | Hand-written leading/trailing-zero arithmetic and zero-sensitive bit-index logic |
| `{integer}::isolate_highest_one`, `isolate_lowest_one` | 1.97 | Hand-written masks that retain the highest/lowest set bit |
| `core::hint::cold_path()`                                                                       | 1.95  | `#[cold]` attribute on entire functions when only one branch is cold                   |
| `Layout::repeat`, `repeat_packed`, `extend_packed`, `dangling_ptr`                              | 1.95  | Manual layout arithmetic for arrays of allocations                                     |
| `MaybeUninit<[T;N]>` ↔ `[MaybeUninit<T>;N]` (`From`/`AsRef`/`AsMut`)                            | 1.95  | Pointer-cast `transmute` between the two layouts                                       |
| `<[T]>::array_windows::<N>()` on slices                                                         | 1.94  | `.windows(n).map(\|w\| <&[T;N]>::try_from(w).unwrap())`                                |
| `Peekable::next_if_map` / `next_if_map_mut`                                                     | 1.94  | `peek().filter(...).map(...)` then `next()` two-step                                   |
| `LazyCell::get` / `get_mut` / `force_mut`, `LazyLock::get` / `get_mut` / `force_mut`            | 1.94  | Inspecting/mutating lazy values without forcing init                                   |
| `<[T]>::element_offset()`                                                                       | 1.94  | Manual pointer arithmetic for index-from-ref                                           |
| `strict_add`, `strict_sub`, `strict_mul`                                                        | 1.91  | `checked_add().unwrap()` — always panics on overflow, even in release                  |
| `Result::flatten()`                                                                             | 1.89  | `.and_then(\|x\| x)` for `Result<Result<T,E>,E>`                                       |
| `std::fmt::from_fn(\|f\| write!(f, ...))`                                                       | 1.93  | `struct Wrapper(T); impl Display for Wrapper` pattern                                  |
| `Duration::from_mins()`, `from_hours()`                                                         | 1.91  | `Duration::from_secs(n * 60)`                                                          |
| `str::ceil_char_boundary()`, `floor_char_boundary()`                                            | 1.91  | Manual UTF-8 boundary scanning                                                         |
| `File::lock()`, `lock_shared()`, `try_lock()`                                                   | 1.89  | `fd-lock`, `fs2`, `file-lock` crates                                                   |
| `RwLockWriteGuard::downgrade()`                                                                 | 1.92  | Drop write guard + reacquire read guard (race-prone)                                   |
| `VecDeque::pop_front_if()`, `pop_back_if()`                                                     | 1.93  | `.front().filter(...).map(\|_\| deque.pop_front())`                                    |
| `<[T]>::as_array::<N>()`                                                                        | 1.93  | `slice.try_into::<&[T; N]>()`                                                          |
| `LazyCell` / `LazyLock` (types)                                                                 | 1.80  | `lazy_static!` / `once_cell` crates                                                    |
| `Vec::into_raw_parts()`, `String::into_raw_parts()`                                             | 1.93  | Manual pointer/len/cap extraction for FFI                                              |
| `cargo publish --workspace`                                                                     | 1.90  | Manual topological publish scripts                                                     |
| Cargo config `include`                                                                          | 1.94  | Duplicated config files across projects                                                |
| TOML 1.1 in `Cargo.toml` (multi-line inline tables, trailing commas)                            | 1.94  | One-line crowded inline tables (raises MSRV)                                           |

### New Default Lints to Be Aware Of

| Lint                                      | Level           | Since | What it catches                                                           |
| ----------------------------------------- | --------------- | ----- | ------------------------------------------------------------------------- |
| `mismatched_lifetime_syntaxes`            | warn            | 1.89  | Inconsistent explicit/elided lifetime syntax                              |
| `dangerous_implicit_autorefs`             | deny            | 1.89  | Implicit autoref of raw pointer dereferences                              |
| `dangling_pointers_from_locals`           | warn            | 1.91  | Raw pointers to local variables being returned                            |
| `never_type_fallback_flowing_into_unsafe` | deny            | 1.92  | Code affected by upcoming `!` type changes                                |
| `dependency_on_unit_never_type_fallback`  | deny            | 1.92  | Code depending on `!` → `()` fallback                                     |
| `function_casts_as_integer`               | warn            | 1.93  | `fn` item cast directly to integer (use `fn` pointer first)               |
| `const_item_interior_mutations`           | warn            | 1.93  | Mutating interior-mutable `const` items (each call site has its own copy) |
| `unused_visibilities`                     | warn            | 1.94  | Visibility on `const _` declarations                                      |
| `ambiguous_glob_imported_traits`          | future-incompat | 1.95  | Glob-imported traits with ambiguous resolution                            |
| `dead_code_pub_in_binary`                 | allow           | 1.97  | Unused `pub` items in binary crates; applications should opt into warning |
| `linker_messages`                         | warn            | 1.97  | Linker output previously hidden; deliberately outside `warnings`          |

### Build Tooling

- **LLVM 22** bundled with Rust 1.95 — newer auto-vectorization and codegen.
- **LLD is default linker on Linux x86_64** (1.90) — no config needed, faster link times
- **`aarch64-pc-windows-msvc` is Tier 1** (1.91) — ARM Windows fully supported
- **Apple ARM platforms promoted to Tier 2** (1.95) — `aarch64-apple-tvos{,-sim}`, `aarch64-apple-watchos{,-sim}`, `aarch64-apple-visionos{,-sim}`
- **`powerpc64-unknown-linux-musl` Tier 2 with host tools** (1.95)
- **`x86_64-apple-darwin` demoted to Tier 2** (1.90) — Intel Mac no longer guaranteed (1.89 was last Tier 1 release)
- **JSON target specs destabilized** (1.95) — now require `-Z unstable-options`. Cargo's new `-Z json-target-spec` flag passes the gate automatically.
- **v0 symbol mangling is the default** (1.97) — update old debuggers/profilers; legacy mangling is now nightly-only.
- **Cargo `build.warnings` is stable** (1.97) — use `CARGO_BUILD_WARNINGS=deny` in CI instead of cache-invalidating `RUSTFLAGS="-D warnings"`.

## Evidence-Backed Critical Rules

### overflow-checks = true in Release (CVE evidence)

CVE-2018-1000810 (std `str::repeat`) — silent integer overflow in release builds. Debug builds panic on overflow, release builds silently wrap. Multiple crates have had similar vulnerabilities. Always enable:

```toml
[profile.release]
overflow-checks = true
```

### 100% of Memory CVEs from unsafe (186-CVE study)

Xu et al. (2021): every memory-safety bug in the full Rust CVE dataset originated in `unsafe` code. None from safe Rust.

### Panics from `.unwrap()` Are Production Incidents

Production services have been taken down by unhandled panics from `.unwrap()` on `Err` values. Root cause pattern: input validation failure + unhandled `Result` propagation. `.unwrap()` in production paths is not a style issue — it is a reliability issue.

### CI Linting Is High-Leverage

Running Clippy in CI catches categories of bugs before they reach production. Servo, Tock OS, the Linux kernel Rust subsystem, and most serious Rust projects enforce `#![deny(clippy::all)]` or stricter.
