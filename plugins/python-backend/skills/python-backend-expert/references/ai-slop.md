# AI Slop: The Top 20 Patterns

A senior Python engineer reviewing AI-assisted FastAPI code in 2026 keeps seeing the same patterns. They compile. They pass the sync test client. They look plausible. They're wrong in production.

The 20 patterns below (10 code-level, 10 architectural) come with a WHY, BAD code an LLM produces, GOOD code an engineer writes, and a brief source. These go deeper than the canonical beginner traps (`mutable default arg`, `bare except`, `time.sleep` in async) which are covered in [`modern-python.md`](modern-python.md). Most of these patterns survive review and only fail under production conditions.

## The AI Slop Test

Before diving into specific patterns, when you look at a Python backend ask:
- Could a senior engineer immediately tell this was AI-generated?
- Does the structure look "trained on every tutorial blog post from 2020" rather than "designed for this problem"?
- Are there idioms that don't match the rest of the codebase?

If yes, the problem is not any specific line. The 20 patterns below are the most common evidence.

## Code-Level

### CODE-01  Pydantic v1 ghosts in a v2 project

**Why it matters.** LLMs were trained on a corpus that's overwhelmingly Pydantic v1. They emit `.dict`, `.json`, `@validator`, `class Config: orm_mode = True`, `schema_extra`: every one of those is either removed or silently broken in v2. The dangerous one is `orm_mode = True` inside `class Config`: it's *silently ignored* in v2. Your ORM-to-schema conversion looks fine in unit tests (where you passed a dict) and explodes in production (where you pass an ORM row).

```python
# BAD. v1 idioms in a v2 project
class UserResponse(BaseModel):
    id: int
    email: str
    class Config:
        orm_mode = True # silently ignored in v2

        @validator("email") # deprecated
        def lower(cls, v): return v.lower()

user.dict() # DeprecationWarning
```

```python
# GOOD. v2 idioms
class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True) # required to read ORM rows
    id: int
    email: str

    @field_validator("email", mode="before")
    @classmethod
    def lower(cls, v: str) -> str: return v.lower()

user.model_dump()
```


### CODE-02  `@lru_cache` on an instance method (process-scoped memory leak)

**Why it matters.** `lru_cache` caches on the *function object*. When applied to a bound method, `self` becomes part of the cache key: the cache holds a strong reference to `self`, so every instance ever called is immortal for the lifetime of the process. In a web app that creates service instances, this is a slow leak that only shows up under load.

```python
# BAD. every UserService ever created is kept alive
class UserService:
    @lru_cache(maxsize=128)
    def is_admin(self, user_id: int) -> bool:
        return self._lookup(user_id)
```

```python
# GOOD. cache at module level, keyed on what actually varies
@lru_cache(maxsize=1024)
def _is_admin(user_id: int) -> bool:
    return User.objects.get(id=user_id).is_admin # or call from service

class UserService:
    def is_admin(self, user_id: int) -> bool:
        return _is_admin(user_id)

# OR per-instance with cached_property (cached on the instance, dies with it):
class UserService:
    @cached_property
    def admin_ids(self) -> frozenset[int]:
        return frozenset(self._fetch_admin_ids)
```


### CODE-03  `dict[str, Any]` tunneling through layers

**Why it matters.** AI tools love `dict[str, Any]` as the "shape I'm not sure about". Once it's in the signature, every downstream function has to either re-validate or guess, and the type system stops helping. Refactors become archaeology because nothing names what's in the dict. Parse at the boundary into a typed object; pass the typed object inward.

```python
# BAD. what's actually in this dict?
async def create_order(data: dict[str, Any]) -> dict[str, Any]:
    user_id = data.get("user_id") # might be a string, might be missing
    items = data.get("items", []) # list of what?. return {"id": order.id, "status": order.status}
```

```python
# GOOD. types name and validate the data at the boundary
class OrderCreate(BaseModel):
    user_id: UUID
    items: list[OrderItem]

class OrderResponse(BaseModel):
    id: UUID
    status: OrderStatus

async def create_order(data: OrderCreate) -> OrderResponse:...
```


### CODE-04: f-string logging of sensitive data (log injection + data leak)

**Why it matters.** AI tools emit `logger.info(f"User {user.email} logged in from {request.client.host}")` reflexively. Three problems: (1) the f-string is evaluated eagerly even if the log level is off, (2) emails / IPs / tokens end up in plaintext logs that ship to third parties, (3) user-controlled fields can inject newlines and forge log lines; structlog with bound context fixes all three.

```python
# BAD. eager evaluation, PII in logs, line-injection risk
logger.info(f"User {user.email} logged in from {request.client.host}")
logger.warning(f"Bad token: {token}") # ☠️ token in plaintext logs
```

```python
# GOOD. structured, lazy, scrubbable
log = structlog.get_logger()
log.info("user.login.succeeded", user_id=str(user.id), source_ip=request.client.host)
log.warning("auth.token.invalid", reason="signature_mismatch")  # no token value
```


### CODE-05  Catching `Exception` and re-raising as `HTTPException` (context destruction)

**Why it matters.** A service raises a `DuplicateEmailError`. A route catches `Exception` and raises `HTTPException(400, "Bad request")`. The original error type, message, and stack trace are gone. Sentry shows a generic 400 with no clue why. Let domain exceptions bubble and convert them in a centralized exception handler; see the **Error design** section in [`modern-python.md`](modern-python.md).

```python
# BAD. every error becomes "Bad request"; stack trace destroyed
@router.post("/users")
async def create(data: UserCreate):
    try:
        return await service.create(data)
    except Exception as e:
        raise HTTPException(400, "Bad request") # what was it? we'll never know
```

```python
# GOOD. typed domain errors mapped centrally
# service raises DuplicateEmailError, UserNotFoundError, etc.

@app.exception_handler(DuplicateEmailError)
async def _(_: Request, exc: DuplicateEmailError):
    return JSONResponse(status_code=409, content={"detail": str(exc), "code": "EMAIL_EXISTS"})

@router.post("/users")
async def create(data: UserCreate):
    return await service.create(data) # let it raise
```


### CODE-06  Async singleton race condition in lazy init

**Why it matters.** "Initialize the HTTP client on first use" reads cleanly. Under concurrent first requests, you race; two coroutines both see `_client is None`, both create one, only one wins, the loser's client is never closed → file-descriptor leak. The fix is an `asyncio.Lock`, or better, initialize in `lifespan`.

```python
# BAD. race on first concurrent requests
_client: httpx.AsyncClient | None = None

async def get_client() -> httpx.AsyncClient:
    global _client
    if _client is None:
        _client = httpx.AsyncClient(timeout=10) # two coroutines can both pass the check
        return _client
```

```python
# GOOD. initialize at startup in lifespan, inject via Depends
@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[State]:
    async with httpx.AsyncClient(timeout=10) as client:
        yield {"client": client}

def get_client(request: Request) -> httpx.AsyncClient:
    return request.state.client
```


### CODE-07  `subprocess.run(..., shell=True)` with user-controlled input

**Why it matters.** Command injection. AI generates this when it sees "run this shell command" and reaches for `shell=True` to handle pipes/expansions. Any user data in the string is now arbitrary code execution as the service user. Sentry has had high-severity advisories for this exact pattern in popular AI-tool-generated code.

```python
# BAD: RCE
result = subprocess.run(f"convert {user_filename} out.png", shell=True)
```

```python
# GOOD. argv list; no shell, no injection
result = subprocess.run(["convert", user_filename, "out.png"], check=True, timeout=30)
```


### CODE-08  Naive datetime arithmetic (timezone blindness)

**Why it matters.** `datetime.utcnow` (deprecated in 3.12) and `datetime.now` (local time!) return *naive* datetimes: no timezone. Mix them with timezone-aware datetimes from your DB and arithmetic silently does the wrong thing. The classic symptom: a "delete me in 24h" job that fires immediately or 5 hours late depending on the host's timezone.

```python
# BAD. naive datetime; arithmetic with aware DB column raises or wrong-results
expires = datetime.utcnow() + timedelta(hours=24)
# session.expires is timezone-aware; comparison raises TypeError silently in some paths
```

```python
# GOOD. timezone-aware UTC end to end
expires = datetime.now(UTC) + timedelta(hours=24)
# Pair with sqlalchemy.DateTime(timezone=True) on the column
```


### CODE-09  `# type: ignore` / `# noqa` as the first response to a typecheck error

**Why it matters.** Type errors usually mean you don't actually know what the value is. `# type: ignore` removes the warning, leaves the bug. AI tools sprinkle these because they make the tool output "clean" without thought. Every `# type: ignore` needs (a) an error code, and (b) a one-line comment explaining why the suppression is correct.

```python
# BAD
result = func # type: ignore
data = process # noqa
```

```python
# GOOD. fix the type
result: User = func
# OR scope the suppression and explain why
client: AsyncClient = mod.client # type: ignore[attr-defined] # legacy attr; remove after migration #234
```


### CODE-10  Boolean parameter flags

**Why it matters.** `send_email(user, urgent=True, retry=False, html=True)`; at the call site nobody knows which `True` does what. Worse, AI compounds the flags over edits until the function has 5 booleans and 2^5 untested combinations. Pick one of: separate functions, an enum, or `*` to force keyword-only calls.

```python
# BAD
send_email(user, True, False, True) # what do these mean?
def send_email(user, urgent=False, retry=True, html=False):...
# GOOD. keyword-only at minimum
def send_email(user, *, urgent=False, retry=True, html=False):...
send_email(user, urgent=True, html=True)

# BETTER. separate intent into separate functions
def send_email(user, body):...
def send_urgent_email(user, body, retry=True):...
# OR an enum
class Priority(StrEnum): NORMAL = "normal"; URGENT = "urgent"
def send_email(user, body, priority: Priority = Priority.NORMAL):...
```


## Architectural

### ARCH-01  Routes that query the database directly

**Why it matters.** When a route handler imports `Session` and runs `db.scalars(select(User)...)` directly, business logic is welded to HTTP. You cannot test the logic without a TestClient. Any change to "how we fetch users" requires touching every route. The route is doing three jobs (HTTP parsing, business rules, data access), and the route layer is exactly the load-bearing place where you can least afford the SRP violation.

```python
# BAD. route is router + service + repository
@router.get("/users/{user_id}/orders")
async def get_user_orders(user_id: int, db: AsyncSession = Depends(get_db)):
    user = await db.get(User, user_id)
    if not user:
        raise HTTPException(404)
        orders = await db.scalars(select(Order).where(Order.user_id == user_id, Order.status != "cancelled")
        )
        return [o for o in orders]
```

```python
# GOOD: thin route, testable service
# services/orders.py
async def get_active_orders_for_user(db: AsyncSession, user_id: int) -> list[Order]:
    if not await db.get(User, user_id):
        raise UserNotFoundError(user_id)
    stmt = select(Order).where(Order.user_id == user_id, Order.status != "cancelled")
    return list(await db.scalars(stmt))

# routers/orders.py
@router.get("/users/{user_id}/orders", response_model=list[OrderResponse])
async def get_user_orders(user_id: int, db: Annotated[AsyncSession, Depends(get_db)]):
    return await get_active_orders_for_user(db, user_id)
```


### ARCH-02  ORM models doubling as API response schemas

**Why it matters.** Returning a SQLAlchemy `User` directly (with `from_attributes=True`) exposes every column; including `hashed_password`, `internal_notes`, `deleted_at`. Adding an internal column silently leaks it. Worse: the API contract is now welded to the DB schema, and you can't rename a column without breaking clients.

```python
# BAD: ORM model is the API schema
class User(Base):
    id: Mapped[int] = mapped_column(primary_key=True)
    email: Mapped[str]
    hashed_password: Mapped[str] # leaked to API
    is_superuser: Mapped[bool] # privilege-escalation surface

@router.get("/users/{id}", response_model=User)
async def get_user(...):...
```

```python
# GOOD. separate response schema
class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: int
    email: str
    # hashed_password, is_superuser intentionally absent

@router.get("/users/{id}", response_model=UserResponse)
async def get_user(...):...
```


### ARCH-03  Service classes that accept `Request`

**Why it matters.** If your service signature is `async def create_user(self, request: Request)`, the service is permanently coupled to Starlette. You can't call it from a CLI, a Celery task, a test without a mock `Request`, or a gRPC handler. The Dependency Rule (Clean Architecture) says inner layers must not depend on outer layers; services live inside the route, not the other way around.

```python
# BAD
class UserService:
    async def create_user(self, request: Request) -> User:
        data = await request.json()
        ...
```

```python
# GOOD. service takes plain domain types; HTTP parsing stays at the route
class UserService:
    async def create_user(self, db: AsyncSession, email: str, password: str) -> User: ...

@router.post("/users", response_model=UserResponse)
async def register(
    data: UserCreate,
    service: Annotated[UserService, Depends(get_user_service)],
    db: Annotated[AsyncSession, Depends(get_db)],
):
    return await service.create_user(db, data.email, data.password)
```


### ARCH-04  Settings imported as a module-level global from everywhere

**Why it matters.** Every `from config import settings` at module top builds an invisible global dependency graph. Tests can't override settings cleanly (must monkey-patch a singleton). Secrets are read at import time; wrong import order causes a circular import. Use `@lru_cache`-wrapped `get_settings`, injected via `Depends`, overridable in tests via `app.dependency_overrides`.

```python
# BAD. evaluated at import time, can't override in tests
# database.py
from config import settings
engine = create_async_engine(settings.DATABASE_URL)  # frozen at import
```

```python
# GOOD. lazy, injectable, test-overridable
@lru_cache
def get_settings() -> Settings:
    return Settings()  # reads env once, cached

@router.get("/health")
async def health(settings: Annotated[Settings, Depends(get_settings)]):
    ...

# In tests:
app.dependency_overrides[get_settings] = lambda: Settings(DATABASE_URL="sqlite+aiosqlite:///:memory:")
```


### ARCH-05  `BackgroundTasks` for work that needs a queue

**Why it matters.** `BackgroundTask` runs *inside the Uvicorn worker*, after the response. No persistence (worker restart = task lost), no retry, no concurrency cap. For anything user-visible (email, webhook, provisioning), this is silent data loss. Use Arq / Dramatiq / Celery for anything you'd page someone about.

```python
# BAD. provisioning lost if the worker restarts mid-task
@router.post("/users")
async def create_user(data: UserCreate, background: BackgroundTasks,...):
    user = await service.create(db, data)
    background.add_task(provision_cloud_resources, user.id) # lost on crash
    return user
```

```python
# GOOD. durable queue
@router.post("/users")
async def create_user(data: UserCreate,...):
    user = await service.create(db, data)
    await arq_redis.enqueue_job("provision_cloud_resources", user.id)
    return user
```


### ARCH-06  Tests that mock the database (mock-heavy "integration" tests)

**Why it matters.** Mocking `session.scalars` means tests never run real SQL. A migration that renames a column, changes a join, or drops an index doesn't break the mocked test. You learn it broke in staging. The mocks become the maintenance burden, producing false confidence. Use SQLite in-memory for fast unit-ish tests and `testcontainers[postgres]` for full integration.

```python
# BAD
mock_db = MagicMock(spec=AsyncSession)
mock_db.scalars().return_value.first().return_value = User(id=1, email="a@b.com")
await user_service.get_by_email(mock_db, "a@b.com")
mock_db.scalars().assert_called_once()  # asserting on the mock, not on behavior
```

```python
# GOOD. real DB
@pytest_asyncio.fixture
async def db():
    engine = create_async_engine("sqlite+aiosqlite:///:memory:")
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)
    async with AsyncSession(engine) as session:
        yield session

async def test_get_user(db):
    db.add(User(email="a@b.com")); await db.commit()
    result = await user_service.get_by_email(db, "a@b.com")
    assert result.email == "a@b.com"
```


### ARCH-07  Authorization checked inline in every route

**Why it matters.** `if not current_user.is_admin: raise HTTPException(403)` copy-pasted in every admin route. AI tools omit it on a new route. That one missed line is a privilege-escalation vulnerability. Use a `require_admin` dependency, applied at the `APIRouter` level so it's structurally impossible to add a new admin route without it.

```python
# BAD. easy to forget
@router.delete("/users/{id}")
async def delete_user(id: int, current_user: Annotated[User, Depends(get_current_user)]):
    if not current_user.is_admin:  # AI tool omits this on the next admin route → CVE
        raise HTTPException(403)
```

```python
# GOOD. auth enforced at router construction; impossible to skip
async def require_admin(current_user: Annotated[User, Depends(get_current_user)]) -> User:
    if not current_user.is_admin:
        raise HTTPException(403, "Admin access required")
    return current_user

admin_router = APIRouter(prefix="/admin", dependencies=[Depends(require_admin)])

@admin_router.delete("/users/{id}")
async def delete_user(id: int):  # auth is structural
    ...
```


### ARCH-08  N+1 lazy loading in async (the silent throughput killer)

**Why it matters.** SQLAlchemy's default `relationship` is `lazy="select"`; accessing `order.user` triggers a separate SELECT per row. 100 orders → 101 queries. Worse in async: lazy loading from outside the original session raises `MissingGreenlet`. Invisible in dev (tiny data); catastrophic in prod.

```python
# BAD
class Order(Base):
    user: Mapped["User"] = relationship()  # default lazy="select"

async def summaries(db: AsyncSession):
    orders = (await db.scalars(select(Order))).all()
    return [{"id": o.id, "email": o.user.email} for o in orders]  # N+1
```

```python
# GOOD. eager-load, or set lazy="raise" to fail loudly in tests
async def summaries(db: AsyncSession):
    stmt = select(Order).options(selectinload(Order.user))
    orders = (await db.scalars(stmt)).all()
    return [{"id": o.id, "email": o.user.email} for o in orders]

# Defensive default for async services:
class Order(Base):
    user: Mapped["User"] = relationship(lazy="raise")  # any lazy access raises
```


### ARCH-09  Domain models inheriting from `BaseModel` (Pydantic as the domain layer)

**Why it matters.** Pydantic is a great I/O validator: It's not a great domain model. Using `BaseModel` for entities conflates validation rules (parse input) with business invariants (state transitions). You end up with a frozen model that can't transition state without re-validation, validators that mix coercion with business rules, and one class trying to serve both API and persistence concerns.

```python
# BAD. domain entity is a Pydantic model
class Order(BaseModel):
    id: UUID; status: OrderStatus; total: Decimal

    @field_validator("total")
    @classmethod
    def positive(cls, v): assert v > 0; return v

    def cancel(self): self.status = OrderStatus.CANCELLED # mutation requires frozen=False, sidesteps validation
```

```python
# GOOD. plain dataclass for the entity; Pydantic only at the API boundary
@dataclass
class Order:
    id: UUID
    status: OrderStatus
    total: Decimal

    def cancel(self) -> None:
        if self.status is not OrderStatus.PENDING:
            raise InvalidTransition(f"cannot cancel from {self.status}")
        self.status = OrderStatus.CANCELLED

# Pydantic only at the API boundary
class OrderResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: UUID
    status: OrderStatus
    total: Decimal
```


### ARCH-10  Flat `src/` with no bounded contexts

**Why it matters.** AI tools generate `src/models.py`, `src/services.py`, `src/routes.py`, `src/schemas.py` (every domain in every file). Every model imports every other model, every change has unbounded blast radius, and you can't carve out a subdomain into its own service without a rewrite. Organize by domain once you have ~5+, with shared cross-cutting concerns in `core/` and per-domain `router.py + service.py + models.py + schemas.py + exceptions.py + dependencies.py`. See [`project-structure.md`](project-structure.md).

```
# BAD. everything in everything
src/
models.py # User, Order, Product, Payment all here
services.py # 800 lines, every domain
routes.py # 60 endpoints
schemas.py
```

```
# GOOD. domain modules
src/
core/ # cross-cutting: config, db, exceptions, logging
auth/ # router.py service.py models.py schemas.py dependencies.py
orders/ # router.py service.py models.py schemas.py dependencies.py
payments/ # router.py service.py models.py schemas.py dependencies.py
main.py
```


## Detection Checklist

When reviewing a Python backend, run through these. Hitting 3+ usually means architectural review, not just lint cleanup.

**Code-level**
- [ ] Pydantic v1 names (`.dict`, `class Config`, `@validator`, `orm_mode`)
- [ ] `@lru_cache` on an instance method
- [ ] `dict[str, Any]` in service-layer signatures
- [ ] f-string log lines with PII or secrets
- [ ] `except Exception` in a route, re-raising `HTTPException`
- [ ] Lazy-init globals without a lock (or worse, with `global`)
- [ ] `subprocess.run(..., shell=True)` with any variable in the command
- [ ] `datetime.utcnow` / `datetime.now` (no tz)
- [ ] `# type: ignore` without an error code + reason
- [ ] Positional boolean flags in function calls

**Architectural**
- [ ] Routes calling `session.execute(...)` directly
- [ ] `response_model=` set to an ORM model class
- [ ] Service methods that accept `Request` / `Response`
- [ ] `from config import settings` at module top, across many modules
- [ ] `BackgroundTasks` for work whose loss would page someone
- [ ] `MagicMock(spec=AsyncSession)` in integration tests
- [ ] Inline `if not user.is_admin: raise HTTPException(403)` in routes
- [ ] `relationship` without `lazy="raise"` or explicit eager-load in async paths
- [ ] Domain entities inheriting `BaseModel`
- [ ] Flat `src/` with `models.py` / `services.py` / `routes.py` and no domain folders

**Bonus tells (pattern, not bug)**
- [ ] `requirements.txt` with no `uv.lock` / `poetry.lock`
- [ ] `from __future__ import annotations` in brand-new 3.14+ files (not wrong, just dated)
- [ ] GitHub Actions on stale major versions (`@v3` checkouts in 2026)
- [ ] Sync `requests` in `async def`
- [ ] `ORJSONResponse` in new code (deprecated since FastAPI 0.131)

## What AI-Free Backend Code Looks Like

- **Deliberate architecture**. layers, boundaries, one responsibility per module
- **Types name what's happening**. no `dict[str, Any]` tunneling, no string-typed enums, no untyped service signatures
- **Sparse comments**. code explains WHAT; rare comments explain WHY
- **Domain-specific abstractions**. `OrderService`, `PaymentGateway`, not `process_data`
- **One consistent style throughout**. one naming convention, one error pattern, one DI approach
- **No dead code**. no TODOs, no `pass` stubs, no commented-out blocks
- **Verified dependencies**. every import exists, every API call uses the currently-installed version
