# The Rust AI Slop Test

**Critical quality check**: If a senior Rust engineer reviewed this code, would they immediately suspect AI generated it? If yes, that's the problem.

AI-generated Rust code that compiles is not the same as idiomatic Rust code. The compiler catches the broken output, but lets through "compiles but no human would write this" patterns. This reference catalogs the tells.

## The Fingerprints — Rust-Specific

### 1. `.clone()` Sprinkled Everywhere

The single most recognizable AI Rust tell. When AI can't figure out ownership, it clones.

```rust
// AI SLOP: clone to silence the borrow checker
fn process(data: Vec<String>) {
    let copy = data.clone();
    analyze(&copy);
    store(data);
}

// HUMAN: understands ownership flows
fn process(data: Vec<String>) {
    analyze(&data);
    store(data);
}
```

**Detection:** Count `.clone()` calls. If more than 1 per 50 lines in non-test code, investigate each one. Most should be `&T` borrows instead.

### 2. `Arc<Mutex<T>>` as Default Concurrency

AI reaches for `Arc<Mutex<T>>` for ANY shared state. It's the tutorial answer that AI memorized. Real code uses channels, actors, atomics, or just passes ownership.

```rust
// AI SLOP: mutex for everything
let config = Arc::new(Mutex::new(load_config()));
let counter = Arc::new(Mutex::new(0u64));
let cache = Arc::new(Mutex::new(HashMap::new()));

// HUMAN: right tool for each job
let config = Arc::new(ArcSwap::from_pointee(load_config()));
let counter = Arc::new(AtomicU64::new(0));
let cache = Arc::new(DashMap::new());
```

### 3. `.unwrap()` on Everything

AI treats error handling as a formality, not a design concern.

```rust
// AI SLOP: unwrap in production code
let file = File::open(path).unwrap();
let content = serde_json::from_str(&text).unwrap();
let user = users.get(&id).unwrap();

// HUMAN: propagates errors with context
let file = File::open(path).context("failed to open config")?;
let content: Config = serde_json::from_str(&text).context("invalid config JSON")?;
let user = users.get(&id).ok_or_else(|| AppError::NotFound(id))?;
```

### 4. Traits With One Implementation

AI creates abstraction layers that serve no purpose. A trait with exactly one implementor, no mocking need, and no planned extensibility is pure ceremony.

```rust
// AI SLOP: unnecessary abstraction
pub trait UserRepository {
    fn get_user(&self, id: UserId) -> Result<User>;
    fn save_user(&self, user: &User) -> Result<()>;
}
pub struct PostgresUserRepository { pool: PgPool }
impl UserRepository for PostgresUserRepository { ... }

// HUMAN: direct implementation (add trait when you need a second impl)
pub struct UserRepository { pool: PgPool }
impl UserRepository {
    pub fn get_user(&self, id: UserId) -> Result<User> { ... }
    pub fn save_user(&self, user: &User) -> Result<()> { ... }
}
// EXCEPTION: a trait with one production impl + one mock impl IS justified
// when testability is an explicit goal. The tell is when there's no mock either.
```

### 5. Over-Annotated Lifetimes

AI adds explicit lifetime annotations where elision would work, or adds `'static` bounds where borrowed references suffice.

```rust
// AI SLOP: unnecessary lifetime annotations
fn process<'a>(data: &'a str) -> &'a str {
    &data[1..]
}

// HUMAN: elision handles this
fn process(data: &str) -> &str {
    &data[1..]
}
```

### 6. `Box<dyn Error>` in Library Code

AI uses `Box<dyn Error>` because it's the path of least resistance. Library consumers can't match on error variants.

```rust
// AI SLOP: type-erased errors in a library
pub fn parse(input: &str) -> Result<Ast, Box<dyn std::error::Error>> { ... }

// HUMAN: typed errors consumers can match on
pub fn parse(input: &str) -> Result<Ast, ParseError> { ... }
```

### 7. Verbose Comments That Explain "What"

AI comments every function header and restates what the code does. It never explains WHY.

```rust
// AI SLOP: comments that add nothing
/// Creates a new instance of Config
pub fn new() -> Config { ... }

/// Iterates over the items and processes each one
for item in items {
    process(item);
}

// HUMAN: comments explain why, not what
/// Default config matches production settings in us-east-1.
pub fn new() -> Config { ... }

// Process sequentially — parallel execution causes lock contention
// on the shared DB connection (measured: 3x slower with rayon).
for item in items {
    process(item);
}
```

### 8. `async` on Functions That Never Await

AI marks functions async "just in case" or because the caller is async.

```rust
// AI SLOP: async with no await
async fn validate(input: &str) -> bool {
    !input.is_empty() && input.len() < 100
}

// HUMAN: sync function called from async context
fn validate(input: &str) -> bool {
    !input.is_empty() && input.len() < 100
}
```

### 9. Premature Generalization

A function that's called once with one type gets generic parameters.

```rust
// AI SLOP: generic for no reason
fn process_items<T: AsRef<str>, I: IntoIterator<Item = T>>(items: I) -> Vec<String> {
    items.into_iter().map(|s| s.as_ref().to_uppercase()).collect()
}

// HUMAN: concrete types for a single use case
fn uppercase_names(names: &[String]) -> Vec<String> {
    names.iter().map(|s| s.to_uppercase()).collect()
}
```

### 10. `for` + `push` Instead of Iterator Chains

The most visually obvious Rust AI tell. AI generates explicit loops where iterator chains are idiomatic.

```rust
// AI SLOP: explicit loop with push
let mut result = Vec::new();
for item in items {
    if item.active {
        result.push(item.id);
    }
}

// HUMAN: iterator chain
let result: Vec<_> = items.iter().filter(|i| i.active).map(|i| i.id).collect();
```

### 11. `Box<dyn Trait>` Where `impl Trait` Works

AI defaults to dynamic dispatch (heap-allocated vtable) when static dispatch (zero-cost monomorphization) is correct.

```rust
// AI SLOP: dynamic dispatch for no reason
fn make_animal() -> Box<dyn Animal> { Box::new(Dog) }

// HUMAN: static dispatch when type is known
fn make_animal() -> impl Animal { Dog }
```

### 12. Type Alias Instead of Newtype

AI uses `type Foo = Bar;` (transparent alias, zero type safety) when `struct Foo(Bar);` (distinct type, compiler-enforced) is the intent.

```rust
// AI SLOP: alias provides no safety
type UserId = String;  // can pass any String where UserId expected

// HUMAN: newtype prevents mixing
struct UserId(String);  // compiler rejects passing raw String
```

### 13. `format!()` Inside Display Implementation

A subtle tell — AI builds an intermediate String instead of writing directly to the formatter.

```rust
// AI SLOP: unnecessary allocation
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("Name: {}", self.0))
    }
}

// HUMAN: write directly
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Name: {}", self.0)
    }
}
```

### 14. Hallucinated Crate APIs / Wrong Feature Flags

AI confidently invents methods that don't exist on real crates, or adds dependencies without required features.

```toml
# AI SLOP: missing required features
[dependencies]
tokio = "1"                    # needs features = ["full"] or specific features
serde = "1"                    # needs features = ["derive"] for #[derive(Serialize)]
```

### 15. Suppressing Warnings Instead of Fixing Them

AI's primary goal is "make it compile." When the compiler or Clippy complains, AI reaches for `#[allow(...)]` instead of addressing the root cause.

```rust
// AI SLOP: silence the compiler
#[allow(unused_variables)]
let config = load_config();

#[allow(dead_code)]
fn legacy_handler() { ... }

#[allow(clippy::too_many_arguments)]
fn process(a: i32, b: i32, c: i32, d: i32, e: i32, f: i32, g: i32, h: i32) { ... }

// HUMAN: fix the actual problem
let _config = load_config();  // prefix with _ if intentionally unused
// delete legacy_handler if it's dead code
// use a config struct instead of 8 arguments
```

**Common AI suppression patterns:**
- `#[allow(unused)]` on everything rather than removing dead code
- `#[allow(clippy::needless_pass_by_value)]` rather than taking `&T`
- `#[allow(clippy::cast_possible_truncation)]` rather than using `try_into()`
- `#[allow(clippy::module_name_repetitions)]` blanket rather than renaming
- Adding `#[allow(warnings)]` at the crate level — nuclear option that hides all problems
- Changing function signatures to appease the compiler without understanding why it complained

**The tell:** Multiple `#[allow(...)]` attributes clustered in recently-written code. Humans use `#[expect(...)]` (Rust 1.81+) when suppression is intentional, and they write a reason: `#[expect(clippy::cast_possible_truncation, reason = "value bounded by config max 255")]`.

### 16. Monolithic Functions

AI generates 100+ line functions mixing I/O, parsing, validation, business logic, and formatting. Humans split into focused 10-30 line functions.

## Benchmark Data

| Metric | Finding | Source |
|---|---|---|
| Ownership/borrowing errors share of all LLM compile errors | >40% | Academic survey |
| Best model on real Rust tasks (CRUST-Bench) | 48% success (o3) | COLM 2025 |
| Idiomatic C-to-Rust translation success | 52% | arXiv SACTOR |
| LLM-generated code with at least one code smell | 60.9% | Cross-LLM study |
| Code clones with AI assistance | 4x increase | GitClear 2025 |
| Refactored code with AI assistance | 60% decrease | GitClear 2025 |

## The Fingerprints — Cross-Language

### Generic Variable Names
AI defaults to `result`, `data`, `temp`, `item`, `value`, `output`, `response`. Humans use domain names: `invoice_total`, `conn`, `buf`.

### Tutorial-Style Flow
AI code reads like a step-by-step tutorial — heavily commented, each step following logically. Production code is terser, makes assumptions, and is written for someone who knows the domain.

### No Refactoring
AI duplicates rather than extracts. It copies a 20-line block and changes two variables instead of parameterizing. Research data: refactored code dropped from 24% to under 10% of changes with AI assistance (GitClear 2025).

### Over-Engineering Without Justification
Factory patterns for one implementation. Repository layers for three tables. Configuration systems that are never configured. Every abstraction should earn its keep — "What bug does this prevent?"

### Defensive Overreach
Redundant validation at every layer. Try-catch around code that cannot fail. Null checks on values just constructed. `else` blocks after `return`.

## The Litmus Test

After reviewing any code, ask:

1. **Would a senior Rust engineer write this?** Not "does it compile" — would someone with 3+ years of Rust choose these patterns?
2. **Can you remove 30% of the code without changing behavior?** If yes, it's over-engineered.
3. **Does every `.clone()` and `Arc<Mutex<>>` have a reason?** If "because the borrow checker complained" is the reason, it's wrong.
4. **Are the comments useful to someone who already knows Rust?** If they explain `for` loops and iterators, they're noise.
5. **Is there a trait with one implementation?** If yes, why does it exist?
6. **Do the variable names tell you the domain or the data type?** `invoice` vs `data` is the tell.

## What to Do

When you detect AI slop:
- **Don't just flag it** — rewrite it to show what idiomatic Rust looks like
- **Explain the WHY** — "this clone exists because the AI couldn't figure out that `&data` is sufficient"
- **Suggest the `/rust-harden` command** for systematic cleanup
- **Use the `/rust-types` command** to replace `String` parameters with domain types

**IMPORTANT**: Any single fingerprint can appear in human code. It is the **clustering** of multiple fingerprints that signals AI generation. Five `.clone()` calls, generic variable names, a trait with one impl, and verbose comments in the same file — that's the pattern.

**CRITICAL**: AI-generated Rust that compiles is the starting point, not the finish line. The compiler eliminates crashes. Human review eliminates slop.
