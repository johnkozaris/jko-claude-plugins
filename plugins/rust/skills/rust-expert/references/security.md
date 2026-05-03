# Security Pitfalls in Safe Rust

Memory safety ≠ overall security. Safe Rust still allows entire categories of bugs that have caused real incidents and CVEs. This reference catalogs the high-leverage ones.

## Integer Overflow Is Silent in Release

Debug builds panic on overflow; release builds wrap silently (CVE-2018-1000810 in `str::repeat` is the canonical example).

```toml
# Cargo.toml — fix it project-wide
[profile.release]
overflow-checks = true
```

For business arithmetic that must reject overflow even with the default profile:

```rust
price.checked_mul(qty).ok_or(ArithmeticError::Overflow)?
// or, always-panic-even-in-release:
price.strict_mul(qty)  // 1.91+
```

## TOCTOU (Time-of-Check to Time-of-Use)

Separate "check" and "use" calls on filesystem paths can be raced via symlink swap. Source of CVE-2022-21658 in `std::fs::remove_dir_all` itself.

```rust
// BAD: check then use — racy
if !path.is_dir() { return Err(...) }
remove_dir_impl(path);  // path may now be a symlink

// GOOD: open with O_NOFOLLOW + O_DIRECTORY first, then operate on the handle
let h = OpenOptions::new()
    .read(true)
    .custom_flags(O_NOFOLLOW | O_DIRECTORY)
    .open(path)?;
remove_dir_impl(&h);
```

Rule: **open first, check the handle, operate on the handle.** Never re-resolve the path between check and use.

## Constant-Time Comparison for Secrets

`==` on byte slices short-circuits on the first mismatch — leaks length-of-prefix-match via timing.

```rust
// BAD: timing attack reveals correct prefix length
fn verify(stored: &[u8], provided: &[u8]) -> bool { stored == provided }

// GOOD: constant time
use subtle::ConstantTimeEq;
fn verify(stored: &[u8], provided: &[u8]) -> bool {
    stored.ct_eq(provided).unwrap_u8() == 1
}
```

Use `subtle` or domain-specific crates (`password-hash`, `ring`, `argon2`) for any comparison of passwords, MACs, signatures, or tokens.

## Bounded Input — DoS Protection

Accepting unbounded input lets a single request OOM the process.

```rust
// BAD
fn handle(body: &[u8]) -> Result<()> {
    let parsed = decode(body)?;  // arbitrary memory
    ...
}

// GOOD: explicit cap, reject early
const MAX_BODY: usize = 1 << 20; // 1 MiB
if body.len() > MAX_BODY { return Err(Error::TooLarge); }
```

The same applies to:

- `serde` deserialization of recursive types — set depth limits, use `serde_json::Deserializer::from_reader` with size-limited reader
- `Vec::with_capacity(n)` where `n` comes from input — cap `n`
- Compression/zip bombs — limit decompressed bytes via `Read::take`
- gRPC / HTTP body — set framework-level limits (axum `DefaultBodyLimit`, tonic `max_decoding_message_size`)

## `Debug` Leaks Secrets

Auto-derived `Debug` prints every field. Logging an error or struct that contains a password, token, or PII silently exposes it.

```rust
// BAD: password ends up in logs
#[derive(Debug)]
struct User { name: String, password: String }

// GOOD: wrap secrets in a redacting newtype
struct Secret<T>(T);
impl<T> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[REDACTED]")
    }
}

#[derive(Debug)]
struct User { name: String, password: Secret<String> }
```

For production, use the `secrecy` crate (`SecretString`, `SecretBox<T>`) which also zeroizes memory on drop. Same rule applies to `Display` — never derive on types containing secrets.

When implementing `Debug` manually, **destructure** so adding a field becomes a compile error:

```rust
impl std::fmt::Debug for DbUri {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let DbUri { scheme, user, password: _, host, db } = self;  // breaks on new field
        write!(f, "{scheme}://{user}:[REDACTED]@{host}/{db}")
    }
}
```

## Validate at the Deserialization Boundary

`#[derive(Deserialize)]` accepts any structurally valid input. Add validation via `#[serde(try_from = "...")]` so invalid values cannot exist:

```rust
#[derive(Deserialize)]
#[serde(try_from = "String")]
pub struct Password(String);

impl TryFrom<String> for Password {
    type Error = PasswordError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() < 12 { return Err(PasswordError::TooShort) }
        Ok(Password(s))
    }
}
```

Also relevant: `#[serde(deny_unknown_fields)]` to reject typo'd or attacker-injected fields.

## Audit Dependencies

- **`cargo-audit`** — RustSec advisory database. Run in CI: `cargo audit --deny warnings`.
- **`cargo-deny`** — license, advisory, source, and ban policies. More configurable than `cargo audit` alone.
- **`cargo-geiger`** — counts `unsafe` blocks in your dependency tree. Useful for "should I trust this crate?" decisions.
- **`cargo-vet`** (Mozilla) — track audited versions of dependencies across an org.

For supply-chain hygiene also pin via `Cargo.lock` for binaries, review `build.rs` scripts in new dependencies (they run during compilation with full FS/network access), and prefer crates with low `cargo geiger` counts for security-sensitive code paths.

## Path Traversal

Attacker-supplied path segments can escape an intended root via `..`:

```rust
// BAD: attacker sends "../../etc/passwd"
let p = base.join(user_input);

// GOOD: canonicalize, then verify still under base
let resolved = base.join(user_input).canonicalize()?;
if !resolved.starts_with(base.canonicalize()?) {
    return Err(Error::OutsideRoot);
}
```

Also remember: `Path::join` with an _absolute_ user-supplied segment silently discards the base. Reject absolute paths from input before joining.

## `unsafe` Code — Soundness Gates

Per Xu et al. (2021), 100% of memory-safety CVEs in the Rust ecosystem trace to `unsafe` blocks. For any crate that contains `unsafe`:

1. **Run Miri in CI**: `cargo +nightly miri test` — catches UB the type system can't.
2. **Every `unsafe` block has a `// SAFETY:` comment** stating the invariants relied on.
3. **Minimize the unsafe surface** — narrow blocks, never expose `unsafe fn` publicly without genuine reason.
4. **Use `#![deny(unsafe_code)]`** at the crate level for app crates that don't need it; surgically `#[allow]` the one module that does.
5. **Fuzz the safe API** of any unsafe-using module with `cargo-fuzz` or `cargo-bolero`.

## Recommended CI Lint Set for Security-Sensitive Crates

```rust
// Cargo.toml
[lints.clippy]
indexing_slicing       = "deny"
unwrap_used            = "warn"
expect_used            = "warn"
panic                  = "warn"
checked_conversions    = "deny"
cast_possible_truncation = "deny"
cast_sign_loss         = "deny"
cast_possible_wrap     = "deny"
cast_precision_loss    = "deny"
arithmetic_side_effects = "deny"
unchecked_duration_subtraction = "deny"
join_absolute_paths    = "deny"
fallible_impl_from     = "deny"
serde_api_misuse       = "deny"
uninit_vec             = "deny"

[lints.rust]
unsafe_code = "deny"   # then surgically allow per-module if needed
```

`cargo clippy --all-targets -- -D warnings` in CI catches all of these at compile time.

## Further Reading

- corrode.dev: ["Pitfalls of Safe Rust"](https://corrode.dev/blog/pitfalls-of-safe-rust/)
- corrode.dev: ["Patterns for Defensive Programming in Rust"](https://corrode.dev/blog/defensive-programming/)
- High Assurance Rust (free book): https://highassurance.rs/
- Rust Security Advisories: https://rustsec.org/
