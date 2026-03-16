---
description: Strengthen the type system — replace primitive obsession with newtypes, booleans with enums, stringly-typed APIs with domain types. Make illegal states unrepresentable.
allowed-tools:
  - Read
  - Edit
  - Grep
  - Glob
  - Bash
argument-hint: "<target>"
---

# Rust Types — Strengthen the Domain Model

Push runtime checks into the type system. Every runtime `assert!`, `if !valid`, or "this should never happen" comment is a type waiting to be born.

## Preparation

1. Read the target code and identify the domain concepts.
2. Look for primitive types carrying domain meaning (`String` for emails, `u64` for IDs, `bool` for states).

## Strengthening Steps

### Step 1: Find Primitive Obsession

Run in the shell:
```bash
rg --type rust 'fn \w+\(.*: (String|&str|u32|u64|i32|i64|bool|f64)' src/ -n | head -30
```

For each function taking raw primitives that represent domain concepts:
- `String` for an email → `struct Email(String)` with validation in constructor
- `u64` for an ID → `struct UserId(u64)`, `struct OrderId(u64)` (distinct types!)
- `f64` for money → `struct Currency { amount: i64, code: CurrencyCode }` (never float for money)

### Step 2: Replace Booleans with Enums

Run in the shell:
```bash
rg --type rust 'fn \w+\(.*: bool' src/ -n
```

For each boolean parameter:
```rust
// Before: process(data, true, false)  — what do these mean?
// After:
enum Direction { Forward, Backward }
enum OutputMode { Raw, Formatted }
fn process(data: &Data, dir: Direction, mode: OutputMode)
```

### Step 3: Make Illegal States Unrepresentable

Look for:
- Structs with fields that can be in inconsistent states → use an enum
- `Option<A>` + `Option<B>` where exactly one must be Some → use an enum
- State transitions modeled with mutable fields → use the typestate pattern

```rust
// Before: can be in invalid state (title set but no body)
struct Post { title: Option<String>, body: Option<String>, published: bool }

// After: invalid states are compile errors
enum Post {
    Draft { title: String },
    Review { title: String, body: String },
    Published { title: String, body: String, url: Url },
}
```

### Step 4: Validated Constructors

For every newtype, ensure construction goes through validation:

```rust
pub struct Email(String);

impl Email {
    pub fn new(s: impl Into<String>) -> Result<Self, ValidationError> {
        let s = s.into();
        if !s.contains('@') { return Err(ValidationError::InvalidEmail); }
        Ok(Self(s))
    }
}
```

Keep the inner field private. Force all construction through the validated path.

### Step 5: Add #[must_use] and #[non_exhaustive]

- `#[must_use]` on functions returning `Result` or computed values that callers must handle
- `#[non_exhaustive]` on public enums that may gain variants in the future

## The Litmus Test

After this pass, ask: "Can a user of this API create an invalid state without writing `unsafe`?" If yes, the types are not strong enough.
