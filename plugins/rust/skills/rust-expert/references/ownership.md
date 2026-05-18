# Ownership, Borrowing & Lifetimes

## The Golden Rules

- Default to borrowing (`&T`, `&mut T`). Move to owned only when the callee must store the value.
- Never use `&String` or `&Vec<T>` as function parameters. Use `&str` and `&[T]`.
- Before adding a lifetime annotation, check if restructuring or using owned types eliminates the need.
- Before calling `.clone()`, ask: does the callee need ownership or just access?

## Parameter Types

| Caller has | Function needs read | Function needs ownership     |
| ---------- | ------------------- | ---------------------------- |
| `String`   | `fn f(s: &str)`     | `fn f(s: String)`            |
| `&str`     | `fn f(s: &str)`     | `fn f(s: impl Into<String>)` |
| `Vec<T>`   | `fn f(v: &[T])`     | `fn f(v: Vec<T>)`            |
| `&[T]`     | `fn f(v: &[T])`     | `fn f(v: impl Into<Vec<T>>)` |

## Clone Anti-Patterns

### Clone to Silence Borrow Checker (Top Anti-Pattern)

```rust
// BAD: clone hides an ownership bug
let copy = data.clone();
process(&copy);
store(data);

// GOOD: borrow first, then move
process(&data);
store(data);
```

### Prefer clone_from Over clone

```rust
// BAD: drops old allocation, creates new one
a = b.clone();

// GOOD: reuses a's allocation if possible
a.clone_from(&b);
```

### Use mem::take / mem::replace Instead of Clone

```rust
// When extracting an owned value from a mutable reference:
let val = std::mem::take(&mut opt_field);       // replaces with Default
let old = std::mem::replace(&mut field, new);   // swaps in new value
```

## Cow — Conditional Allocation

Use `Cow<'_, T>` when a function usually returns borrowed data but occasionally must allocate:

```rust
use std::borrow::Cow;

fn sanitize(input: &str) -> Cow<str> {
    if input.contains('<') {
        Cow::Owned(input.replace('<', "&lt;"))  // allocates only when needed
    } else {
        Cow::Borrowed(input)                     // zero allocation
    }
}
```

**Use Cow when:** function conditionally allocates.
**Don't use Cow when:** data is always modified (return `String`) or never modified (return `&str`).

## Lifetime Rules

### Elision Covers 95% of Cases

The compiler's three elision rules handle most cases automatically:

1. Each input reference gets its own lifetime
2. Single input lifetime → all outputs inherit it
3. Methods: return values inherit `&self`'s lifetime

You only write explicit annotations when the compiler cannot determine which input a return reference borrows from.

### Lifetime Contagion Warning

Once a lifetime appears on a struct, it propagates to everything containing that struct. This cascades through the codebase. **When in doubt, own your data.** The ergonomic cost of lifetime propagation usually exceeds the memory cost of cloning.

### Never Write `&'a mut self` on Methods

If a struct is generic over `'a`, writing `&'a mut self` means the mutable borrow lasts for the entire lifetime of the struct. The struct becomes effectively frozen after the call.

```rust
// BAD: borrows self for its entire lifetime 'a
impl<'a> Parser<'a> {
    fn next(&'a mut self) -> Option<Token> { ... }
}

// GOOD: let the borrow checker infer
impl<'a> Parser<'a> {
    fn next(&mut self) -> Option<Token> { ... }
}
```

### Common Lifetime Misconceptions

- `T: 'static` does NOT mean T lives forever — it means T contains no non-static borrows (all owned types are `'static`)
- Lifetimes are not chosen by the programmer at call sites — the compiler infers them
- A longer lifetime is not "safer" — it's more restrictive

## References in Structs

Use references in structs only for short-lived views over existing data (parsers, cursors). For long-lived structs, use owned types. If the struct must be returned from a function, stored in a collection, or sent to a thread — own the data.

```rust
// SHORT-LIVED: fine for a parser scanning a buffer
struct Lexer<'a> { input: &'a str, pos: usize }

// LONG-LIVED: own the data
struct Config { name: String, path: PathBuf }
```

## Modern Ownership APIs

- **`str::ceil_char_boundary()` / `floor_char_boundary()`** (1.91) — safe UTF-8 boundary detection. Replaces manual scanning loops. Use before slicing strings to avoid panics on multi-byte characters.
- **`Duration::from_mins()` / `from_hours()`** (1.91) — replaces `Duration::from_secs(n * 60)`.
- **`<[T]>::as_array::<N>()`** (1.93) — safe slice-to-fixed-array conversion. Returns `None` if length doesn't match. Replaces `slice.try_into::<&[T; N]>()`.
- **`Vec::push_mut` / `Vec::insert_mut`** (1.95) — push or insert and get a `&mut T` to the inserted element in one call. Replaces:

  ```rust
  // BEFORE
  vec.push(value);
  let r = vec.last_mut().unwrap();   // unwrap is provably-safe but noisy

  // AFTER
  let r = vec.push_mut(value);
  ```

  Same for `VecDeque::push_front_mut` / `push_back_mut` / `insert_mut` and `LinkedList::push_front_mut` / `push_back_mut`.

- **`bool: TryFrom<{integer}>`** (1.95) — `bool::try_from(n)` returns `Err` for any value other than 0 or 1. Replaces hand-rolled `match n { 0 => false, 1 => true, _ => return Err(..) }`.

## Drop order — a real footgun

Rust guarantees the order in which values get dropped ([RFC 1857](https://rust-lang.github.io/rfcs/1857-stabilize-drop-order.html)), but the rules are asymmetric in a way that catches people. The fields inside a struct drop in the order they're declared — first field drops first, last field drops last. But local variables in a function body drop in *reverse* order — the last `let` drops first, the first `let` drops last. The same backwards-by-default rule applies to function parameters and pattern bindings.

This asymmetry is where the bugs come from. When a later field references something owned by an earlier field, the earlier field drops first and the later field's destructor finds a dangling reference. Here's the canonical case:

```rust
// FIELDS drop in declaration order — `device` drops FIRST
struct Renderer {
    device: VulkanDevice,    // dropped first → driver crash
    pipeline: VulkanPipeline, // dropped second → already invalid
}

// LOCALS drop in reverse — `device` drops LAST
fn render() {
    let device = VulkanDevice::new();
    let pipeline = VulkanPipeline::new(&device);  // dropped first ✓
}                                                 // then device drops ✓
```

The same problem shows up in a lot of places. Vulkan and GPU code has pipelines and buffers that have to drop before the device that created them. Database code has transactions that have to commit or rollback before the connection drops. Tokio runtimes have tasks that have to finish before the `Runtime` drops, or the runtime's destructor blocks waiting. Threads that share `Arc<Mutex<>>` data need their join handles to drop before the shared state.

When drop order matters, there are three ways to fix it. The easiest is to reorder the struct fields so children come before parents in the declaration — no `unsafe`, no `ManuallyDrop`, you just write the fields in the right order. If that doesn't work, you can split the type into multiple structs so the composition encodes the ordering. The last resort is `ManuallyDrop<T>` with explicit drops in your `Drop::drop` implementation, which requires `unsafe` and is genuinely hard to get right under panic unwinding.

A few other Drop-related rules worth knowing:

You can't derive both `Copy` and `Drop` on the same type (this is compile error E0184). The reason is that a bitwise copy would alias the resource and free it twice. If your type has cleanup work to do, it can't also be `Copy`.

You can't partial-move out of a value with a `Drop` impl either (E0509). The destructor expects the whole value to be there, so taking ownership of one field by pattern-matching is forbidden. The two workarounds are `std::mem::replace(&mut self.field, default)` to swap a default into the slot, or wrapping the field in `Option<T>` and using `self.field.take()`.

`Drop` is synchronous — there's no stable `AsyncDrop` in 2026. If your type needs to do async cleanup (close a network connection, flush a buffer, send a final message), provide an explicit `async fn close(self)` method and document that callers should call it. Sync `Drop` becomes a best-effort cleanup path. Don't try to `block_on` inside `Drop` either — that deadlocks on single-threaded runtimes.

If `Drop` panics while the stack is already unwinding from another panic, the program aborts (this is the "double-panic" rule). Don't `.unwrap()` inside a `Drop` impl. Either swallow the error with `let _ = ...`, log it via `tracing::error!`, or set a flag the next operation will observe.

## Smart Pointer Decision Matrix

| Scenario                      | Use                                 |
| ----------------------------- | ----------------------------------- |
| One owner, heap needed        | `Box<T>`                            |
| Shared read, single thread    | `Rc<T>`                             |
| Shared mutable, single thread | `Rc<RefCell<T>>`                    |
| Shared read, multi-thread     | `Arc<T>`                            |
| Shared mutable, multi-thread  | `Arc<Mutex<T>>` or `Arc<RwLock<T>>` |
| Break reference cycles        | `Weak<T>`                           |

Never use `Arc` in provably single-threaded code — atomic operations are slower than `Rc`'s plain integer operations. Never wrap `Copy` types (u32, bool) in `Arc<Mutex<T>>` — use atomics or local variables.
