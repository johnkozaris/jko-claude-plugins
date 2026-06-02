# Async Correctness

## Critical Rules

### 1. Never Block the Event Loop

```python
# BAD: blocks the entire event loop
import time
import requests

async def handler():
    time.sleep(5)               # Blocks all concurrent requests
    resp = requests.get(url)    # Blocks all concurrent requests

# GOOD: async alternatives
import asyncio
import httpx

async def handler():
    await asyncio.sleep(5)      # Non-blocking
    async with httpx.AsyncClient() as client:
        resp = await client.get(url)                          # Non-blocking
        data = await asyncio.to_thread(json.loads, heavy)     # Offload CPU
```

Common blocking calls to watch for:
- `time.sleep` -> `asyncio.sleep`
- `requests.*` -> `httpx.AsyncClient` or `aiohttp`
- `open` for file I/O -> `aiofiles.open`
- `subprocess.run` -> `asyncio.create_subprocess_exec`
- `bcrypt.hash` -> `await asyncio.to_thread(bcrypt.hash,...)`

### Choosing Route Color (async def vs def)

FastAPI runs `async def` routes on the event loop and `def` routes in a 40-thread pool. Pick by what the route actually does:

| Route does this | Use |
| --- | --- |
| `await`able non-blocking I/O (httpx, AsyncSession) | `async def` |
| Blocking I/O with no async client | `def` (runs in the threadpool, loop stays free) |
| Mix of async I/O and a blocking call | `async def` + `run_in_threadpool(sync_fn,...)` |
| CPU-bound > ~50ms | Worker process (Celery/Arq/RQ) or `ProcessPoolExecutor`: the GIL means threads don't help |

```python
# Blocking SDK from an async route: offload it, don't call it directly
from fastapi.concurrency import run_in_threadpool

@router.get("/report")
async def report(service: ReportServiceDep):
    data = await service.get_data()  # async I/O
    return await run_in_threadpool(SyncRenderer.render, data)
```

`PYTHONASYNCIODEBUG=1` in development prints a warning when any coroutine runs >100ms: a cheap way to find blockers before production.

### 2. Session Scope in Async

```python
# BAD: session escapes its scope
class BadService:
    def __init__(self, session: AsyncSession):
        self._session = session  # Stored, may be used after close

    async def background_task(self):
        await asyncio.sleep(10)
        await self._session.execute(...)  # Session may be closed

# GOOD: create session per operation
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
# BAD: lazy load triggers synchronous IO
async def get_user(session: AsyncSession) -> UserModel:
    user = await session.get(UserModel, user_id)
    print(user.orders)  # MissingGreenlet error or silent sync IO

# GOOD: explicit eager loading
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
# BAD: partial state on cancellation
async def transfer(from_acc, to_acc, amount):
    await debit(from_acc, amount)
    # If cancelled here, money is lost
    await credit(to_acc, amount)

# GOOD: atomic transaction
async def transfer(session: AsyncSession, from_acc, to_acc, amount):
    async with session.begin():
        await debit(session, from_acc, amount)
        await credit(session, to_acc, amount)
        # Commit or full rollback (atomic)
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
# FastAPI: BackgroundTasks runs AFTER the response, in-process, no retry
from fastapi import BackgroundTasks

@router.post("/users", response_model=UserResponse, status_code=201)
async def create_user(data: CreateUserRequest, service: UserServiceDep, bg: BackgroundTasks):
    user = await service.create(data.email, data.password)
    bg.add_task(send_welcome_email, user.email)  # fire-and-forget, OK to drop
    return user
```

`BackgroundTasks` has no retry, no visibility, and dies with the worker. Use it only for work you can afford to lose (a welcome email, an audit row). For anything you'd page on (payments, provisioning, anything needing retries or scheduling) use a real queue (Celery/Arq/RQ). See the decision matrix in the FastAPI reference.

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
# BAD: exceptions silently lost
asyncio.create_task(send_notification(user_id))

# GOOD: at minimum, log errors
async def safe_notify(user_id: UUID):
    try:
        await send_notification(user_id)
    except Exception:
        logger.exception("notification_failed", user_id=str(user_id))

asyncio.create_task(safe_notify(user_id))
```

### 2. Sequential Where Concurrent Is Possible

```python
# BAD: sequential when independent
user = await get_user(user_id)
orders = await get_orders(user_id)
stats = await get_stats(user_id)

# GOOD: concurrent with TaskGroup (3.11+)
async with asyncio.TaskGroup() as tg:
    user_task = tg.create_task(get_user(user_id))
    orders_task = tg.create_task(get_orders(user_id))
    stats_task = tg.create_task(get_stats(user_id))
user, orders, stats = user_task.result(), orders_task.result(), stats_task.result()
```

### 3. Sync-in-Async Wrappers

```python
# BAD: wrapping sync code doesn't make it async
async def get_hash(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt())  # Still blocks

# GOOD: offload to thread pool
async def get_hash(password: str) -> str:
    return await asyncio.to_thread(bcrypt.hashpw, password.encode(), bcrypt.gensalt())
```

### 4. `create_task` Without a Strong Reference

A task created by `asyncio.create_task` is held only weakly by the loop. If nothing keeps a strong reference, the GC may collect it mid-flight: the task simply vanishes.

```python
# BAD: task may be garbage-collected
asyncio.create_task(bg_work())

# GOOD: keep a reference set
BACKGROUND_TASKS: set[asyncio.Task[None]] = set()

def spawn(coro):
    t = asyncio.create_task(coro)
    BACKGROUND_TASKS.add(t)
    t.add_done_callback(BACKGROUND_TASKS.discard)
    return t

spawn(bg_work())
```

For anything more than fire-and-forget, prefer `asyncio.TaskGroup`: it owns the references for you and propagates errors.

## Timeouts and Cancellation

### `asyncio.timeout` (3.11+) over `wait_for`

```python
# MODERN (3.11+): context manager; cleaner; reschedulable
async with asyncio.timeout(5):
    result = await something_slow()

# Reschedule a timeout mid-block:
async with asyncio.timeout(5) as cm:
    cm.reschedule(asyncio.get_running_loop().time() + 30)
    await something_slower()

# LEGACY: still works, but less ergonomic
result = await asyncio.wait_for(something_slow(), timeout=5)
```

`asyncio.timeout` raises the builtin `TimeoutError` (`asyncio.TimeoutError` is now an alias). Inside a `TaskGroup`, that timeout becomes part of an `ExceptionGroup`.

### ExceptionGroup / `except*` (PEP 654)

`TaskGroup` always wraps task failures in an `ExceptionGroup`. Match by type with `except*`:

```python
try:
    async with asyncio.TaskGroup() as tg:
        tg.create_task(call_a())
        tg.create_task(call_b())
        tg.create_task(call_c())
except* HTTPError as eg:
    log.warning("upstream_failures", count=len(eg.exceptions))
except* TimeoutError as eg:
    log.warning("upstream_timeouts", count=len(eg.exceptions))
```

The whole asyncio / anyio / trio ecosystem has converged on "exceptions from concurrent tasks are always wrapped in `ExceptionGroup`". Write handlers accordingly. (`anyio` 4 changed its default to match; `trio` defaults `strict_exception_groups=True`.)

## Event Loop Setup (uvloop, post-policy world)

The asyncio **event-loop policy system is deprecated in 3.14 and scheduled for removal in 3.16**. Don't write `asyncio.set_event_loop_policy(...)` or `uvloop.install()` in new code.

```python
# MODERN (preferred)
import uvloop
uvloop.run(main())  # uvloop ≥ 0.18

# OR: explicit loop_factory through asyncio.run
import asyncio, uvloop
asyncio.run(main(), loop_factory=uvloop.new_event_loop)  # Python 3.12+

# OBSOLETE: DO NOT USE in new code
# uvloop.install(); asyncio.run(main())
# asyncio.set_event_loop_policy(uvloop.EventLoopPolicy())
```

In FastAPI/Starlette deployments, `uvicorn[standard] --loop uvloop` (or Uvicorn's auto-detection) sets this up; the application code doesn't touch it. The rule above matters for scripts, workers, and tests.

## Production Introspection (Python 3.14+)

PEP 768 added a safe external debugger interface, and 3.14 ships an asyncio-aware live introspection CLI built on top of it:

```bash
python -m asyncio ps <pid> # flat table: every task with its current coroutine + stack
python -m asyncio pstree <pid> # tree: who awaited whom
```

These attach without stopping the process; use them when a worker is wedged. Combine with `py-spy record --pid` for time attribution. See [`performance.md`](performance.md) for the rest of the profiler matrix.

## Concurrency Choice Quick Reference

| Work shape | Use |
| --- | --- |
| Single I/O call | `await` it |
| Independent I/O calls | `asyncio.TaskGroup` |
| Calling a sync function from `async def` | `await asyncio.to_thread(...)` or `await run_in_threadpool(...)` |
| Time-bounded operation | `async with asyncio.timeout(secs):` |
| CPU-bound > ~50 ms | `ProcessPoolExecutor`, or `InterpreterPoolExecutor` (3.14+; see [`performance.md`](performance.md)) |
| Rate-limited concurrent fan-out | `asyncio.Semaphore` |
| Library that should support trio too | use `anyio` for primitives |
| Application targeting asyncio only | use `asyncio` directly |
