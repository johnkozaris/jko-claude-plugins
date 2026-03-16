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

High-level modules depend on traits, not concrete types.

```rust
// Concrete dependency (violation)
struct NotificationService { sender: EmailSender }

// Trait-based (correct)
struct NotificationService<S: MessageSender> { sender: S }
```

The escalation ladder (Microsoft M-DI-HIERARCHY):
1. **Concrete type** — when only one implementation exists
2. **Generics** — when users provide implementations at compile time
3. **`dyn Trait`** — last resort, wrapped in a custom struct

Rust's trait system IS the DI container. No framework needed.

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

| Feature | Since | Replaces |
|---|---|---|
| `let` chains in `if`/`while` | 1.88 (Edition 2024 required) | Nested `if let` |
| `#[expect(lint)]` | 1.81 | `#[allow(lint)]` (stale suppression detection) |
| Trait upcasting `&dyn Sub` → `&dyn Super` | 1.86 | Manual `as_supertrait()` methods |
| Async closures `async \|\| {}` | 1.85 | `\|\| async {}` workarounds |
| `&raw const` / `&raw mut` | 1.82 | `addr_of!` / `addr_of_mut!` macros |
| Safe `#[target_feature]` functions | 1.86 | `unsafe` target_feature functions |
| `core::error::Error` (no_std) | 1.81 | std-only error traits |
| `cfg` on individual `asm!` statements | 1.93 | Duplicating entire asm blocks per platform |
| `#[repr(u128)]` / `#[repr(i128)]` | 1.89 | Workarounds for large discriminants |

### New APIs That Replace Old Patterns

| New API | Since | Replaces |
|---|---|---|
| `strict_add`, `strict_sub`, `strict_mul` | 1.91 | `checked_add().unwrap()` — always panics on overflow, even in release |
| `array_windows::<N>()` on slices | 1.94 | `.windows(n).map(\|w\| <&[T;N]>::try_from(w).unwrap())` |
| `Result::flatten()` | 1.89 | `.and_then(\|x\| x)` for `Result<Result<T,E>,E>` |
| `std::fmt::from_fn(\|f\| write!(f, ...))` | 1.93 | `struct Wrapper(T); impl Display for Wrapper` pattern |
| `Duration::from_mins()`, `from_hours()` | 1.91 | `Duration::from_secs(n * 60)` |
| `str::ceil_char_boundary()`, `floor_char_boundary()` | 1.91 | Manual UTF-8 boundary scanning |
| `File::lock()`, `lock_shared()`, `try_lock()` | 1.89 | `fd-lock`, `fs2`, `file-lock` crates |
| `RwLockWriteGuard::downgrade()` | 1.92 | Drop write guard + reacquire read guard (race-prone) |
| `VecDeque::pop_front_if()`, `pop_back_if()` | 1.93 | `.front().filter(...).map(\|_\| deque.pop_front())` |
| `<[T]>::as_array::<N>()` | 1.93 | `slice.try_into::<&[T; N]>()` |
| `<[T]>::element_offset()` | 1.94 | Manual pointer arithmetic for index-from-ref |
| `LazyCell` / `LazyLock` (types) | 1.80 | `lazy_static!` / `once_cell` crates |
| `Vec::into_raw_parts()`, `String::into_raw_parts()` | 1.93 | Manual pointer/len/cap extraction for FFI |
| `cargo publish --workspace` | 1.90 | Manual topological publish scripts |
| Cargo config `include` | 1.94 | Duplicated config files across projects |

### New Default Lints to Be Aware Of

| Lint | Level | Since | What it catches |
|---|---|---|---|
| `mismatched_lifetime_syntaxes` | warn | 1.89 | Inconsistent explicit/elided lifetime syntax |
| `dangerous_implicit_autorefs` | deny | 1.89 | Implicit autoref of raw pointer dereferences |
| `dangling_pointers_from_locals` | warn | 1.91 | Raw pointers to local variables being returned |
| `never_type_fallback_flowing_into_unsafe` | deny | 1.92 | Code affected by upcoming `!` type changes |
| `dependency_on_unit_never_type_fallback` | deny | 1.92 | Code depending on `!` → `()` fallback |
| `unused_visibilities` | warn | 1.94 | Visibility on `const _` declarations |

### Build Tooling

- **LLD is default linker on Linux x86_64** (1.90) — no config needed, faster link times
- **`aarch64-pc-windows-msvc` is Tier 1** (1.91) — ARM Windows fully supported
- **`x86_64-apple-darwin` demoted to Tier 2** (1.90) — Intel Mac no longer guaranteed (1.89 was last Tier 1 release)

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
