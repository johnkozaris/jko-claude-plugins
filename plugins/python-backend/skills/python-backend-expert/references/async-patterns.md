# Async Correctness

## Critical Rules

### 1. Never Block the Event Loop

```python
# BAD -- blocks the entire event loop
import time
import requests

async def handler():
    time.sleep(5)              # Blocks all concurrent requests
    resp = requests.get(url)   # Blocks all concurrent requests

# GOOD -- async alternatives
import asyncio
import httpx

async def handler():
    await asyncio.sleep(5)                    # Non-blocking
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)          # Non-blocking
    data = await asyncio.to_thread(json.loads, heavy)  # Offload CPU
```

Common blocking calls to watch for:
- `time.sleep()` -> `asyncio.sleep()`
- `requests.*` -> `httpx.AsyncClient` or `aiohttp`
- `open()` for file I/O -> `aiofiles.open()`
- `subprocess.run()` -> `asyncio.create_subprocess_exec()`
- `bcrypt.hash()` -> `await asyncio.to_thread(bcrypt.hash, ...)`

### 2. Session Scope in Async

```python
# BAD -- session escapes its scope
class BadService:
    def __init__(self, session: AsyncSession):
        self._session = session  # Stored, may be used after close

    async def background_task(self):
        await asyncio.sleep(10)
        await self._session.execute(...)  # Session may be closed!

# GOOD -- create session per operation
class GoodService:
    def __init__(self, session_factory: async_sessionmaker):
        self._sf = session_factory

    async def background_task(self):
        await asyncio.sleep(10)
        async with self._sf() as session:
            await session.execute(...)  # Fresh session
```

### 3. No Lazy Loading in Async

```python
# BAD -- lazy load triggers synchronous IO
async def get_user(session: AsyncSession) -> UserModel:
    user = await session.get(UserModel, user_id)
    print(user.orders)  # MissingGreenlet error or silent sync IO

# GOOD -- explicit eager loading
async def get_user(session: AsyncSession) -> UserModel:
    result = await session.execute(
        select(UserModel)
        .where(UserModel.id == user_id)
        .options(selectinload(UserModel.orders))
    )
    return result.scalar_one()
```

### 4. Task Cancellation Safety

```python
# BAD -- partial state on cancellation
async def transfer(from_acc, to_acc, amount):
    await debit(from_acc, amount)
    # If cancelled here, money is lost!
    await credit(to_acc, amount)

# GOOD -- atomic transaction
async def transfer(session: AsyncSession, from_acc, to_acc, amount):
    async with session.begin():
        await debit(session, from_acc, amount)
        await credit(session, to_acc, amount)
    # Commit or full rollback -- atomic
```

## Concurrency Patterns

### TaskGroup (Python 3.11+)

```python
# Run independent operations concurrently
async def get_dashboard(user_id: UUID):
    async with asyncio.TaskGroup() as tg:
        profile_task = tg.create_task(get_profile(user_id))
        orders_task = tg.create_task(get_orders(user_id))
        stats_task = tg.create_task(get_stats(user_id))

    return Dashboard(
        profile=profile_task.result(),
        orders=orders_task.result(),
        stats=stats_task.result(),
    )
```

### Semaphore for Rate Limiting

```python
semaphore = asyncio.Semaphore(10)

async def call_external(url: str):
    async with semaphore:
        async with httpx.AsyncClient() as client:
            return await client.get(url)
```

### Background Tasks

```python
# Litestar: use BackgroundTask
from litestar import Response
from litestar.background_tasks import BackgroundTask

@post("/users")
async def create_user(self, data: CreateUserRequest) -> Response:
    user = await user_service.create(data)
    return Response(
        content=UserResponse.from_entity(user),
        background=BackgroundTask(send_welcome_email, user.email),
    )
```

## Async Context Managers

```python
from contextlib import asynccontextmanager

@asynccontextmanager
async def managed_client(base_url: str):
    client = httpx.AsyncClient(base_url=base_url)
    try:
        yield client
    finally:
        await client.aclose()
```

## Async Anti-Patterns

### 1. Fire-and-Forget Without Error Handling

```python
# BAD -- exceptions silently lost
asyncio.create_task(send_notification(user_id))

# GOOD -- at minimum, log errors
async def safe_notify(user_id: UUID):
    try:
        await send_notification(user_id)
    except Exception:
        logger.exception("notification_failed", user_id=str(user_id))

asyncio.create_task(safe_notify(user_id))
```

### 2. Sequential Where Concurrent Is Possible

```python
# BAD -- sequential when independent
user = await get_user(user_id)
orders = await get_orders(user_id)
stats = await get_stats(user_id)

# GOOD -- concurrent with TaskGroup (3.11+)
async with asyncio.TaskGroup() as tg:
    user_task = tg.create_task(get_user(user_id))
    orders_task = tg.create_task(get_orders(user_id))
    stats_task = tg.create_task(get_stats(user_id))
user, orders, stats = user_task.result(), orders_task.result(), stats_task.result()
```

### 3. Sync-in-Async Wrappers

```python
# BAD -- wrapping sync code doesn't make it async
async def get_hash(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt())  # Still blocks!

# GOOD -- offload to thread pool
async def get_hash(password: str) -> str:
    return await asyncio.to_thread(
        bcrypt.hashpw, password.encode(), bcrypt.gensalt()
    )
```
