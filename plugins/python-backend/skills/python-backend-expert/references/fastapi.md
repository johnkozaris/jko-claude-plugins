# FastAPI Runtime & Footguns

FastAPI-specific patterns that AI assistants routinely get wrong. These are runtime-correctness issues; they pass tests and fail in production.

## Version Landscape (mid-2026)

Recent releases worth knowing when reading or upgrading a codebase:

| Version | Date | Why it matters |
| --- | --- | --- |
| **0.115** | Late 2024 | `Query`/`Header`/`Cookie`/`Path` accept `Annotated`-style lists; cleaner dependency syntax |
| **0.118** | 2025-09-29 | Official auth tutorial migrated from `passlib` (broke on Python 3.13) to **`pwdlib`** (Argon2 default) |
| **0.130** | 2026-02-22 | **2× JSON response perf** via pydantic-core Rust serializer when you use a Pydantic return type |
| **0.131** | 2026-02-22 | **`ORJSONResponse` / `UJSONResponse` deprecated**. return a Pydantic model, FastAPI Rust-serializes it |
| **0.135** | 2026-03 | Pydantic floor → ≥2.9.0; Starlette **1.0.0** released |
| **0.136** | 2026-04 | Free-threaded Python 3.14t support |

## Application Lifecycle: `lifespan`, not `on_event`

`@app.on_event("startup")` / `@app.on_event("shutdown")` are **deprecated**. They can't share state between startup and shutdown without globals. Use the `lifespan` async context manager; shared resources are scoped naturally, and the yielded dict becomes typed lifespan state (the ASGI standard, more portable than `app.state`).

```python
from contextlib import asynccontextmanager
from collections.abc import AsyncIterator
from typing import TypedDict, cast
from fastapi import FastAPI, Request
from httpx import AsyncClient

class State(TypedDict):
    client: AsyncClient

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[State]:
    # Startup: open pools, load models
    async with AsyncClient() as client:
        yield {"client": client}  # lifespan state → request.state.client
        # Shutdown runs after yield (client closed by the async with)

app = FastAPI(lifespan=lifespan)

@app.get("/")
async def root(request: Request):
    client = cast(AsyncClient, request.state.client)
    ...
```

It's typed, ASGI-portable, and avoids the "shared mutable namespace" coupling of `app.state`.

## The Threadpool Is Finite (40 threads)

FastAPI runs sync routes and sync dependencies in an anyio threadpool that defaults to **40 threads total**. Under a traffic spike with slow sync I/O, the pool saturates silently and requests queue: the app looks frozen.

```python
import anyio
from contextlib import asynccontextmanager

@asynccontextmanager
async def lifespan(app: FastAPI):
    limiter = anyio.to_thread.current_default_thread_limiter
    limiter.total_tokens = 100 # raise the ceiling when justified
    yield
```

Better than raising the ceiling: stop sending blocking work to the pool (use async clients), and make pure-compute dependencies `async def` so they don't consume a thread at all.

**Hidden footgun**: a non-async **dependency** in an async route also runs in the threadpool; even if the route itself is async. If `def http_client(request) -> AsyncClient: return request.state.client` looks zero-cost to you, it isn't; mark it `async def`.

## Calling Sync SDKs From Async Routes

When a library has no async client (boto3, a legacy SDK), don't call it directly inside `async def`; offload it:

```python
from fastapi.concurrency import run_in_threadpool

@app.get("/report")
async def report():
    data = await service.get_data()  # async I/O
    client = SyncReportClient()
    return await run_in_threadpool(client.render, data)  # yields the loop while it runs
```

## Response Serialization: Return Type vs `response_model`

Since FastAPI **0.130** (Feb 2026), declaring a Pydantic return type on the route activates the pydantic-core Rust JSON serializer. ~2× the throughput of the previous path. **`ORJSONResponse` and `UJSONResponse` are deprecated as of 0.131** because they're no longer faster than the default.

```python
# OPTIMAL: Pydantic return type → Rust serializer path
@app.post("/items/", status_code=201)
async def create_item(item: ItemIn) -> ItemOut:
    return await service.create(item)

# Use response_model= ONLY when input and output types must differ
# in a way the type checker can't express (e.g. excluding the password field):
@app.get("/me", response_model=UserOut, response_model_exclude_none=True)
async def me() -> Any:
    return current_user_orm_row
```

**`response_model` double-instantiation** (still common in AI-generated code): returning a Pydantic instance from a route that *also* declares `response_model=` of the **same class** constructs the model twice; once by you, once by FastAPI for validation/serialization.

```python
# ANTI-PATTERN: built twice
@app.get("/me", response_model=ProfileResponse)
async def me() -> ProfileResponse:
    return ProfileResponse(name="Alice")

# BETTER: return a dict or ORM row; response_model validates/serializes once
@app.get("/me", response_model=ProfileResponse)
async def me():
    return {"name": "Alice"}     # or: return orm_user (ConfigDict(from_attributes=True))
```

## Middleware: Avoid `BaseHTTPMiddleware` on the Hot Path

`BaseHTTPMiddleware` has the well-known Starlette issue [#1012](https://github.com/encode/starlette/issues/1012): when a slow client reads a streaming response, the internal asyncio queue grows without bound (not strictly a "leak" because there's no reference cycle, but unbounded memory growth under back-pressure). The issue has had several rounds of mitigation but stays open as of mid-2026 because the underlying back-pressure design hasn't fundamentally changed. Beyond that, `BaseHTTPMiddleware` carries a measurable **per-request performance penalty** vs pure ASGI middleware. Use it for occasional or admin-side concerns; write pure ASGI middleware for the hot path or anywhere you stream responses to potentially-slow clients.

```python
class TimingMiddleware:
    def __init__(self, app):
        self.app = app

    async def __call__(self, scope, receive, send):
        if scope["type"] != "http":
            return await self.app(scope, receive, send)
        start = time.perf_counter()
        async def send_wrapper(message):
            if message["type"] == "http.response.start":
                elapsed = time.perf_counter() - start
                message["headers"].append((b"x-process-time", f"{elapsed:.4f}".encode()))
            await send(message)
        await self.app(scope, receive, send_wrapper)
```

Note: `@app.middleware("http")` is sugar around `BaseHTTPMiddleware`; same caveats apply.

## Background Work: `BackgroundTasks` vs a Real Queue

`BackgroundTasks` runs *after the response*, in the same worker process, with **no retry, no visibility, no scheduling**. If the worker restarts or OOMs, the task is gone.

| Use `BackgroundTasks` | Use Celery / Arq / Dramatiq |
| --- | --- |
| Task < 1s, in-process | Seconds-to-minutes work |
| Failure can be silently dropped (fire-and-forget email, audit row) | You need retries / dead-letter |
| No scheduling needed | cron / ETA / rate limiting |
| No CPU heaviness | CPU-bound or separate pool |
| Single-machine deployment | Distributed work |

**Picking a queue** (community sentiment, mid-2026):
- **Arq**. async-native, Redis-backed; the easy fit for async FastAPI services with simple needs.
- **Dramatiq**. fast, batteries-included, prometheus metrics built-in; the modern "Celery without the warts".
- **Celery**. still the most-deployed, but the operational complexity and sync-first design make it a heavy choice; pick when you actually need its breadth (`beat`, custom routers, multi-broker).

Rule of thumb: **"If you'd page someone when the task is lost, it doesn't belong in `BackgroundTasks`."**

## Validation in Dependencies

Dependencies aren't just for injection: they're the right place for DB/ownership checks that should short-circuit with an HTTP error. Results are cached per request, so chaining is cheap.

```python
async def valid_post_id(post_id: UUID) -> Post:
    post = await service.get_by_id(post_id)
    if post is None:
        raise PostNotFound  # 404
    return post

async def valid_owned_post(
    post: Annotated[Post, Depends(valid_post_id)],
    user: Annotated[User, Depends(current_user)],
) -> Post:
    if post.owner_id != user.id:
        raise NotOwner  # 403
    return post

@router.put("/posts/{post_id}", response_model=PostResponse)
async def update_post(
    data: PostUpdate,
    post: Annotated[Post, Depends(valid_owned_post)],
):
    return await service.update(post.id, data)
```

## Security & Ops Footguns

**DO**: Hide API docs outside dev/staging.

```python
app_kwargs = {"title": "My API"}
if settings.ENVIRONMENT not in {"local", "staging"}:
    app_kwargs["openapi_url"] = None  # disables /docs and /redoc
app = FastAPI(**app_kwargs)
```

**DO**: Use **PyJWT** (`import jwt`). FastAPI's own security tutorial moved off `python-jose` years ago. PyJWT is actively maintained and the dependency floor in major templates.
**DO**: Use **pwdlib** with Argon2, not `passlib`. FastAPI's own auth tutorial moved to pwdlib in 0.118. `pwdlib` is async-friendly and has an Argon2 default.
**DO**: Run with `uvicorn[standard]` (pulls `uvloop` + `httptools`) as the production default; it's the battle-tested choice.
**Be aware**: FastAPI 0.132 made strict `Content-Type` checking the default for JSON requests. Clients without `Content-Type: application/json` start getting 415s on upgrade. The right fix is to fix the clients. Use `strict_content_type=False` only as a temporary escape hatch while you migrate them; never as a permanent setting.
**DON'T**: Use sync `requests` or a sync DB session anywhere in an `async def` path.
**DON'T**: Use `ORJSONResponse` / `UJSONResponse` in new code (deprecated 0.131); return a Pydantic model and let FastAPI's Rust serializer take it.

## WebSockets

Use `async for` over the socket rather than `while True`: it handles disconnects automatically:

```python
@app.websocket("/ws")
async def ws(websocket: WebSocket):
    await websocket.accept
    async for message in websocket.iter_text:
        await websocket.send_text(f"echo: {message}")
```

For websocket workloads in general, prefer `uvicorn[standard]`: it's well-tested and ships with the optimized `wsproto`/`websockets` paths: Only investigate alternatives if you have a measured throughput problem.
