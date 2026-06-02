# Validation: Parse, Don't Validate

Validate at the boundary. Trust the typed objects everywhere else. This is the "parse, don't validate" rule; it's the only model for input handling that scales.

After parsing succeeds, every function inside the service operates on the trustworthy type and never re-checks. If a codebase already validates in the route, re-validates in the service, and defensively re-checks in the repository, it has a validation-debt problem. Fix it by deleting validation, not by adding more.

## The One Rule

**Validate at the boundary. Trust everywhere else.**

| Layer | Validation responsibility |
| --- | --- |
| HTTP route | Receive a `BaseModel` parameter. FastAPI calls `model_validate`, returns 422 on failure. You don't catch it. |
| Pydantic model | Shape, format, regex, cross-field constraints. Fires before your code runs. |
| Service layer | **Business invariants only** (uniqueness, balance, state transitions). Raises typed domain errors: Never `ValidationError`. |
| Repository | Zero validation. Inputs are already typed domain objects. |
| Domain entity | Construction-time invariants (smart constructors). Once constructed, valid forever. |

If your service has `if not isinstance(req.email, str)`, delete it. If it has `if not request.get("email")`, you're not using Pydantic correctly.

## Parse, Don't Validate: The Pattern

The rule: *validation* returns `None` or raises on bad data: it discards information. *Parsing* returns a richer, proof-carrying type: it preserves information. After a successful parse, the type itself is proof that the data is valid.

```python
# DO. parse at the boundary, return a rich type
from fastapi import APIRouter
from pydantic import BaseModel, EmailStr, field_validator

router = APIRouter()

class UserCreateRequest(BaseModel):
    email: EmailStr
    username: str
    age: int

    @field_validator("username", mode="after")
    @classmethod
    def trim_and_check(cls, v: str) -> str:
        v = v.strip()
        if not v:
            raise ValueError("username must not be blank")
        return v

@router.post("/users", status_code=201)
async def create_user(body: UserCreateRequest, service: UserServiceDep) -> UserResponse:
    # FastAPI already called model_validate. If we're here, body is valid.
    # The service does not re-check. It cannot, because the type says so.
    return await service.create(body)
```

```python
# DO NOT. "shotgun parsing": every layer re-checks
async def create_user(data: dict):
    if "email" not in data: raise HTTPException(422, "email required")
    if "@" not in data["email"]: raise HTTPException(422, "bad email")
    return await service.create(data)

async def create(data: dict):  # service re-validates
    if not isinstance(data.get("email"), str):
        raise ValueError("...")
```

The dict-passing approach is what AI assistants generate by default. It's untyped, untestable, and every new caller is a new place validation might be forgotten. Refusing it is non-negotiable.

## Request and Response Schemas: Always Separate

One model for both directions is a bug magnet. Different fields are required, different fields are server-set, different fields are secret. Split them.

```python
# BAD: one schema for everything
class UserSchema(BaseModel):
    id: UUID | None = None
    email: str
    password: str | None = None
    created_at: datetime | None = None

# GOOD: one schema per purpose
class CreateUserRequest(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8)

class UpdateUserRequest(BaseModel):
    email: EmailStr | None = None
    display_name: str | None = None

class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: UUID
    email: str
    display_name: str | None
    created_at: datetime
```

**DO** set `model_config = ConfigDict(from_attributes=True)` on response models so you can return the ORM row directly. Let FastAPI's `response_model` do the serialization.

**DO NOT** return a `Pydantic` instance AND set `response_model=` to the same class. FastAPI constructs it twice. Return the ORM row.

**DO NOT** return `dict[str, Any]` from a handler. Declare a typed `response_model`.

### Schema organization (per-domain)

Each domain owns its `schemas.py` next to its `router.py` and `service.py`. Split into a `schemas/` package only once it crosses the LOC cap.

```
src/
  users/schemas.py     # CreateUserRequest, UpdateUserRequest, UserResponse
  auth/schemas.py      # LoginRequest, TokenResponse
  orders/schemas.py    # CreateOrderRequest, OrderResponse
```

### Pydantic v2 gotcha: contradictory constraint + default

```python
# BAD: constraint contradicts the None default
age: int = Field(ge=18, default=None)

# GOOD: pick one
age: int = Field(ge=18)                      # required, constrained
age: int | None = Field(default=None, ge=18) # optional, constrained when present
```

### A shared base model for global serialization

Centralize datetime serialization so timestamps go out timezone-aware and consistent across every response model.

```python
from datetime import datetime
from zoneinfo import ZoneInfo
from pydantic import BaseModel, ConfigDict, field_serializer

class AppModel(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    @field_serializer("*", when_used="json", check_fields=False)
    def _serialize_dt(self, value):
        if isinstance(value, datetime):
            if value.tzinfo is None:
                value = value.replace(tzinfo=ZoneInfo("UTC"))
            return value.strftime("%Y-%m-%dT%H:%M:%S%z")
        return value
```

### Domain entities are not Pydantic

Keep Pydantic at the API/settings boundary. Domain entities are `@dataclass(slots=True, frozen=True)`. Re-validating data read from a trusted database on every access is wasted work and welds the domain to a validation library.

## The Three Validation Layers

| Layer | Responsibility | Example |
| --- | --- | --- |
| **Schema validation** (Pydantic at the route) | Shape, types, format | Email format, string length, required fields |
| **Domain validation** (Service) | Business rules | "User must have unique email", "Order total > 0" |
| **Database constraints** (DDL) | Data integrity safety net | Unique index, foreign keys, NOT NULL |

All three layers are needed. Schema validation rejects bad payloads cheaply. Domain validation enforces rules the DB can't express. Constraints catch the race conditions both upper layers can miss.

```python
# Schema (Pydantic, entrypoint)
class CreateUserRequest(BaseModel):
    email: EmailStr                      # Format
    password: str = Field(min_length=8)  # Length

# Domain (Service, application layer)
class UserService:
    async def create_user(self, email: str, password: str) -> User:
        if await self._repo.get_by_email(email):
            raise DuplicateEmailError(email)  # Business rule

# Database constraint (ORM model, infrastructure)
email: Mapped[str] = mapped_column(String(255), unique=True)  # Safety net
```

## Pagination Contracts

```python
from fastapi import Query

class PaginatedResponse[T](BaseModel):
    items: list[T]
    total: int
    page: int
    page_size: int
    has_next: bool

@router.get("/users", response_model=PaginatedResponse[UserResponse])
async def list_users(
    service: UserServiceDep,
    page: Annotated[int, Query(ge=1)] = 1,
    page_size: Annotated[int, Query(ge=1, le=100)] = 20,
):
    users, total = await service.list_paginated(page, page_size)
    return PaginatedResponse(
        items=users,
        total=total,
        page=page,
        page_size=page_size,
        has_next=(page * page_size) < total,
    )
```

**DO** cap `page_size` with `le=`. Without a ceiling, a client requests `page_size=10_000_000` and OOMs the worker.
**DO** prefer cursor pagination for infinite scroll and ordered timelines. Offset pagination degrades quadratically and skips/duplicates rows when data shifts under the user.

## API Versioning

```python
from fastapi import APIRouter

v1 = APIRouter(prefix="/api/v1")
v2 = APIRouter(prefix="/api/v2")
v1.include_router(users_v1_router)
v2.include_router(users_v2_router)
app.include_router(v1)
app.include_router(v2)
```

Path-prefix versioning is the default. Header-based versioning (`Accept: application/vnd.app.v2+json`) is possible via a dependency that reads the header and dispatches, but it complicates caching and CDN configuration; use only when you have a concrete reason.

## Types Carry Invariants: `NonEmptyString`, `Cents`, `UserID`

A type that *carries* the invariant cannot be misused. Build them once, reuse them everywhere.

```python
from typing import Annotated, NewType
from pydantic import AfterValidator, BaseModel

# Nominal IDs (type-checker enforcement, zero runtime cost)
UserID = NewType("UserID", int)
OrderID = NewType("OrderID", int)

def get_user(uid: UserID) -> User:...
# get_user(OrderID(42)) # pyright/mypy error: expected UserID, got OrderID

# Value-constrained types. Pydantic runtime + type-checker
def _non_empty(v: str) -> str:
    v = v.strip()
    if not v:
        raise ValueError("must not be blank")
        return v

def _non_negative(v: int) -> int:
    if v < 0:
        raise ValueError(f"must be non-negative, got {v}")
        return v

def _positive(v: int) -> int:
    if v <= 0:
        raise ValueError(f"must be > 0, got {v}")
        return v

NonEmptyString = Annotated[str, AfterValidator(_non_empty)]
Cents = Annotated[int, AfterValidator(_non_negative)]
PositiveInt = Annotated[int, AfterValidator(_positive)]

# Reuse across every model. never write `if not name: raise` in a service again
class ProductCreate(BaseModel):
    name: NonEmptyString
    price_cents: Cents
    stock: PositiveInt
```

Once you hold a `NonEmptyString`, the string is not blank. Period. Functions that take `NonEmptyString` don't check. Functions that produce one have to validate: The compiler does the bookkeeping.

## Strict vs Lax Pydantic: A Hard Rule

Pydantic v2 has two modes (<https://docs.pydantic.dev/latest/concepts/strict_mode/>): The rule for which to use is **about data origin**, not preference.

| Source | Mode | Why |
| --- | --- | --- |
| HTTP body, form fields, env vars, CSV rows | **Lax** (default) | Wire format is strings-all-the-way-down. `"42"` → `42` is correct behavior. |
| Internal service-to-service calls, message queue payloads with typed schema, gRPC | **Strict** | You control the types. Coercion = programming bug. |
| Database rows (`from_attributes=True`) | **Strict** | The DB already has typed columns. Coercion = schema/migration bug. |

```python
# External boundary. lax (default), accepts coerced inputs
class UserCreateRequest(BaseModel):
    email: EmailStr
    age: int # "42" from JSON becomes 42

# Internal contract. strict, rejects anything not already typed
class InternalUserEvent(BaseModel):
    model_config = ConfigDict(strict=True)
    user_id: UUID # must be UUID instance, not str
    occurred_at: datetime # must be datetime, not iso string

# Per-field strictness. most surgical
class HybridRequest(BaseModel):
    user_id: Annotated[UUID, Field(strict=True)] # path param, must already be UUID
    notes: str # body field, normal coercion
```

The mistake to avoid: defaulting everything to strict for "safety". You'll break clients that send `{"age": "42"}` over JSON, where coercion is the protocol-defined behavior.

## Defensive vs Offensive: Interior Code Is Offensive

Defensive programming inside a service is a smell. If your service receives a typed `UserCreateRequest` from the route, adding `if not isinstance(req.email, str)` is not defensive: it papers over a contract violation that should crash hard so the broken caller is found in test, not production.

```python
# DO. be offensive about the contract
@define
class Money:
    amount: int = field(validator=[v.instance_of(int), v.ge(0)])
    currency: str = field(validator=[v.instance_of(str), v.min_len(3), v.max_len(3)])

    def add(self, other: Money) -> Money:
        assert self.currency == other.currency, f"currency mismatch: {self.currency} vs {other.currency}"
        return Money(amount=self.amount + other.amount, currency=self.currency)

# Boundary (defensive): convert exceptions to user-facing errors
try:
    m = Money(amount=request_amount, currency=request_currency)
except (TypeError, ValueError) as e:
    raise HTTPException(422, str(e))

# Interior (offensive): assert the invariant. if it fails, it's a bug, not bad input
def apply_discount(price: Money, discount: Money) -> Money:
    assert price.currency == discount.currency # crash hard
    return price.add(Money(-discount.amount, price.currency))
```

`try/except Exception: return None` inside a service is the AI default: It turns invariant violations into mystery `None`s downstream and corrupts data quietly: Never do it.

## Cross-Field Invariants. `@model_validator(mode="after")`

For constraints that involve multiple fields, use `@model_validator(mode="after")`: It runs once, after every field has passed its own validator, with a fully-typed `self`.

```python
from pydantic import BaseModel, model_validator
from typing import Self

class DateRange(BaseModel):
    start: date
    end: date
    max_days: int = 365

    @model_validator(mode="after")
    def check_range(self) -> Self:
        if self.end < self.start:
            raise ValueError(f"end ({self.end}) must be ≥ start ({self.start})")
            if (self.end - self.start).days > self.max_days:
                raise ValueError(f"range exceeds {self.max_days} days")
                return self

class PasswordReset(BaseModel):
    password: SecretStr
    password_confirm: SecretStr

    @model_validator(mode="after")
    def passwords_match(self) -> Self:
        if self.password.get_secret_value() != self.password_confirm.get_secret_value():
            raise ValueError("passwords do not match")
            return self
```

Do not put cross-field checks in `@field_validator`; if the *first* field's validator fails, the second field's validator never runs, and you get confusing partial errors.

## Smart Constructors: The Only Way to Build a Valid Instance

If a domain type has invariants, make the constructor the **only** valid path. Combine `frozen=True` (immutable after creation) + `model_validator` (validates on construction) + an explicit `@classmethod` factory.

```python
import re
from pydantic import BaseModel, ConfigDict, field_validator

class EmailAddress(BaseModel):
    """
    If you hold an EmailAddress, the @ is there, the domain is there, and it's lowercased.
    There is no other way to construct one.
    """
    model_config = ConfigDict(frozen=True)
    value: str

    @field_validator("value", mode="before")
    @classmethod
    def normalize(cls, v: str) -> str:
        v = v.strip().lower()
        if not re.match(r"^[^@\s]+@[^@\s]+\.[^@\s]+$", v):
            raise ValueError(f"invalid email: {v!r}")
        return v

    @classmethod
    def parse(cls, raw: str) -> "EmailAddress":
        return cls.model_validate({"value": raw})

    def __str__(self) -> str:
        return self.value

# Usage
email = EmailAddress.parse(" User@EXAMPLE.COM ")
# email.value == "user@example.com"
# EmailAddress(value="not-an-email") raises ValidationError
```

Anything that takes `EmailAddress` knows it has a valid, normalized email: no defensive re-checking, no string-pattern matching at every call site.

## Env Vars. `str` Is a Smell

Reading env vars as `os.environ["PORT"]` and parsing at the call site fails at request time. Use `pydantic-settings` once, fail at startup, get typed config everywhere.

```python
from pydantic import PostgresDsn, RedisDsn, SecretStr, field_validator
from pydantic_settings import BaseSettings, SettingsConfigDict
from typing import Literal

class Settings(BaseSettings):
    model_config = SettingsConfigDict(env_prefix="APP_", env_file=".env")

    database_url: PostgresDsn        # parsed + validated as a real DSN
    redis_url: RedisDsn
    secret_key: SecretStr            # never appears in repr/logs
    debug: bool = False              # "true"/"1"/"yes" all coerced
    port: int = 8000
    log_level: Literal["DEBUG", "INFO", "WARNING", "ERROR"] = "INFO"
    allowed_origins: list[str] = []  # APP_ALLOWED_ORIGINS='["a.com","b.com"]'
    max_workers: int = 4

    @field_validator("max_workers", mode="after")
    @classmethod
    def reasonable_workers(cls, v: int) -> int:
        if not 1 <= v <= 64:
            raise ValueError(f"max_workers must be 1-64, got {v}")
        return v

settings = Settings # fails at import. before any HTTP / DB I/O
```

The AI default. `os.environ.get("DATABASE_URL", "")` with a warning later; defers the failure to the first DB call, produces a cryptic connection error instead of a configuration error, and hides misconfiguration behind a warning that gets ignored.

## Idempotency: Consistency, Not Just Networking

Idempotency is a property of *commands*. Three mechanisms; pick by the situation.

**Natural key + `ON CONFLICT`**. when there is a real business uniqueness constraint:

```python
# One signup per email: DB enforces; retries are safe
row = await db.fetchrow("""
INSERT INTO users (email, name) VALUES ($1, $2)
ON CONFLICT (email) DO NOTHING
RETURNING id, email, name, created_at
""",
req.email, req.name,
)
if row is None:
    row = await db.fetchrow("SELECT id, email, name, created_at FROM users WHERE email = $1", req.email)
return User(**row)
```

**Client-supplied idempotency key**. when natural uniqueness doesn't exist (payments, generic POST commands):

```python
@router.post("/payments")
async def create_payment(body: PaymentRequest,
idempotency_key: Annotated[str, Header],
user: CurrentUser,
redis: Annotated[Redis, Depends(get_redis)],
):
    # Scope to the user. never global, or one user can replay another's key
    cache_key = f"idem:{user.id}:{idempotency_key}"
    if cached := await redis.get(cache_key):
        return JSONResponse(json.loads(cached), status_code=200)
        result = await process_payment(body)
        await redis.setex(cache_key, 86400, result.model_dump_json())
        return result
```

**Event-ID dedupe**. when consuming from a queue:

```python
async def handle_event(event: OrderCreated):
    if await db.fetchval("INSERT INTO processed_events (id) VALUES ($1) ON CONFLICT DO NOTHING RETURNING id", event.id) is None:
        return # already processed, drop
        await order_service.create(event.payload)
```

## Optimistic vs Pessimistic Locking

Concurrency invariants need explicit choice. Pick one per resource.

**Optimistic (default for most reads + writes)**: a `version` column; SQLAlchemy raises `StaleDataError` when the UPDATE finds zero matching rows.

```python
from sqlalchemy.exc import StaleDataError

class Account(Base):
    __tablename__ = "accounts"
    __mapper_args__ = {"version_id_col": "version"} # SQLAlchemy auto-checks on UPDATE/DELETE

    id: Mapped[UUID] = mapped_column(primary_key=True)
    balance_cents: Mapped[int]
    version: Mapped[int] = mapped_column(default=0)

async def withdraw(db: AsyncSession, account_id: UUID, amount: int) -> None:
    account = await db.get(Account, account_id)
    if account is None:
        raise AccountNotFoundError(account_id)
        if account.balance_cents < amount:
            raise InsufficientFundsError(account_id)
            account.balance_cents -= amount
            # On flush, SQLAlchemy issues UPDATE... WHERE id=? AND version=?
            # and raises StaleDataError if rowcount is 0.
            try:
                await db.flush()
            except StaleDataError as e:
                raise ConcurrentUpdateError(account_id) from e
```

**Pessimistic (use sparingly; for hot rows or where retry is impractical)**. `SELECT FOR UPDATE`:

```python
async def withdraw_pessimistic(db: AsyncSession, account_id: UUID, amount: int) -> None:
    async with db.begin:
        account = (await db.execute(select(Account).where(Account.id == account_id).with_for_update
        )).scalar_one()
        if account.balance_cents < amount:
            raise InsufficientFundsError(account_id)
            account.balance_cents -= amount
            # Commit releases the lock
```

Default to optimistic. Pessimistic locking serializes writers and is a contention magnet under load.

## `ExceptionGroup` / `except*`. Batch Validation

When you genuinely need to collect *all* failures and report them at once (form validation UX, data import), use `ExceptionGroup` (Python 3.11+). For fast-fail single-error flow, stay with regular exceptions.

```python
async def validate_order_items(items: list[OrderItem]) -> None:
    errors: list[Exception] = []
    for i, item in enumerate(items):
        try:
            await validate_product_exists(item.product_id)
            await validate_stock_available(item.product_id, item.quantity)
        except (ProductNotFoundError, InsufficientStockError) as e:
            errors.append(type(e)(f"item[{i}]: {e}"))
            if errors:
                raise ExceptionGroup("order validation failed", errors)

# Caller handles by sub-type:
try:
    await validate_order_items(items)
except* ProductNotFoundError as eg:
    raise HTTPException(422, {"error": "products_missing", "items": [str(e) for e in eg.exceptions]})
except* InsufficientStockError as eg:
    raise HTTPException(409, {"error": "insufficient_stock", "items": [str(e) for e in eg.exceptions]})
```

Returning only the first error when validating 10 000 rows for batch import is the worst UX in data tools; users fix-and-retry forever. Use `ExceptionGroup` when the caller benefits from seeing everything at once.

## Schema Evolution: Additive Only

The only safe change model for live distributed systems. **Never rename a field: Never change a field's type: Never tighten a constraint without a deprecation window.**

```python
# v1
class UserResponseV1(BaseModel):
    id: int
    email: str
    created_at: datetime

# v2. additive change; v1 clients still work
class UserResponseV2(BaseModel):
    id: int
    email: str
    created_at: datetime
    display_name: str | None = None # NEW. optional with default
    legacy_username: str | None = Field(None,
    description="Deprecated. Use display_name. Removed in v3.",
    json_schema_extra={"deprecated": True},
    )

# For event schemas, version field is mandatory:
class UserCreatedEvent(BaseModel):
    event_version: int = 2
    user_id: int
    email: str
    display_name: str | None = None # added in v2
```

Renaming `price` to `price_usd` in a deployed model is how rolling deploys turn into outages: old pods write the old name, new pods read the new name, both fail at the version boundary.

## Inter-Service Contracts. Codegen Inbound, Hand-Write Outbound

For service-to-service in 2026, you **own** your service's Pydantic models and the OpenAPI schema they generate. Consumers **generate** client models from that schema. When you ship a breaking change, their typecheck fails in CI, not in production at 3am.

```bash
# Generate Pydantic v2 client models from a live FastAPI service
uv run datamodel-codegen \
 --url http://payments-service/openapi.json \
 --output src/clients/payments_models.py \
 --output-model-type pydantic_v2.BaseModel \
 --use-annotated \
 --strict-nullable
```

```python
# Then in your service, parse responses strictly (contract drift fails loudly here)
from src.clients.payments_models import PaymentResponse

async def get_payment(payment_id: str) -> PaymentResponse:
    r = await http_client.get(f"http://payments-service/payments/{payment_id}")
    r.raise_for_status
    return PaymentResponse.model_validate(r.json()) # strict by default for inter-service
```

Accepting `dict` from another service and doing `data["payment"]["amount"]` is the single largest source of subtle microservice bugs. Always parse, always strict, for any data crossing a service boundary.

## `assert_never` for Exhaustive Match

When you discriminate on a `Union`/`Literal`, force the type checker to verify you handled every case. Add a new variant → forget a case → typecheck fails in CI.

```python
from typing import assert_never, Literal

class CardPayment(BaseModel):
    kind: Literal["card"]
    card_number: str
    cvv: str

class BankTransfer(BaseModel):
    kind: Literal["bank"]
    iban: str

class CryptoPayment(BaseModel):
    kind: Literal["crypto"]
    wallet_address: str

PaymentMethod = CardPayment | BankTransfer | CryptoPayment

def process_payment(method: PaymentMethod) -> str:
    match method:
        case CardPayment(card_number=num): return f"charge card...{num[-4:]}"
        case BankTransfer(iban=iban): return f"transfer to {iban}"
        case CryptoPayment(wallet_address=a): return f"send to {a}"
        case _ as unreachable:
            assert_never(unreachable) # mypy/pyright errors here if you forgot a case
```

`else: raise ValueError("unknown payment type")` defers the missing-case error to runtime, on the code path that gets called. `assert_never` makes it compile-time and catches it in CI.

## Property-Based Testing with Hypothesis

For typed invariants. `Money`, `EmailAddress`, anything with a constraint. Hypothesis finds the cases your example-based tests miss: Specifically:

```python
from hypothesis import given, assume, strategies as st

@composite
def money_strategy(draw) -> Money:
    return Money(amount=draw(st.integers(min_value=0, max_value=1_000_000_00)),
    currency=draw(st.sampled_from(["USD", "EUR", "GBP"])),
    )

@given(a=money_strategy, b=money_strategy)
def test_money_addition_preserves_invariants(a: Money, b: Money) -> None:
    assume(a.currency == b.currency)
    result = a.add(b)
    assert result.amount == a.amount + b.amount
    assert result.currency == a.currency
    assert result.amount >= 0
```

Hypothesis will find `user@[::1]`, `"user"@example.com`, and zero-balance edge cases you didn't think of. Use it for any value-constrained type that ships to production.

## When `Result[T, E]` Is Worth It (Rarely)

For 99 % of FastAPI application code, **use exceptions**. They're typed (mypy can check), they're Pythonic, they integrate with FastAPI's exception handlers, and every Python developer reads them at a glance.

Reach for a `Result[T, E]` pattern (hand-rolled, or via the `returns` library) **only** when:
- You need to accumulate multiple errors instead of failing fast.
- You're writing a library and don't want to couple callers to your exception hierarchy.
- You're in a hot path where exception construction overhead actually shows up in profiles.

Application code using `.map.bind.alt` chains throughout is cognitive overhead without a payoff: that's not the Python way, and reviewers will hate it.

## The Anti-Pattern Gallery

| Pattern | What AI generates | Why it's wrong |
| --- | --- | --- |
| God validator | `validate_everything(data: dict)` with 200 lines of `if/else` | Untestable, untyped, grows without bound. King's "shotgun parsing" |
| Silent coercion | `age = int(data.get("age", 0))` | Hides `None` and missing keys; `0` is a valid age, the missing-field bug is masked |
| Re-validation | Service re-checks what the route already validated | Mixes boundary and interior responsibility; nobody knows what guarantees what |
| Dict-passing | `def create_user(data: dict)` | No type contract; every caller reads the source to know what keys are needed |
| Catch-all except | `except Exception: return None` | Swallows invariant violations; turns crashes into silent data corruption |
| Mutable schema | Renaming `price` → `price_usd` in production | Breaks deployed consumers; additive-only is the only safe model |
| `str` config | `PORT = os.environ["PORT"]` inline | Scattered parsing; fails at request time, not startup |
| Comment-as-contract | `email: str # must be a valid email` | Comments are not types. The type says any string; anyone can pass garbage. |

## The 2026 Mental Model

1. **Parse once at the boundary** (Pydantic model at the route). Never parse in the interior.
2. **Types carry invariants** (`Cents`, `NonEmptyString`, `EmailAddress`). If you hold the type, the invariant holds.
3. **Lax outside, strict inside.** HTTP input is lax-coerced; internal calls are strict.
4. **Be offensive about interior contracts.** `assert`, don't `try/except Exception: return None`.
5. **Domain errors ≠ validation errors.** `ValidationError` is 422 (shape); `InsufficientFundsError` is 422 (business); `StaleDataError` is 409.
6. **Evolve schemas additively.** Rename nothing. Make new fields optional. Version endpoints for real breaks.
7. **Hypothesis finds the cases you didn't imagine.** Write property tests for every constrained type.

