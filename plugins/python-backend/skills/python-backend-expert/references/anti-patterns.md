# Anti-Patterns Catalog

Severity-labeled catalog of common Python backend anti-patterns. Each entry includes the pattern, why it's harmful, and the correct alternative.

## Architecture Anti-Patterns

### AP-01: Fat Controller (blocking)

Business logic in route handlers instead of services.

```python
# BAD -- controller does validation, DB queries, business logic, formatting
@post("/orders")
async def create_order(self, data: OrderRequest, db: AsyncSession):
    user = await db.execute(select(User).where(User.id == data.user_id))
    user = user.scalar_one_or_none()
    if not user:
        raise NotFoundException("User not found")
    if user.balance < data.total:
        raise ClientException("Insufficient funds")
    user.balance -= data.total
    order = OrderModel(user_id=user.id, total=data.total, status="pending")
    db.add(order)
    await db.commit()
    await send_email(user.email, "Order confirmed")
    return {"id": str(order.id), "status": order.status}

# GOOD -- controller delegates to service
@post("/orders")
async def create_order(self, data: OrderRequest, order_service: OrderService):
    order = await order_service.place_order(data.user_id, data.total)
    return OrderResponse.from_entity(order)
```

**Signal:** Handler >100 lines, imports ORM models, contains `if/else` business logic.

### AP-02: God Module (important)

Single file >500 lines handling multiple responsibilities.

**Signal:** A `services.py` that handles users, orders, notifications, and configuration.

**Fix:** Split by domain: `user_service.py`, `order_service.py`, `notification_service.py`.

### AP-03: Import Spaghetti (important)

Infrastructure imports in domain layer; circular dependencies.

```python
# BAD -- domain imports infrastructure
# domain/entities/user.py
from infrastructure.persistence.orm_models import UserModel  # Boundary violation!

# GOOD -- domain defines ports, infrastructure implements them
# domain/ports/user_repository.py
class IUserRepository(Protocol):
    async def get_by_id(self, id: UUID) -> User | None: ...
```

### AP-04: Anemic Service (nit)

Service that just proxies repository calls with no business logic.

```python
# BAD -- pointless indirection
class UserService:
    async def get_all(self) -> list[User]:
        return await self._repo.get_all()

# Better -- either add real logic or call repo directly from controller
```

## ORM Anti-Patterns

### AP-05: N+1 Query (blocking)

```python
# BAD -- 1 query for users + N queries for orders
users = (await session.execute(select(UserModel))).scalars()
for user in users:
    orders = user.orders  # Lazy load = separate query per user!

# GOOD -- eager load
users = (await session.execute(
    select(UserModel).options(selectinload(UserModel.orders))
)).scalars()
```

### AP-06: Naked SQLAlchemy in Handlers (important)

```python
# BAD -- raw DB queries in controller
@get("/users")
async def list_users(self, db: AsyncSession):
    result = await db.execute(select(UserModel).order_by(UserModel.name))
    return result.scalars().all()

# GOOD -- use repository
@get("/users")
async def list_users(self, user_service: UserService):
    return await user_service.list_all()
```

### AP-07: Session Leak (blocking)

```python
# BAD -- session not closed in background task
async def background_job(session: AsyncSession):
    result = await session.execute(...)
    # Session never closed! Connection pool exhaustion

# GOOD -- scoped session
async def background_job(session_factory: async_sessionmaker):
    async with session_factory() as session:
        result = await session.execute(...)
        await session.commit()
```

### AP-08: Missing expire_on_commit=False (blocking)

```python
# BAD -- default expire_on_commit=True in async
session_factory = async_sessionmaker(bind=engine)
# After commit, accessing attributes triggers sync lazy load -> MissingGreenlet!

# GOOD
session_factory = async_sessionmaker(bind=engine, expire_on_commit=False)
```

## Async Anti-Patterns

### AP-09: Blocking in Async (blocking)

```python
# BAD
import requests
async def fetch():
    return requests.get(url)  # Blocks entire event loop!

# GOOD
async def fetch():
    async with httpx.AsyncClient() as client:
        return await client.get(url)
```

### AP-10: Fire-and-Forget (important)

```python
# BAD -- exception silently lost
asyncio.create_task(process(data))

# GOOD -- error handling
task = asyncio.create_task(safe_process(data))
task.add_done_callback(lambda t: t.exception() and logger.error(...))
```

## Python Anti-Patterns

### AP-11: Bare Except (blocking)

```python
# BAD -- catches SystemExit, KeyboardInterrupt, GeneratorExit
try:
    process()
except:
    pass

# GOOD -- specific exceptions
try:
    process()
except ProcessingError as e:
    logger.warning("processing_failed", error=str(e))
```

### AP-12: Mutable Default Argument (blocking)

```python
# BAD -- shared across all calls
def create_user(roles: list[str] = []):
    roles.append("viewer")  # Mutates the default!
    return User(roles=roles)

# GOOD
def create_user(roles: list[str] | None = None):
    roles = roles or []
    roles.append("viewer")
    return User(roles=roles)
```

### AP-13: Stringly-Typed Configuration (important)

```python
# BAD -- dict[str, Any] everywhere
config = {"db_url": "...", "debug": "true", "max_conn": "10"}
if config["debug"] == "true":  # String comparison for bool

# GOOD -- Pydantic Settings
class Settings(BaseSettings):
    db_url: str
    debug: bool = False
    max_conn: int = 10
```

### AP-14: Global Mutable State (blocking)

```python
# BAD -- module-level mutable state shared across requests
_cache: dict[str, Any] = {}

async def get_user(user_id: str):
    if user_id in _cache:
        return _cache[user_id]
    user = await fetch_user(user_id)
    _cache[user_id] = user  # Race condition in async!
    return user

# GOOD -- use proper caching (Redis, TTL cache with locks)
```

### AP-15: Star Imports (important)

```python
# BAD -- namespace pollution, unclear origins
from models import *
from utils import *

# GOOD -- explicit imports
from domain.entities.user import User
from domain.exceptions import UserNotFoundError
```

### AP-16: Missing Type Hints (nit)

```python
# BAD -- untyped function
async def process(data, options=None):
    ...

# GOOD -- fully typed
async def process(data: ProcessRequest, options: ProcessOptions | None = None) -> ProcessResult:
    ...
```

### AP-17: dict[str, Any] Return Types (important)

```python
# BAD -- no type safety, no IDE support
async def get_user(id: UUID) -> dict[str, Any]:
    return {"id": str(id), "name": "John"}

# GOOD -- typed response
async def get_user(id: UUID) -> UserResponse:
    return UserResponse(id=id, name="John")
```

### AP-18: Print Debugging in Production (nit)

```python
# BAD
print(f"User created: {user.id}")

# GOOD
logger.info("user_created", user_id=str(user.id))
```

## Design Pattern Anti-Patterns

### AP-19: God Class (blocking)

```python
# BAD -- class does everything
class AppManager:
    async def create_user(self): ...
    async def send_email(self): ...
    async def process_payment(self): ...
    async def generate_report(self): ...
    async def sync_inventory(self): ...
    # 1000+ lines, 30+ methods
```

### AP-20: Premature Abstraction (nit)

```python
# BAD -- abstract factory for one implementation
class IUserRepositoryFactory(ABC):
    @abstractmethod
    def create(self) -> IUserRepository: ...

class SQLAlchemyUserRepositoryFactory(IUserRepositoryFactory):
    def create(self) -> IUserRepository:
        return UserRepository(self._session)

# GOOD -- just inject the repository directly until you need >1 implementation
```

### AP-21: Boolean Parameters (nit)

```python
# BAD -- unclear at call site
await process_order(order, True, False, True)

# GOOD -- use enums or keyword-only args
await process_order(order, priority=Priority.HIGH, notify=False, audit=True)
```

### AP-22: Exception as Flow Control (important)

```python
# BAD -- using exceptions for expected cases
try:
    user = await repo.get_by_email(email)
except UserNotFoundError:
    user = await repo.create(User(email=email))

# GOOD -- check first (for single-request contexts)
user = await repo.get_by_email(email)
if user is None:
    user = await repo.create(User(email=email))

# ALSO GOOD -- for concurrent contexts, use get_or_create / upsert
# The check-then-act pattern above has a TOCTOU race under concurrency.
# In high-contention scenarios, EAFP (try/except) or database-level
# upserts (INSERT ... ON CONFLICT) are more correct than check-first.
user = await repo.get_or_create(email=email)
```
