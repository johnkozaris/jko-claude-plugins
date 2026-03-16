# Type System Patterns

## Core Philosophy

- **Parse, don't validate.** Convert raw inputs into richer types that carry their validity in the type itself. Validate once at the boundary — never downstream.
- **Make illegal states unrepresentable.** Don't add `is_valid()` methods people forget to call. Make invalid construction impossible.
- **Push errors to compile time.** Every runtime check replaceable with a type constraint is a bug category eliminated permanently.

## Newtype Pattern (Zero-Cost)

Wrap primitives to create distinct types that the compiler distinguishes:

```rust
struct UserId(u64);
struct ProductId(u64);

fn get_user(id: UserId) -> User { ... }
// get_user(ProductId(42))  // compile error!
```

**Best practices:**
- Keep inner field private by default
- Implement standard traits (`Debug`, `Display`, `From`, `Deref` where appropriate)
- Use `derive_more` to reduce boilerplate
- **Caveat:** Serde's `#[serde(transparent)]` bypasses validation during deserialization

### Validated Newtypes
```rust
pub struct Email(String);

impl Email {
    pub fn new(s: impl Into<String>) -> Result<Self, InvalidEmail> {
        let s = s.into();
        if s.contains('@') && s.contains('.') {
            Ok(Self(s))
        } else {
            Err(InvalidEmail)
        }
    }
}
```

## Typestate Pattern (Zero-Cost State Machines)

Encode state transitions into the type system. Invalid transitions become compile errors:

```rust
struct Draft;
struct Published;

struct Article<State> {
    title: String,
    body: String,
    _state: std::marker::PhantomData<State>,
}

impl Article<Draft> {
    fn publish(self) -> Article<Published> {
        Article { title: self.title, body: self.body, _state: PhantomData }
    }
}

impl Article<Published> {
    fn url(&self) -> String { ... }
}

// article.url()  // compile error on Draft!
// After publish: article.publish().url()  // works
```

`PhantomData` is zero-sized — completely compiled away. Same machine code, zero runtime overhead.

## Builder Pattern

For structs with many fields, especially optional ones:

```rust
pub struct ServerConfig {
    host: String,
    port: u16,
    max_connections: usize,
}

pub struct ServerConfigBuilder {
    host: String,
    port: Option<u16>,
    max_connections: Option<usize>,
}

impl ServerConfigBuilder {
    pub fn new(host: impl Into<String>) -> Self {
        Self { host: host.into(), port: None, max_connections: None }
    }
    pub fn port(mut self, port: u16) -> Self { self.port = Some(port); self }
    pub fn max_connections(mut self, n: usize) -> Self { self.max_connections = Some(n); self }
    pub fn build(self) -> ServerConfig {
        ServerConfig {
            host: self.host,
            port: self.port.unwrap_or(8080),
            max_connections: self.max_connections.unwrap_or(100),
        }
    }
}
```

For compile-time validation of required fields, combine builder with typestate.

## Boolean Parameters Anti-Pattern

```rust
// BAD: unreadable at call site
process(data, true, false, true);

// GOOD: self-documenting, type-safe
enum Direction { Forward, Backward }
enum OutputMode { Raw, Formatted }
process(data, Direction::Forward, OutputMode::Raw);
```

## `..Default::default()` Anti-Pattern

```rust
// BAD: silently uses wrong defaults when new fields are added
let config = Config {
    timeout: Duration::from_secs(30),
    ..Default::default()
};

// GOOD: compiler errors when new fields are added
let config = Config {
    timeout: Duration::from_secs(30),
    retries: 3,
    verbose: false,
};
```

## Defensive Construction: Private Fields + `_private: ()`

```rust
pub struct Config {
    pub timeout_ms: u64,
    pub retries: u32,
    _private: (),  // external code cannot construct directly
}

impl Config {
    pub fn new(timeout_ms: u64, retries: u32) -> Self {
        Self { timeout_ms, retries, _private: () }
    }
}
```

## Slice Patterns Over Index Checks

```rust
// BAD: runtime panic possible
if !users.is_empty() {
    let user = &users[0];
}

// GOOD: compiler-verified exhaustiveness
match users.as_slice() {
    [] => Err(NotFound),
    [user] => Ok(user),
    [first, ..] => Ok(first),
}
```

## Enums vs Trait Objects

| | Enum | `dyn Trait` |
|---|---|---|
| Variants known at compile time | Yes | No |
| Heap allocation | No | Yes (Box) |
| External extensibility | No | Yes |
| Performance | Faster (inlinable) | Slower (vtable, no inlining) |
| `match` exhaustiveness | Yes | No |

Use enums for closed sets (parsers, AST nodes, error types). Use `dyn Trait` for open sets (plugins, middleware).

## `#[must_use]` — Prevent Ignored Values

```rust
#[must_use = "this Result may contain an error that should be handled"]
pub fn save(&self) -> Result<(), SaveError> { ... }
```

Mark functions and types that callers must handle. The compiler warns on ignored values.

## `#[non_exhaustive]` — Future-Proof Enums and Structs

```rust
#[non_exhaustive]
pub enum Error {
    NotFound,
    Timeout,
}
```

Downstream code must include a wildcard arm in match, allowing you to add variants without breaking semver.

## Error → Design Question Reframing

See the reframing table in SKILL.md. When a compiler error appears, always trace UP to ask what design decision caused it before applying a mechanical fix.

## Services Are Clone (Microsoft M-SERVICES-CLONE)

Long-lived service types should implement `Clone` cheaply via internal `Arc`:

```rust
#[derive(Clone)]
pub struct Database {
    inner: Arc<DatabaseInner>,
}

struct DatabaseInner {
    pool: Pool,
    config: Config,
}
```

`Clone` creates a new handle, not a full copy. This pattern enables passing services to spawned tasks without explicit Arc wrapping at call sites.
