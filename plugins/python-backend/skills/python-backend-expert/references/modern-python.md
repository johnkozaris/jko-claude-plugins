# Modern Python

Three things, in order:

1. **Pure-Python language fundamentals** that apply to every Python program. Stable idioms from the official tutorial and PEP 8; they don't age.
2. **Strong defaults for class, data, and error design**. Stable at the language level; library positioning (Pydantic v2 default, attrs role) follows current ecosystem consensus.
3. **Version-specific features**. Assume Python 3.13+, ideally 3.14+. For older projects, the per-version table in [`2026-currency.md`](2026-currency.md) shows when each feature landed.

## Pure Python You Should Follow

These patterns apply to every Python program, backend or runtime. AI-written code that ignores them passes review and breaks under load.

### EAFP, not LBYL

"Easier to ask forgiveness than permission." Look up the key, catch the miss. Don't check first, then look up: in multi-threaded or async-concurrent code the value can change between check and use, and even in single-threaded code LBYL requires two lookups where EAFP needs one. The Python glossary calls EAFP the Pythonic style.

```python
# LBYL: two lookups
if key in d:
    value = d[key]
else:
    value = default

# EAFP: one lookup, one branch
try:
    value = d[key]
except KeyError:
    value = default

# Better yet
value = d.get(key, default)
```

Same rule for files: `try: open(path) except FileNotFoundError:` beats `if path.exists(): open(path)`.

### Truthiness traps

`if x:` is `True` when `x` is non-empty, non-zero, non-`None`. That seems convenient until you bite on:
- `if value:` matches `False`, `0`, `""`, `[]`, `{}`, `None`, `0.0`, `Decimal("0")`. If any of those is a *valid* value, you have a bug.
- The fix: be explicit. `if value is None:`, `if not items:`, `if value == 0:`. Spell what you mean.

```python
# BUG: treats 0 and "" as "missing"
if config.timeout:
    set_timeout(config.timeout)

# CORRECT
if config.timeout is not None:
    set_timeout(config.timeout)
```

Pydantic's `field == None` vs `field is None` follows the same rule; use `is None` for sentinel checks.

### `is` is identity, `==` is equality

`is` checks "same object in memory". Only use it for `None`, `True`, `False`, and `Enum` singletons. Everything else uses `==`. Writing `x is "abc"` emits a `SyntaxWarning` since Python 3.8, but plenty of code paths still slip through; just don't write it.

```python
# WRONG: identity comparison on a string literal; SyntaxWarning since 3.8
if user.role is "admin":
    ...

# RIGHT
if user.role == "admin":
    ...
if user.role is Role.ADMIN:        # OK: Enum members are singletons
    ...
```

### Iteration is a protocol, not a list

A `for x in items:` loop works for anything that's **iterable** (has `__iter__`). Within iterables there are two kinds: **iterators** (single-pass; have `__next__`; once exhausted, done) and **multi-pass iterables** (re-iterable; `__iter__` returns a fresh iterator each time).

| Single-pass iterators | Multi-pass iterables |
| --- | --- |
| Generators, `open(path)`, `csv.reader(f)`, `iter(x)` | `list`, `dict`, `set`, `tuple`, `range`, `dict.items()` / `.keys()` / `.values()` |

Most iterators stream and consume O(1) memory; once you call `list(x)` on them you've materialized everything and lost the streaming property.

```python
# Streams: O(1) memory
total = sum(int(line) for line in open("nums.txt"))

# Materializes: O(n) memory
total = sum([int(line) for line in open("nums.txt")])
```

True iterators are **single-pass**. Iterating one twice gives nothing the second time. `range(3)` and `dict.items()` are NOT iterators; iterating them twice works fine. If you need to iterate a generator twice, save the work to a list (only if it fits) or use `itertools.tee`.

### Closures capture by reference, not by value

The classic late-binding bug:

```python
# All three functions return 2
fs = [lambda: i for i in range(3)]
for f in fs:
    print(f())                          # 2, 2, 2

# Fix: default arg binds at definition time
fs = [lambda i=i: i for i in range(3)]
# Or use functools.partial
```

Same trap in `for url in urls: asyncio.create_task(fetch(url))` when `fetch` closes over `url`. Pin it: `for url in urls: asyncio.create_task(fetch(url=url))`.

### Scope: LEGB

Name lookup goes Local → Enclosing → Global → Built-in. The two failure modes:
- `global x` inside a function is rarely the right answer. If you need it, the design is probably wrong.
- `nonlocal x` is required to assign to an enclosing-function variable. Without it, `x = 1` creates a new local that shadows the outer one.

```python
def make_counter():
    n = 0
    def inc():
        nonlocal n              # without this, `n = n + 1` creates a new local and raises UnboundLocalError
        n += 1
        return n
    return inc
```

### Comparison chaining

`a < b < c` is `a < b and b < c` with `b` evaluated once. Use it for range checks; don't write `0 < x and x < 10`.

```python
if 0 <= status < 400:           # readable, b evaluated once
    ...
```

### Walrus (`:=`): use it when it removes a real duplication

Good use: avoiding a redundant call or test inside a comprehension or `while`.

```python
# Good: avoids a second call to f()
while (chunk := f.read(8192)):
    process(chunk)

# Good: avoids re-running the regex
if (m := pattern.match(line)) is not None:
    use(m.group(1))

# Bad: walrus where a plain assign would read better
total = (n := len(items)) * 2     # just use n = len(items); total = n * 2
```

When in doubt, plain assignment + a real line break is clearer.

### `match`: use for sum types, not as a fancier `if/elif`

`match` shines for discriminated unions, parsing trees, AST traversal, deconstructing structured payloads. It does **not** improve a two-branch `if`. Don't replace `if x == 1: ... elif x == 2: ...` with `match x: case 1: ... case 2: ...`; that's just longer.

```python
# Good: structural decomposition
match event:
    case OrderCreated(id=id, total=total) if total > 0:
        ...
    case OrderCancelled(id=id, reason=reason):
        ...
    case _:
        raise NotImplementedError(event)
```

Pair with `typing.assert_never` in the `_` arm to get exhaustiveness from mypy/pyright (and an `AssertionError` at runtime if a new variant somehow slips through).

### Sort by `key=`, not by `cmp=`

`sorted` and `list.sort` take a `key=callable`; the callable returns whatever the sort should order by. Python's sort is stable, so multi-key sort is `sorted(items, key=lambda r: (r.region, r.created))`. Don't write a `cmp` wrapper.

```python
sorted(orders, key=lambda o: (o.region, o.created), reverse=False)
sorted(orders, key=attrgetter("region", "created"))   # operator.attrgetter is faster
```

For caching the sort key on each element, `functools.cmp_to_key` exists but is rarely the right answer.

### Generators, `yield`, `yield from`

A function with `yield` is a generator factory: calling it returns a generator object. Iterating that object runs the function body up to the next `yield` and pauses.

```python
def read_records(path: Path) -> Iterator[Record]:
    with open(path) as f:
        for line in f:
            yield Record.model_validate_json(line)
```

Two things to remember:
- The `with open(...)` block stays open across `yield`s. The file closes when the generator is garbage-collected or `.close()`-d. For deterministic close, drive the generator inside a `with` block of your own.
- `yield from gen` delegates iteration to `gen`; cleaner than a `for x in gen: yield x` loop.

### Context managers: write your own when there's a paired setup/teardown

```python
from contextlib import contextmanager

@contextmanager
def timed(label: str):
    start = time.monotonic()
    try:
        yield
    finally:
        log.info("timed", label=label, ms=round((time.monotonic() - start) * 1000, 2))

with timed("query"):
    rows = db.execute(...).all()
```

`contextlib.ExitStack` for variable-length setups; `contextlib.suppress(SpecificError)` instead of bare `try/except/pass`.

### Decorators: order matters

The bottom-most decorator wraps the function first. Then the next one up wraps that, and so on. `@classmethod` / `@staticmethod` go outermost (they expect the function as their input, not a wrapper). `@property` followed by `@property.setter` and `@property.deleter` must be in that order.

```python
class Repo:
    @staticmethod                      # outermost
    @lru_cache(maxsize=128)            # inner
    def fetch(key: str) -> Item: ...
```

### `*args` / `**kwargs`: keyword-only after `*`

```python
def send_email(to: str, body: str, *, urgent: bool = False, retry: bool = True) -> None: ...

send_email("a@b.io", "hi")                           # ok
send_email("a@b.io", "hi", urgent=True)              # ok
send_email("a@b.io", "hi", True)                     # TypeError: keyword-only
```

The `*,` forces every flag after it to be keyword-only. Catches the boolean trap from the ai-slop catalog.

### Float comparison: `math.isclose`, not `==`

```python
0.1 + 0.2 == 0.3            # False
math.isclose(0.1 + 0.2, 0.3) # True
```

For money use `Decimal`, never `float`. For thresholds use `isclose(a, b, rel_tol=1e-9)` and pick a tolerance you can justify.

### Integer division: `//` vs `/`

`/` always returns `float`. `//` is floor division and returns `int` when both sides are `int`. `divmod(a, b)` gives `(quotient, remainder)` in one call.

```python
hours, remainder = divmod(seconds, 3600)
minutes, seconds = divmod(remainder, 60)
```

### `zip`, `enumerate`, `itertools`

- `for i, x in enumerate(items):` instead of `for i in range(len(items)): x = items[i]`.
- `zip(a, b, strict=True)` (3.10+) raises if lengths differ. Without `strict=True`, the shorter wins silently, eating data.
- `itertools.batched(iterable, n)` (3.12+) for chunking.
- `itertools.chain.from_iterable` to flatten one level.
- `itertools.groupby` requires the input already sorted on the key.

### String formatting: f-strings, with conscious format-spec usage

```python
f"{value:>10}"           # right-aligned, width 10
f"{n:_}"                 # thousands separator: 1_000_000
f"{n:,d}"                # 1,000,000
f"{f:.4f}"               # fixed-point, 4 decimals
f"{f:.3e}"               # scientific
f"{p:.1%}"               # percent: 0.25 -> "25.0%"
f"{dt:%Y-%m-%d}"         # datetime format
f"{value!r}"             # repr() instead of str()
```

`.format()` and `%` formatting still work and are sometimes necessary (formatting from a config-defined template); for inline code, f-strings everywhere.

### Path handling with `pathlib`

```python
from pathlib import Path

p = Path("~/data").expanduser()
out = p / "results.csv"           # operator overload, not strings
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text("...", encoding="utf-8")
```

Never `os.path.join` in new code. Never string concatenation for paths.

### Default-mutable-argument trap

```python
# Trap: the list is created ONCE, at function-def time, and shared across calls
def append_log(entry, log=[]):
    log.append(entry)
    return log

# Fix
def append_log(entry, log=None):
    if log is None:
        log = []
    log.append(entry)
    return log
```

Same trap for `def f(x={})` and dataclasses (`field(default_factory=list)` is the dataclass form).

### Exceptions: catch the specific type, chain across layers

```python
# Catch what you can handle, propagate the rest
try:
    row = db.fetchrow(...)
except DatabaseError as e:
    raise OrderNotFoundError(f"order {oid} not found") from e   # `from e` preserves the original cause
```

Never `except Exception` outside the outermost handler. Never bare `except:` (that catches `SystemExit` and `KeyboardInterrupt`).

### Variable typing: annotate the boundary, infer the body

Function parameters and return types: always annotated. Local variables: only when the inferred type is wrong or unclear.

```python
def process(items: list[Item]) -> Result:
    seen: set[UUID] = set()              # annotation because empty literal is ambiguous
    total = 0                            # int inferred, no annotation needed
    for item in items:
        if item.id in seen:
            continue
        seen.add(item.id)
        total += item.qty
    return Result(count=len(seen), total=total)
```

`from __future__ import annotations` is no longer needed in new 3.14+ code (deferred annotations are native). Don't churn existing files for it.

### Imports: absolute > relative, top of file, no circulars

- Absolute imports (`from src.auth import service`) are the default; relative imports (`from ..auth import service`) only when the module is moved and you want imports to follow.
- All imports at the top of the file (PEP 8). Inside-function imports are reserved for breaking import cycles or lazy-loading heavy optional deps.
- Circular imports are a design smell. Fix by extracting the shared types to a third module, not by deferring imports.

## Strong Defaults: Class, Data, Error Design

The rules below are not preferences. They are what recognized Python authors converge on in 2026. Apply them by default. Deviate only with a concrete written reason.

### When to use a class vs a function vs a module

**DO** use a **module** when you want a namespace of related functions. Python's "module" is a class for free.

**DO** use a **function** for stateless logic with no invariants to protect.

**DO** use a **class** when you have data + invariants that must travel together (a value object, an entity that enforces its own consistency).

**DO NOT** create a class purely to group functions. `class StringUtils: @staticmethod def slugify(...)` is Java cosplay; write `def slugify(s: str) -> str` in `utils/strings.py`.


### The decision matrix: dataclass vs attrs vs Pydantic vs NamedTuple vs TypedDict

| Choice | Use it for | Rationale |
| --- | --- | --- |
| `@dataclass(slots=True, frozen=True, kw_only=True)` | **Default** for domain value objects and entities | stdlib, zero deps, mypy/pyright native, all the perf wins |
| `attrs` `@define` | Domain classes that need validators / converters / extensibility hooks / PyPy | Strict superset of dataclass; moves faster than stdlib; battle-tested ecosystem |
| Pydantic `BaseModel` | **Only at I/O boundaries**. HTTP bodies, env vars, serialized JSON, message-queue payloads | Rust validation core; coercion; JSON schema. Don't use it for internal domain objects. |
| `NamedTuple` | Tuple semantics on purpose; positional unpacking, CSV/DB row shape | Equality is structural (`NT1(1) == NT2(1) == (1,)` is `True`); it's a tuple with names, not a class |
| `TypedDict` | Shape of dicts you **don't own**. third-party JSON, `**kwargs`, intermediate parsing | Zero runtime overhead; describes shape without instantiating |

Pydantic-as-domain-model is the most common AI default and the most damaging. Re-validating data read from a trusted database every time you construct a `User` is wasted work and couples your domain to a validation library. Reject this pattern; use `@dataclass` for domain, Pydantic only at the wire.


### Class options: the right defaults

**Value object default**: `@dataclass(slots=True, frozen=True, kw_only=True)`.
- `slots=True`. ~20% less memory, faster attribute access, typos can't silently create new attributes.
- `frozen=True`; immutable; gets `__hash__` for free; "action at a distance" bugs go away.
- `kw_only=True`; forces callers to be explicit; adding/reordering fields never silently breaks positional callers.

**Mutable aggregate**: `@dataclass(slots=True)`. Drop `frozen` only when you actually mutate (a shopping cart accumulating items, a domain aggregate accumulating events).

**Reject**: `@dataclass` with no options: The defaults (no slots, no frozen, positional args) are the historical compromise; in 2026 the right defaults are explicit.

```python
# Value object. frozen + slots + kw_only
@dataclass(slots=True, frozen=True, kw_only=True)
class Address:
    street: str
    city: str
    country: str

# Mutable aggregate
@dataclass(slots=True)
class ShoppingCart:
    items: list[CartItem] = field(default_factory=list)
    def add(self, item: CartItem) -> None: self.items.append(item)
```

### Inheritance vs composition vs Protocol

**DO** use composition for sharing behavior. Inject dependencies; don't inherit them.

**DO** use `typing.Protocol` for **interface contracts**. The implementing class never imports the Protocol: that's the whole point: The consumer defines what it needs; any class with that shape satisfies it.

**DO NOT** subclass for code sharing unless you're adapting a class you don't control: This is "Type 1" subclassing: the problematic kind; and it produces inheritance trees, namespace muddle, and confusing indirection.

**DO NOT** use `IFoo` Hungarian naming for Protocols: Just `MailSender`, not `IMailSender`. PEP 544 never recommended that style.

```python
# Protocol defined where it's USED. no imports required by the implementor
class MailSender(Protocol):
    def send(self, to: str, body: str) -> None:...
    class SMTPMailer:
        def send(self, to: str, body: str) -> None:...
        # satisfies MailSender structurally

def notify(sender: MailSender, email: str) -> None:
    sender.send(email, "Hello")
```

Use ABC only when you need shared concrete methods. For pure interfaces, Protocol every time.


### The constructor anti-pattern: don't do work in `__init__`

**DO NOT** open connections, hit the network, read files, or do heavy computation in `__init__`: The class becomes impossible to construct in tests without side effects.

**DO** accept already-constructed dependencies. Use a `@classmethod` factory (or a module-level factory function) for the "build me one from a URL" case.

```python
# DO NOT
class ReportService:
    def __init__(self, db_url: str):
        self.conn = psycopg2.connect(db_url) # I/O in __init__

# DO
class ReportService:
    def __init__(self, conn: Connection):
        self.conn = conn # receives built dependency

        @classmethod
        def from_url(cls, db_url: str) -> Self:
            return cls(psycopg2.connect(db_url)) # factory does the work
```


### `@cached_property` vs `@lru_cache` on methods: pick `cached_property`

**DO** use `functools.cached_property` for an expensive lazy attribute on an instance: The result is stored in `self.__dict__`; the computation runs once per instance and dies with it.

**DO NOT** use `functools.lru_cache` on an instance method: The cache lives on the **function**, not the instance. `self` is part of the cache key, so the cache holds every instance ever called for the lifetime of the process: This is a slow memory leak.

```python
# LEAK. every Report instance is kept alive by the function-scoped cache
class Report:
    @lru_cache(maxsize=None)
    def build(self) -> str:...
    # CORRECT. cached on the instance; GC reclaims it normally
class Report:
    @cached_property
    def built(self) -> str:...
```

Caveat: `@cached_property` doesn't work on `@dataclass(frozen=True)` (frozen blocks `__dict__` writes). For frozen objects either use a `_cache: dict[str, Any] = field(default_factory=dict)` with manual lookup, or compute eagerly in `__post_init__` if the computation is cheap.

### Equality and hashing

`__eq__` and `__hash__` are a contract: if `a == b`, then `hash(a) == hash(b)`. Python enforces this by setting `__hash__ = None` on any class that defines `__eq__` without `__hash__`.

**DO** use `@dataclass(frozen=True)`; you get both for free, safely.

**DO NOT** define `__eq__` on mutable objects and then put them in sets or dict keys. Hashes computed before mutation become stale; lookups silently fail.

### Data structure picks

| Need | Use | Don't reach for |
| --- | --- | --- |
| Ordered, mutable sequence | `list` | `collections.OrderedDict` (since 3.7 plain `dict` is ordered) |
| Immutable, fixed-size sequence | `tuple` | `list` cast at the boundary |
| Membership testing, unique elements | `set` / `frozenset` | `list` (O(n) lookups) |
| Queue / stack (push/pop both ends) | `collections.deque` | `list` (`pop(0)` is O(n)) |
| Closed set of named values | `enum.StrEnum` or `enum.Enum` | string literals scattered through the code |
| Open set of allowed string values | `typing.Literal["a", "b"]` | a free-form `str` |
| Dict with auto-default values | `collections.defaultdict` or `dict.setdefault` | check-then-set in a loop |
| Bytes that you slice often | `memoryview` | repeated `bytes` slicing (each slice copies) |
| Map a key to several values | `defaultdict(list)` | `dict[K, list[V]]` with manual `setdefault` |

`enum.StrEnum` (3.11+) is the right pick for things like `Status`, `Role`, `Priority`; values are real strings, fit cleanly into JSON, and pattern-match nicely. `IntEnum` is rarely the right choice; `Literal["pending","done"]` is correct when you need a tiny open set without an enum class.

### Error design: typed hierarchy, shallow, chain with `from`

**DO** define one base exception per package/module, then one layer of specifics. Two levels. Never deeper. Carry context in attributes, not just in the message string.

```python
# domain/exceptions.py: typed, two-level
class DomainError(Exception):
    """Base for all domain errors. Never catch this in handlers; let
    the registered handlers in the composition root translate to HTTP."""

class NotFoundError(DomainError):
    def __init__(self, entity: str, identifier: str) -> None:
        super().__init__(f"{entity} not found: {identifier}")
        self.entity = entity
        self.identifier = identifier

class ConflictError(DomainError):
    def __init__(self, detail: str) -> None:
        super().__init__(detail)

class AuthorizationError(DomainError):
    pass

class ServiceConnectionError(DomainError):
    def __init__(self, service_name: str, reason: str) -> None:
        super().__init__(f"connection to {service_name!r} failed: {reason}")
        self.service_name = service_name
        self.reason = reason
```

**DO** chain exceptions across layer boundaries: `raise OrderNotFoundError(...) from db_err`. The `from e` clause sets `__cause__` and `__suppress_context__=True`, giving you a clean traceback that shows the translation.

**DO** use `contextlib.suppress(SpecificError)` when you genuinely want to silence one specific kind of error. It's the explicit form of `try/except/pass`.

**DO NOT** catch bare `Exception` except at the outermost request handler / worker loop / CLI entry point. Anywhere else, name the exception you actually expect. `except Exception:` also swallows `SystemExit` and `KeyboardInterrupt` in older patterns; the cure is precision.

**DO NOT** map domain errors to HTTP inside services. The route or a registered `@app.exception_handler` does that. Services raise `OrderNotFoundError`; the adapter layer turns it into 404.

**DO NOT** swallow exceptions silently. `try: ... except Exception: pass` is a bug. `log.error(...)` without re-raising is usually a bug too (the caller gets `None` and continues into corrupt state).

```python
# Map at the adapter layer, not in the service
@app.exception_handler(NotFoundError)
async def _not_found(request: Request, exc: NotFoundError):
    return JSONResponse(
        status_code=404,
        content={"detail": str(exc), "code": f"{exc.entity.upper()}_NOT_FOUND"},
    )

@app.exception_handler(ConflictError)
async def _conflict(request: Request, exc: ConflictError):
    return JSONResponse(status_code=409, content={"detail": str(exc), "code": "CONFLICT"})

# Chain across abstraction boundaries
try:
    row = await db.fetchrow(...)
except DatabaseError as e:
    raise NotFoundError("order", oid) from e

# Explicit silence: only the error you expect
from contextlib import suppress
with suppress(KeyError):
    del cache[key]
```

### Error response shape (when you do shape one yourself)

Clients should switch on a stable machine code, not parse human strings. Never let a stack trace or driver-level message reach the wire.

```python
# Stable response shape (matches the handlers above)
{
    "detail": "A user with this email already exists",
    "code": "EMAIL_ALREADY_EXISTS",
    "status_code": 409,
}

# BAD: leaks the driver-level message
return {"detail": "SQLAlchemyError: UNIQUE constraint failed: users.email"}

# GOOD: log the internal cause, return the safe one
logger.error("duplicate_email", email=email, error=str(db_error))
return {"detail": "A user with this email already exists", "code": "EMAIL_EXISTS"}
```

### Structured logging on errors

`logger.error(f"failed: {e}")` is unqueryable. Log events with fields.

```python
# BAD
logger.error(f"Failed to create user {email}: {error}")

# GOOD
logger.error(
    "user_creation_failed",
    email=email,
    error_type=type(error).__name__,
    error_detail=str(error),
)
```

### `ExceptionGroup` / `except*`: use it for concurrent failures, not serial

PEP 654's `ExceptionGroup` is designed for **multiple concurrent failures**. That's why `asyncio.TaskGroup`, anyio task groups, and Trio nurseries always raise it. Use `except*` to handle the cases you care about.

**DO NOT** wrap a single serial error in an `ExceptionGroup` "for consistency". That's noise.

### Naming: the rules that matter

| Convention | Rule | Why |
| --- | --- | --- |
| Acronyms in CamelCase | `HttpClient`, `JsonParser`, `OauthToken`. **not** `HTTPClient` | The stdlib's `HTTPSConnection` is legacy; modern code (typeshed, mypy, FastAPI, Pydantic) uses word-cased acronyms |
| Functions | `verb_noun`. `fetch_user`, `parse_payload` | Functions DO things |
| Classes | `Noun`. `OrderService`, `PaymentGateway` | Classes ARE things |
| Predicates | `is_/has_/can_`. `is_expired`, `has_permission` | Returns bool |
| Module-private | `_name` (single leading underscore) | Convention, no enforcement |
| Name-mangled | `__name` (double leading underscore). **avoid** | Mangles to `_ClassName__name`; confuses inheritance and serialization |
| Trailing underscore | `type_`, `id_`, `class_` | Avoids builtin collisions |

### Mutability, copying, equality

**DO** default to immutability for value objects. Mutable value objects cause action-at-a-distance bugs.

**DO** use `dataclasses.replace(obj, field=new_value)` for "copy with one field changed" on a frozen dataclass.

**DO NOT** use `copy.deepcopy` casually: It's slow, and if you need it often your data structure has too much shared mutable state; fix the design.

**DO NOT** use `is` for value equality. `is` is identity. CPython interns small ints (-5 to 256) and identifier-shaped strings, so `if name is "admin"` may appear to work in some contexts and break in others. Use `==`. Use `is` only for `None`, `True`, `False`, and `Enum` singletons.

### Pythonic vs Java-esque

**DO NOT** write `get_name` / `set_name` pairs. Plain attributes. `@property` only when you need computed access; setter only when validation truly belongs at the attribute level (rare with frozen dataclasses).

**DO NOT** name a class `XManager`, `XHandler`, `XHelper`, `XProcessor` unless that word actually carries meaning. "Manager" usually means "I couldn't think of what this does".

**DO** prefer **anemic value objects + functional core** when the data is value-like (DTOs, events, snapshots): that's the right Python idiom. **Entity objects** with methods are correct when the data has lifecycle (state transitions, identity).

### 2026 defaults cheatsheet

```python
# Value object
@dataclass(slots=True, frozen=True, kw_only=True)
class Money:
    amount: int
    currency: str

# Mutable aggregate
@dataclass(slots=True)
class Cart:
    items: list[CartItem] = field(default_factory=list)

# Interface
class Repository(Protocol):
    def get(self, id: UUID) -> Entity:...
    # Pydantic at the boundary
class CreateOrderRequest(BaseModel):
    customer_id: UUID
    items: list[OrderItemRequest]

# Strict internal contract
class InternalEvent(BaseModel):
    model_config = ConfigDict(strict=True)

# Reusable constrained type
def _positive_decimal(v: Decimal) -> Decimal:
    if v <= 0:
        raise ValueError(f"must be > 0, got {v}")
        return v

PositiveDecimal = Annotated[Decimal, AfterValidator(_positive_decimal)]

# Error hierarchy. exactly 2 levels
class OrderError(Exception):...
class OrderNotFoundError(OrderError):...
# Chain
raise OrderNotFoundError(f"{oid} not found") from db_err

# Explicit silence
with suppress(KeyError):
    del cache[key]

# Closed enum
class Status(StrEnum):
    PENDING = "pending"
    DONE = "done"
```

For validation strategy (parse-don't-validate, smart constructors, Annotated constraints), see [`validation.md`](validation.md).

## Type Hints

### Basic Patterns

```python
# Function signatures: always typed
async def get_user(user_id: UUID) -> User | None: ...

# Class attributes
class Config:
    debug: bool
    db_url: str
    max_connections: int = 10

# Use built-in generics, never the typing aliases
names: list[str]                    # not List[str]
mapping: dict[str, int]             # not Dict[str, int]
ids: set[UUID]                      # not Set[UUID]
pair: tuple[str, int]               # not Tuple[str, int]
```

### Union types

```python
def process(value: str | int | None) -> str: ...
# Never the old way: Union[str, int, Optional[str]]
```

### Generic syntax (PEP 695)

```python
class Repository[T]:
    async def get_by_id(self, id: UUID) -> T | None: ...
    async def create(self, entity: T) -> T: ...

type UserID = UUID
type Result[T] = T | None
```

The old `TypeVar("T") + Generic[T]` form still works but there is no reason to write it in new code.

### TypeVar defaults

`TypeVar`, `ParamSpec`, and `TypeVarTuple` support default values. Simplifies generic APIs where most callers use the same type.

```python
class Repository[T = dict[str, Any]]:
    async def list_all(self) -> list[T]: ...

repo: Repository = ...              # T defaults to dict[str, Any]
repo: Repository[User] = ...        # T is User
```

### Deferred annotations

Python 3.14 evaluates annotations lazily by default (PEP 649). `from __future__ import annotations` is no longer needed; forward references just work. The future import still functions and won't be deprecated until after Python 3.13 reaches end-of-life (~2029), so don't churn existing files removing it.

```python
class Parent:
    children: list[Child]           # forward reference; no quotes, no __future__

class Child:
    parent: Parent

# Access annotations explicitly when needed
import annotationlib
annotations = annotationlib.get_annotations(Parent, format=annotationlib.Format.VALUE)
```

**Impact on frameworks:** FastAPI, Pydantic, and SQLAlchemy all benefit. Type-based frameworks that inspect annotations at import time get reduced startup cost. `from __future__ import annotations` is no longer needed in new 3.14+ code, but it still works fine and is only scheduled for deprecation **after Python 3.13 reaches end-of-life** (per PEP 749, expected ~2029). Don't churn existing files to remove it; just don't add it to new ones on 3.14+.

### ReadOnly TypedDict (3.13+)

```python
from typing import ReadOnly, TypedDict

class UserConfig(TypedDict):
    name: str
    api_key: ReadOnly[str] # Type checkers flag attempts to modify this
```

### TypeIs for Type Narrowing (3.13+): prefer over `TypeGuard`

`TypeIs` (PEP 742) narrows in **both** branches; `TypeGuard` (PEP 647) narrows only when it returns `True`. Use `TypeIs` for new code: it matches the way humans read a predicate.

```python
from typing import TypeIs

def is_admin(user: User | AnonymousUser) -> TypeIs[User]:
    return isinstance(user, User) and user.is_admin

# After is_admin returns True, type checkers narrow to User
if is_admin(current_user):
    current_user.admin_panel # current_user is User here
else:
    current_user.show_login # current_user is AnonymousUser here. TypeGuard wouldn't narrow this branch
```

Only fall back to `TypeGuard` when the predicate is *wider* than the type system can express (e.g. "this str is a hex-color literal". `TypeIs` requires the narrowed type to be a real subtype of the input).

## Protocol: Structural Subtyping

Prefer Protocol over ABC when you want duck typing with type safety:

```python
from typing import Protocol, runtime_checkable

@runtime_checkable
class Encryptable(Protocol):
    def encrypt(self, data: str) -> str:...
    def decrypt(self, data: str) -> str:...
    # Any class with encrypt/decrypt methods satisfies this
# No inheritance required!
class FernetEncryption:
    def encrypt(self, data: str) -> str:...
    def decrypt(self, data: str) -> str:...
    # Works without inheriting from Encryptable
def process(enc: Encryptable) -> None:
    enc.encrypt("data") # Type-checked
```

### Protocol vs ABC

| Feature | Protocol | ABC |
|---|---|---|
| Requires inheritance | No (structural) | Yes (nominal) |
| Runtime checkable | Optional (`@runtime_checkable`) | Always |
| Abstract methods | Implicit (all methods) | Explicit (`@abstractmethod`) |
| Best for | Ports, interfaces | Shared base behavior |

**Rule of thumb:** Use Protocol for ports in hexagonal architecture. Use ABC when you need shared method implementations.

## Dataclasses vs attrs vs Pydantic

### When to Use Each

| Tool | Use For | Strengths |
|---|---|---|
| `dataclass` | Domain entities, value objects, internal DTOs | stdlib, zero deps, fast; `slots=True, frozen=True` for memory + immutability |
| `attrs` | Classes needing validators, converters, custom `__eq__`, richer features | "a full toolkit to write powerful classes" vs dataclasses' "easy class with attributes" |
| Pydantic v2 | API request/response, settings, untrusted input at boundaries | Rust core, fast validation, JSON Schema, coercion |
| `NamedTuple` | Small immutable value types | tuple performance, pattern matching |

**Rule:** Pydantic belongs at I/O boundaries. Don't model internal domain objects with it; re-validating data read from a trusted database on every construction is wasted work and couples the domain to a validation library (the attrs project makes this argument explicitly). Use `@dataclass(slots=True, frozen=True)` for domain entities; reach for `attrs` when you need validators/converters without Pydantic's validation-everywhere posture.

> Use `msgspec.Struct` when you need decode+validate in one Rust pass and your schemas are stable: it beats orjson on typed JSON decoding. Use it for internal hot paths and message queues. Don't reach for it at the FastAPI boundary; FastAPI 0.130+ already Rust-serializes Pydantic models, and msgspec doesn't replace Pydantic's validator ecosystem.

### Dataclass Best Practices

```python
from dataclasses import dataclass, field
from uuid import UUID
from datetime import datetime, timezone

@dataclass(slots=True, frozen=True) # slots=True for memory, frozen for immutability
class User:
    id: UUID
    email: str
    created_at: datetime
    roles: list[str] = field(default_factory=list)

@dataclass(slots=True) # Mutable when state changes needed
class ServiceConnection:
    id: UUID | None
    name: str
    base_url: str
    is_enabled: bool = True
    health_status: HealthStatus = HealthStatus.UNKNOWN
```

### Pydantic 2.12+ for API Boundaries

```python
from pydantic import BaseModel, Field, EmailStr, ConfigDict

class CreateUserRequest(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8, max_length=128)

class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: UUID
    email: str
    created_at: datetime
```

### msgspec (optional, high-throughput internal serialization)

```python
import msgspec

class CacheEntry(msgspec.Struct, frozen=True):
    id: UUID
    payload: bytes
    created_at: datetime

# Optional: very fast encode/decode for internal hot paths.
# Not a replacement for Pydantic at the FastAPI boundary.
```

## Pattern Matching

```python
# Error-handling dispatch
match exc:
    case ServiceConnectionError():
        detail = f"Connection to '{exc.service_name}' failed"
        code = ApiMessageCode.SERVICE_CONNECTION_FAILED
    case UnsupportedServiceError():
        detail = f"Unsupported service: {exc.service_type}"
        code = ApiMessageCode.SERVICE_TYPE_UNSUPPORTED
    case _:
        detail = "Request failed"
        code = ApiMessageCode.HTTP_BAD_REQUEST

# Structural decomposition over typed events
match event:
    case UserCreated(user_id=uid):
        await notify_admins(uid)
    case UserDeleted(user_id=uid, reason=reason):
        await archive_user_data(uid, reason)
    case _ as unreachable:
        assert_never(unreachable)        # catches forgotten new variants at typecheck time
```

## StrEnum

```python
from enum import StrEnum

class ServiceType(StrEnum):
    GITHUB = "github"
    GITLAB = "gitlab"
    PORTAINER = "portainer"

# Auto-serializes to its string value in JSON; no .value needed
```

## Modern Collections

```python
# TypedDict for structured dicts (avoid dict[str, Any])
from typing import TypedDict

class HealthCheckResult(TypedDict):
    healthy: bool
    latency_ms: float
    message: str

# collections.abc for type hints on iterables
from collections.abc import Sequence, Mapping, AsyncGenerator

async def stream_results() -> AsyncGenerator[User, None]:
    async for row in result_stream:
        yield row
```

## Pydantic Settings (Configuration)

```python
from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import Field, SecretStr

class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="APP_",
        case_sensitive=False,
    )

    database_url: str
    secret_key: SecretStr            # Never printed in logs
    debug: bool = False
    encryption_key: str = Field(min_length=32)
    max_connections: int = 10

settings = Settings()  # Auto-loads from env vars
```

## Datetime: Use Timezone-Aware Always

```python
from datetime import datetime, timezone

# CORRECT (3.12+)
now = datetime.now(timezone.utc)

# DEPRECATED since 3.12 (no confirmed removal date -- avoid in all new code)
now = datetime.utcnow() # Returns naive datetime, causes bugs
```

## Deprecated Patterns to Replace

| Old Pattern | Modern Replacement | Since |
|---|---|---|
| `datetime.utcnow` | `datetime.now(timezone.utc)` | deprecated 3.12 |
| `from __future__ import annotations` | Native deferred annotations | 3.14 |
| `typing.Optional[X]` | `X \| None` | 3.10 |
| `typing.Union[X, Y]` | `X \| Y` | 3.10 |
| `typing.List`, `Dict`, `Tuple` | `list`, `dict`, `tuple` | 3.9 |
| `TypeVar("T"); Generic[T]` | `class Foo[T]:` | 3.12 |
| `collections.OrderedDict` | `dict` (ordered since 3.7) | 3.7 |
| `pkg_resources` | `importlib.resources` / `importlib.metadata` | 3.9 |
| `asyncio.get_event_loop` | `asyncio.get_running_loop` | 3.10 |
| `@asyncio.coroutine` | `async def` | 3.5 (removed 3.11) |

## Free-Threaded Python (3.14)

Python 3.14 officially supports the free-threaded build (no GIL). Performance penalty is ~5-10% on single-threaded code: The `concurrent.interpreters` module enables true multi-core parallelism via subinterpreters.

**For backend developers:** This matters most for CPU-bound work that currently uses `multiprocessing` or `asyncio.to_thread`. I/O-bound backends (most web APIs) see minimal benefit yet, but the ecosystem is preparing. SQLAlchemy 2.1 beta ships free-threaded wheels.
