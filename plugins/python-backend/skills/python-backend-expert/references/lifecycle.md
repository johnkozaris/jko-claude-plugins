# Backend Lifecycle

How code flows from process boot to shutdown, and where it usually breaks. Most production incidents happen at lifecycle edges (startup races, in-flight requests during shutdown, lost background tasks, stale connections after DB failover). Get these right and the rest of the architecture has room to be merely good.

## App boot: startup → ready

The order matters. Wrong env values should fail before any I/O. DB pool warmup should happen before traffic is accepted. Health-check state should flip to ready only after warmup succeeds.

```python
class Settings(BaseSettings):
    database_url: str
    redis_url: str

settings = Settings()  # 1. fails at import on bad env

# 2. pool object exists, not connected yet
engine = create_async_engine(
    settings.database_url,
    pool_pre_ping=True,
    pool_recycle=1800,
)

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[State]:
    async with httpx.AsyncClient(timeout=10.0) as client:
        async with engine.connect() as conn:  # 3. prove DB is reachable
            await conn.execute(text("SELECT 1"))
        app.state.ready = True                # 4. only after warmup succeeds
        yield {"client": client}              # 5. serve traffic
        app.state.ready = False               # 6. flip not-ready first on shutdown
        await engine.dispose()                # 7. teardown in reverse order

app = FastAPI(lifespan=lifespan)
```

**Do** inside lifespan: pool warmup, lookup-table loads, ML weight loading, signal handler registration.
**Don't** inside lifespan: run database migrations (do them in a separate one-shot job; a bad migration should crash that job, not your app). Block on a network call without a timeout. Fetch secrets that need retries (do that in a startup script or sidecar).

## Liveness vs readiness

Different jobs. Failing one should not trigger the other.

| Probe | What Kubernetes does on failure | What to check |
| --- | --- | --- |
| Liveness | Restart the container | Process stuck (deadlock, OOM, frozen event loop). Trivial check. |
| Readiness | Remove pod from Service endpoints | Dependencies reachable, startup complete. May touch the DB. |
| Startup | Block the other two until pass | Slow one-time init finished. |

```python
@app.get("/healthz/live", include_in_schema=False)
async def liveness():
    return {"status": "ok"} # just proves the event loop ticks

@app.get("/healthz/ready", include_in_schema=False)
async def readiness(request: Request):
    if not request.app.state.ready:
        raise HTTPException(503, "not ready")
        try:
            async with engine.connect as conn:
                await conn.execute(text("SELECT 1"))
            except Exception:
                raise HTTPException(503, "db unreachable")
                return {"status": "ready"}
```

Exclude probe paths from tracing or they'll dominate span volume: `OTEL_PYTHON_FASTAPI_EXCLUDED_URLS="healthz"`.

If liveness hits the DB and the DB is slow, Kubernetes kills a healthy pod. That's a self-inflicted outage. Keep liveness trivial.

## Graceful shutdown

Without graceful shutdown every rolling deploy turns in-flight requests into 5xxs: A deploy-correlated error spike is the telltale.

What Uvicorn ≥ 0.18 does on SIGTERM:
1. Stop accepting new connections.
2. Wait for in-flight requests up to `--timeout-graceful-shutdown` (default 30 s).
3. Run lifespan teardown.
4. Exit.

What usually breaks:
- **K8s `terminationGracePeriodSeconds` < your drain budget**: pod SIGKILLed mid-drain. Keep `terminationGracePeriodSeconds ≥ --timeout-graceful-shutdown + buffer`.
- **`asyncio.create_task(...)` without a strong reference**: the task can be garbage-collected mid-flight. Track it in a set, or use a real queue.
- **Gunicorn `--worker-tmp-dir` on slow disk**: in Docker on EBS/overlay FS the Gunicorn heartbeat blocks. Fix with `--worker-tmp-dir /dev/shm`.

```bash
gunicorn app.main:app \
 --workers 2 \
 --worker-class uvicorn.workers.UvicornWorker \
 --worker-tmp-dir /dev/shm \
 --graceful-timeout 30 \
 --timeout 60
```

## Worker recycling

Python processes accumulate memory (cycles, C extension leaks, fragmentation). `--max-requests` with `--max-requests-jitter` bounds blast radius and prevents synchronous restart storms:

```bash
gunicorn app.main:app \
 --max-requests 1000 \
 --max-requests-jitter 100 \ # without jitter, all workers restart at once
 --workers 4 \
 --worker-class uvicorn.workers.UvicornWorker
```

Without jitter, all workers hit the limit on the same request and restart simultaneously: a brief zero-worker window. Most relevant for services that import large libraries (numpy, torch) or use ORMs with identity maps and >24h uptimes.

## Request lifecycle

Order inside FastAPI:

```
1. Uvicorn: ASGI receive (headers + body)
2. Middleware stack, outermost first (CORS → TrustedHost → custom → GZip)
3. Starlette router: match path (404 here)
4. FastAPI dependency graph: depth-first, cached per request
5. Path operation (your async def)
6. response_model serialization (pydantic-core, Rust on 0.130+)
7. Middleware stack on the way out, reverse order
8. Uvicorn: ASGI send
```

`HTTPException` raised anywhere becomes JSON via FastAPI's default handler. Domain exceptions raised in step 5 are matched against `@app.exception_handler(...)` registrations; map `DuplicateEmailError → 409` there.

## The `yield` dependency pattern (and where it leaks)

```python
AsyncSessionLocal = async_sessionmaker(engine, expire_on_commit=False)

async def get_db() -> AsyncGenerator[AsyncSession, None]:
    async with AsyncSessionLocal() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise
```

Three traps:
- `session.rollback` itself raises: the original exception is replaced. Log the rollback failure separately if you care.
- `BackgroundTasks` runs after the response but before teardown completes. Do not pass `session` to a `BackgroundTask`; it's already closed by the time the task fires.
- A sync dependency invoked via `run_in_executor` does not propagate `contextvars`. Make any dependency that touches contextvars `async def`.

## Request context with `contextvars` (not `threading.local`)

`threading.local` is per-OS-thread. In an async app thousands of requests share one thread; `threading.local` bleeds data across requests. Use `contextvars`.

```python
import structlog, structlog.contextvars
import uuid

class RequestContextMiddleware:
    def __init__(self, app): self.app = app
    async def __call__(self, scope, receive, send):
        if scope["type"] == "http":
            structlog.contextvars.clear_contextvars
            structlog.contextvars.bind_contextvars(request_id=str(uuid.uuid4()),
            path=scope["path"],
            method=scope["method"],
            )
            await self.app(scope, receive, send)

structlog.configure(processors=[
structlog.contextvars.merge_contextvars,
structlog.processors.TimeStamper(fmt="iso"),
structlog.processors.JSONRenderer,
])
```

Bindings set in sync code don't appear in async log calls (and vice versa) when the sync ran on the executor thread. Always bind from async middleware.

## Cancellation on client disconnect

When a client disconnects, Uvicorn cancels the ASGI `receive` coroutine. That `CancelledError` propagates into your request coroutine **only at the next `await` point**. Pure CPU code or anything in `run_in_executor` won't be cancelled until it returns.

For long-running endpoints, check explicitly and bound with `asyncio.timeout`:

```python
@app.get("/slow")
async def slow_endpoint(request: Request, db: Annotated[AsyncSession, Depends(get_db)]):
    if await request.is_disconnected():
        raise HTTPException(499, "client disconnected")
        async with asyncio.timeout(30):
            return await db.execute(expensive_query)
```

## Timeouts: outer → inner

```
Edge proxy (nginx/Envoy upstream_response_timeout, typical 60s)
└── Gunicorn --timeout (default 30s; HARD SIGKILL on breach)
└── asyncio.timeout in your endpoint (set SMALLER than Gunicorn's)
└── httpx timeout (connect=5, read=30, total=60)
└── DB statement timeout (server-side)
```

`asyncio.timeout` should always be smaller than Gunicorn's hard `--timeout` so you can return a clean 504 before the worker is killed.

## Retries and idempotency

Retries live in the **outbound HTTP client layer**, never inside DB transactions. Always pair with idempotency keys.

```python
from tenacity import retry, stop_after_attempt, wait_exponential_jitter, retry_if_exception_type

@retry(stop=stop_after_attempt(3),
wait=wait_exponential_jitter(initial=0.5, max=5),
retry=retry_if_exception_type(httpx.TransportError),
reraise=True,
)
async def call_payment_service(payload: dict) -> dict:
    r = await http_client.post("/charge", json=payload)
    r.raise_for_status
    return r.json()
```

Idempotency keys (Stripe-style): the client generates a UUID, sends it in `Idempotency-Key:`, the server stores `(key, user_id) → response_body` in Redis with a TTL. On duplicate, return the cached response without re-executing.

```python
@app.post("/payments")
async def create_payment(body: PaymentRequest,
idempotency_key: Annotated[str, Header],
user: CurrentUser,
redis: Annotated[Redis, Depends(get_redis)],
):
    cache_key = f"idem:{user.id}:{idempotency_key}" # scope to user, never global
    if cached := await redis.get(cache_key):
        return JSONResponse(json.loads(cached), status_code=200)
        result = await process_payment(body)
        await redis.setex(cache_key, 86400, result.model_dump_json())
        return result
```

## Resources: pools and clients

- **DB pool**: preconnect in lifespan with `SELECT 1`, `pool_pre_ping=True`, `pool_recycle=1800`. Sizing: `pool_size = expected concurrent DB queries per worker` (usually 10-20).
- **`httpx.AsyncClient`**: one per process, opened in lifespan: Never per request: each instantiation creates a fresh connection pool. At 100 RPS to an external API that's 100 TLS handshakes a second.
- **Per-downstream bulkhead**: one `httpx.AsyncClient` per upstream service, so a slow Stripe can't exhaust the pool that talks to user-service.
- **Redis**: `socket_connect_timeout=2`, `retry=Retry(ExponentialBackoff, retries=3)` to bound failover impact.

## Failure modes that bite in production

- **Unhandled exceptions**: register a catch-all handler that logs structured context, captures to Sentry/Logfire, and returns RFC 7807 problem+json: Never let a stack trace reach the client.
- **`RequestValidationError`**: override the default handler to return a stable client-facing shape.
- **`BackgroundTasks` silent loss**: anything user-visible goes through a real queue (Arq, Dramatiq, Celery). `BackgroundTasks` is for in-process drop-tolerant work only (cache warmups, nice-to-have audit rows).
- **Cascading failures**: one slow downstream backs up the worker. Use per-downstream `asyncio.Semaphore(N)` bulkheading.

## Deploys

- **Zero-downtime requires three things lined up**: graceful shutdown drains in-flight requests, `terminationGracePeriodSeconds` ≥ drain budget, schema-compatible migrations (old and new pods serve traffic at the same time during a rolling deploy).
- **DB migrations are expand → backfill → contract**: add the new column nullable, deploy code that writes both, backfill, deploy code that reads new, then drop the old. Multiple deploys instead of one: That's the cost of zero-downtime.
- **Config reload**: most services don't. Restart on rotation. For runtime knobs use a feature-flag service (OpenFeature, Unleash, LaunchDarkly), not env vars.

## Observability essentials

**Every request emits**:

```
request.start {request_id, method, path, user_id?, client_ip}
request.complete {request_id, status, duration_ms}
request.error {request_id, exc_type, exc_message_safe}
```

```python
@app.middleware("http")
async def access_log(request: Request, call_next):
    start = time.perf_counter()
    log.info("request.start", method=request.method, path=request.url.path)
    try:
        response = await call_next(request)
        log.info("request.complete",
        status=response.status_code,
        duration_ms=round((time.perf_counter() - start) * 1000, 2))
        return response
    except Exception:
        log.exception("request.error",
        duration_ms=round((time.perf_counter() - start) * 1000, 2))
        raise
```

**`/metrics` minimum set**: `http_requests_total{method,path,status}`, `http_request_duration_seconds` (histogram), `inflight_requests` (gauge), `db_pool_checked_out` (gauge), `db_query_duration_seconds` (histogram), `bg_task_queue_size` (gauge), `process_resident_memory_bytes` (gauge).

**Tracing**: install `opentelemetry-instrumentation-fastapi` + `-sqlalchemy` + `-httpx` + `-redis`: They auto-propagate `traceparent`. Set `OTEL_PYTHON_FASTAPI_EXCLUDED_URLS="healthz,metrics"`. Sample at the SDK (`TraceIdRatioBased(0.1)`), not at the collector.

## Quick reference

| Symptom | Look at |
| --- | --- |
| Service won't start | env validation, lifespan exceptions, pool warmup |
| 5xx spike on every deploy | graceful shutdown, terminationGracePeriodSeconds, migration compatibility |
| Slow under load | timeout layering, per-downstream bulkhead, pool sizing |
| Memory creep over days | worker recycling, `httpx`/`Redis` singletons, memray |
| "Where's my email?" | `BackgroundTasks` silent loss; move to a queue |
| Logs missing request_id | bind contextvars from async middleware, not sync |
| Failures only at deploy time | startup ordering; readiness probe touches wrong dependency |
| Double-charges / dup writes | idempotency keys (always pair with retries) |
