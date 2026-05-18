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
| Result/Option from outside the program (parse, IO, env, deserialize, network, user input) | **NEVER** | Propagate with `?` and `.context()`. This is what took down Cloudflare on Nov 18, 2025. |
| Invariant the type system can't express (e.g., "I just inserted, so .get() is Some") | `expect("documented reason")` | Assertion-of-invariant is OK; lazy error handling is not. |
| Mutex::lock() poison case | `expect("mutex poisoned")` acceptable | Or use `parking_lot` which doesn't poison. |
| Startup init that MUST succeed | `expect("could not load config")` acceptable | Program can't proceed without it. |
| Tests, benchmarks | `unwrap()` fine | Tests should panic on unexpected state. |
| Doc examples | Use `?` ideally | Per API Guidelines C-QUESTION-MARK. `unwrap` acceptable for brevity if it focuses the example. |

**`expect()` is always better than `unwrap()`** — the message appears in the panic output.

### The Cloudflare outage of November 18, 2025

This is the incident to cite when explaining why `.unwrap()` on external input is dangerous. A ClickHouse permission change caused one of Cloudflare's Bot Management feature files to double in size. When Cloudflare's FL2 proxy loaded the oversized file, it hit a hard-coded 200-feature limit. The code that handled the limit used `.unwrap()` on a `Result`, which panicked:

```
thread fl2_worker_thread panicked: called Result::unwrap() on an Err value
```

The panic cascaded through the proxy, and Cloudflare served 5xx responses globally for hours. The lesson Cloudflare drew in their own postmortem isn't just "stop using unwrap" — it's broader than that. They identified the underlying problem as missing defensive boundaries: input validation, explicit guards, type checks, and staged rollout that would have caught the oversized file before it hit production. The `.unwrap()` was a symptom of unvalidated external input meeting brittle assumptions about its shape.

The full postmortem is at [blog.cloudflare.com/18-november-2025-outage](https://blog.cloudflare.com/18-november-2025-outage/). When you flag a `.unwrap()` on external input in code review, this is the incident to point at.

### When unwrap is acceptable as an invariant assertion

There's a well-known post called [Using unwrap() in Rust is Okay](https://burntsushi.net/unwrap/) that's worth knowing about. The position is that panicking shouldn't be used for error handling, but `unwrap()` as an assertion of an invariant the type system can't express is fine. So the rule isn't "no unwrap ever." It's "prefer `?` for error propagation; restrict `.unwrap()` to tests, benchmarks, and cases where you're asserting an invariant the type system can't capture." And when you do use it for an invariant, prefer `.expect("reason")` over `.unwrap()` — the panic message documents what was assumed, which is useful when something does go wrong.

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
