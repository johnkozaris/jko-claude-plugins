# Testing

## Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_return_error_when_input_is_empty() {
        assert!(parse("").is_err());
    }
}
```

- Co-locate with source in `#[cfg(test)]` — excluded from release builds
- Can access private functions via `use super::*`
- Descriptive names: `should_X_when_Y` or `given_X_when_Y_expect_Z`

## Integration Tests

Live in top-level `tests/` directory. Each file is a separate crate — can only access public API.

```
tests/
  common/mod.rs     ← shared helpers (not a test binary)
  api_tests.rs
  user_tests.rs
```

- `cargo test --test api_tests` — run one file
- `cargo test -- --test-threads=1` — serial execution for shared state
- For CLI testing: use `std::process::Command` to spawn binary

## Property-Based Testing (proptest)

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn reverse_is_involution(v in proptest::collection::vec(0i32..100, 0..50)) {
        let mut reversed = v.clone();
        reversed.reverse();
        reversed.reverse();
        prop_assert_eq!(v, reversed);
    }
}
```

- Use `prop_assert!` / `prop_assert_eq!` — not bare `assert!`
- Combine with targeted unit tests for specific edge values
- `proptest` > `quickcheck` for new projects (explicit strategies, more flexible)

## Mocking (mockall)

```rust
#[cfg_attr(test, mockall::automock)]
trait EmailSender {
    fn send(&self, to: &str, body: &str) -> Result<(), Error>;
}

#[test]
fn sends_welcome_email() {
    let mut mock = MockEmailSender::new();
    mock.expect_send()
        .with(eq("user@test.com"), predicate::str::contains("Welcome"))
        .times(1)
        .returning(|_, _| Ok(()));
    register_user(&mock, "user@test.com");
}
```

- Mock at boundaries (I/O, network, DB) — not internal logic
- Don't mock in integration tests — use real dependencies
- Constructor injection: `Box<dyn Trait>` or generic `T: Trait`

## Fixtures (rstest)

```rust
use rstest::rstest;

#[rstest]
#[case(0, 0)]
#[case(1, 1)]
#[case(5, 5)]
fn test_fibonacci(#[case] input: u32, #[case] expected: u32) {
    assert_eq!(fibonacci(input), expected);
}
```

- `#[fixture]` for shared setup
- `#[once]` for expensive one-time initialization
- Implement `Drop` on fixture structs for teardown

## Doc Tests

```rust
/// Adds two numbers.
///
/// # Examples
/// ```
/// assert_eq!(my_crate::add(2, 3), 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 { a + b }
```

- Every public item should have at least one doc example
- Use `?` not `unwrap()` in examples (API Guideline C-QUESTION-MARK)
- Hidden setup lines with `# ` prefix (compiled but not shown)
- Test README as doctest: `#[doc = include_str!("../README.md")]`
- `cargo test --doc` to run doc tests only

## Snapshot Testing (insta)

```rust
use insta::assert_json_snapshot;

#[test]
fn test_api_response() {
    let response = get_response();
    assert_json_snapshot!(response, {
        ".created_at" => "[timestamp]",
        ".id" => "[id]"
    });
}
```

- `cargo insta review` to accept/reject new snapshots
- Set `INSTA_UPDATE=no` in CI to fail on new snapshots
- Prefer JSON/YAML snapshots over Debug for stability

## Test Runner: cargo-nextest

Up to 3x faster. Each test in its own process. Rich filtering. JUnit output for CI.

```bash
cargo nextest run
cargo nextest run -E 'test(user)'  # filter by name
```

Doc tests not supported — run separately with `cargo test --doc`.

## CI Mandatory Checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo test --doc
cargo audit
```
