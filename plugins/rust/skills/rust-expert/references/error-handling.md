# Error Handling

## The Decision Axis

The real question is not "library vs application" — it is: **does the caller need to match on the specific error type?**

| Answer | Use |
|---|---|
| Yes, caller handles different errors differently | Typed errors: `thiserror` or hand-written |
| No, errors just propagate/log | `anyhow` (or `eyre` for rich reporting) |
| Large multi-crate workspace with context chains | `snafu` |
| User-facing diagnostic output (CLI, compiler) | `miette` + `thiserror` |
| Core ecosystem library, minimal deps | Hand-written `std::error::Error` |

## thiserror — Typed Errors for Libraries

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("failed to read config: {0}")]
    Config(#[from] std::io::Error),

    #[error("invalid input: {reason}")]
    InvalidInput { reason: String },

    #[error("database error")]
    Database(#[source] sqlx::Error),
}
```

Limitation: `#[from]` uses `From` trait — you cannot have two variants from the same source type. Use `#[source]` without `#[from]` and construct manually to disambiguate.

## anyhow — Opaque Errors for Applications

```rust
use anyhow::{Context, Result};

fn load_config(path: &Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .context("failed to read config file")?;   // ALWAYS add context
    let config: Config = toml::from_str(&content)
        .context("failed to parse config TOML")?;
    Ok(config)
}
```

**Critical rule:** Every `?` should have `.context()` or `.with_context(|| ...)`. A bare `?` is wasted diagnostic opportunity.

## Result vs Panic Policy

### When to Use `Result<T, E>`
Always — for any function that can fail due to input, environment, or external systems. This is the default.

### When `panic!` Is Acceptable
- A programming invariant is broken (logically impossible state)
- Tests and benchmarks
- Prototyping and examples
- The caller violated a documented precondition

### `unwrap()` vs `expect()` Policy

| Context | Allowed? | Notes |
|---|---|---|
| Production code | No | Use `?` with context |
| Tests | `unwrap()` fine | Tests should panic on failure |
| Examples / doc tests | Use `?` | Per API Guidelines C-QUESTION-MARK |
| Invariant that type system can't express | `expect("reason")` | Document why it can't fail |

**`expect()` is always better than `unwrap()`** — the message appears in the panic output.

## The `?` Operator

- Works in both sync and async functions
- Requires error types to be compatible via `From`
- **Do not both log AND propagate** — this causes duplicate logging. Either add context and propagate, or handle and log.

## From for Fallible Conversions — A Subtle Bug

```rust
// BAD: From implies infallibility but this can fail
impl From<&str> for UserId {
    fn from(s: &str) -> Self {
        Self(s.parse().unwrap())  // hidden panic!
    }
}

// GOOD: use TryFrom for fallible conversions
impl TryFrom<&str> for UserId {
    type Error = ParseIntError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(Self(s.parse()?))
    }
}
```

## Modern Error Handling APIs

- **`Result::flatten()`** (1.89) — `Result<Result<T, E>, E>` → `Result<T, E>`. Replaces `.and_then(|x| x)`.
- **`std::fmt::from_fn()`** (1.93) — create `Display` impl from a closure. Replaces `struct Wrapper(T); impl Display for Wrapper` boilerplate for error formatting.

## Error Type Design Checklist

- [ ] Error type implements `std::error::Error`, `Debug`, `Display`
- [ ] Error type is `Send + Sync + 'static` (required for anyhow compatibility)
- [ ] Each variant carries enough context to be actionable
- [ ] No stringly-typed errors (`Box<dyn Error>`) in library public APIs
- [ ] `#[non_exhaustive]` on public error enums for future-proofing
- [ ] Re-export error type at crate root: `pub use error::Error`
