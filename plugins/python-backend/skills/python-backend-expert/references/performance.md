# Performance: FastAPI / Python Backend

Performance for a FastAPI service is mostly about not blocking the event loop, choosing pool sizes that match the workload, and using the Rust-backed parts of the ecosystem (pydantic-core, orjson, asyncpg) where they help. Server choice is a smaller knob than people expect; once the code is async-correct, the gap between a well-tuned Uvicorn and any newer entrant shrinks dramatically under realistic I/O-bound load.

Pair this reference with [`async-patterns.md`](async-patterns.md) (blocking taxonomy) and [`sqlalchemy.md`](sqlalchemy.md) (ORM specifics).

## ASGI Server: Tune Uvicorn First

For production FastAPI in 2026, **Uvicorn with `uvloop` + `httptools`** is the default: It's the most battle-tested combination, what FastAPI's own documentation recommends, what every major template uses, and what the FastAPI/Starlette maintainer team operates against. `uvicorn[standard]` pulls both extras automatically: The non-obvious failure mode is that Uvicorn's *other* HTTP parser. `h11`, pure Python; is **significantly slower** than `httptools`. Uvicorn's own docs note: *"The httptools implementation provides greater performance, but is not compatible with PyPy."* If you `pip install uvicorn` without `[standard]`, you silently get the slow parser. Confirm with `uvicorn --http httptools --loop uvloop` in your start command (or just use `uvicorn[standard]`).

**DO**: Use `uvicorn[standard]` (or pin `httptools` + `uvloop` directly). Use `--http httptools --loop uvloop` in your start command if you want to be explicit.
**DO**: Set workers = vCPUs (give or take), then measure. With Gunicorn supervising (`gunicorn -k uvicorn.workers.UvicornWorker -w N`) you get graceful reload, preload, and the same async loop underneath.
**DON'T**: Use Hypercorn for throughput-sensitive services: it's the slowest commonly-recommended ASGI server by a wide margin.
**DON'T**: Run plain `uvicorn` without `[standard]` in production; you're leaving a multi-× perf factor on the floor because of the parser choice.

> Default to Uvicorn for FastAPI in production. Rust-based ASGI servers (Granian and others) show meaningful wins on raw GET throughput and HTTP/2 in benchmarks, but the gap shrinks to within noise at typical I/O-bound loads, every middleware/integration in the FastAPI ecosystem is exercised against Uvicorn, and "battle-tested under real production" matters more than benchmark RPS for most services. Evaluate alternatives only after measuring Uvicorn-on-httptools as the actual bottleneck.

## JSON Serialization

orjson and msgspec are the Rust/C-backed fast paths; stdlib `json` is too slow for production hot paths. FastAPI 0.130 (Feb 2026) added a Rust serialization path **inside FastAPI itself** via pydantic-core, which is why `ORJSONResponse` was deprecated in 0.131: it's no longer faster than the default when you return a typed Pydantic model.

| Library | Typical use | Why |
| --- | --- | --- |
| **pydantic-core** (FastAPI 0.130+ default) | FastAPI responses | Rust parse/serialize when you return a typed Pydantic model; ~2× the old path; no extra dep |
| **orjson** | Anywhere you'd reach for `json.dumps/loads` | Rust; drop-in; native `datetime`/`UUID`/`dataclass`/`numpy`; strict UTF-8 |
| **msgspec.Struct** (optional, hot paths) | Internal serialization with known schemas | Decode + validate in one Rust pass; faster than orjson decode-then-validate on typed data |
| ujson |. | Older C parser; orjson is faster and stricter, no reason to pick ujson today |
| stdlib `json` | Cold paths, configs | Acceptable when speed doesn't matter; never in a hot loop |

**DO**: Return a Pydantic model with a typed return annotation (`-> ItemOut`). FastAPI 0.130+ uses pydantic-core's Rust serializer automatically.
**DO**: Use `orjson` anywhere outside FastAPI's response path where you'd otherwise call `json.dumps/loads` on non-trivial data.
**DO**: Prefer `model_validate_json(raw)` over `model_validate(json.loads(raw))`; single Rust pass, avoids the Python `dict` allocation.
**DON'T**: Use `ORJSONResponse` / `UJSONResponse` in new code; deprecated in FastAPI 0.131.
**DON'T**: Round-trip JSON in a hot loop (`json.dumps(orjson.loads(...))` and similar). Pick one library at the boundary, work with the parsed structure inside.

## Pydantic v2 Hot-Path Rules

The pydantic team has documented the hot-path rules explicitly at `docs.pydantic.dev/latest/concepts/performance/`: The high-leverage ones for a backend:

**DO**: Reuse a single `TypeAdapter` across calls; instantiating one rebuilds the validator and serializer each time. Cache it at module load.
**DO**: Use concrete generic types (`list[T]`, `dict[K, V]`) over abstract ones (`Sequence[T]`, `Mapping[K, V]`); pydantic skips an `isinstance` chain when the type is concrete (~20 % win on collection-heavy schemas).
**DO**: Use **discriminated unions** with `Field(discriminator="type")`. O(1) dispatch instead of trying each member in order.
**DO**: Use `TypedDict` for nested data you don't need to validate beyond shape; pydantic docs measure it ~2.5× faster than nested `BaseModel`.
**DO**: Use `FailFast` on list types when an early bail is acceptable. `Annotated[list[Item], FailFast]` stops at the first invalid element.
**DON'T**: Use **wrap validators** in hot paths: they force materialization of the data in Python during validation, defeating the Rust core.
**DON'T**: Use `@computed_field` for anything expensive: it recomputes on every access. Cache in `model_post_init` instead.
**DON'T**: Inherit `BaseModel` for internal trusted data. `dataclass(slots=True, frozen=True)` is much faster to construct and the validation work is wasted on data you already trust.

## SQLAlchemy 2.0 Async Pool Sizing

`create_async_engine` returns an engine wrapping `AsyncAdaptedQueuePool` (the asyncio-safe pool). The defaults are wrong for production; set pool size, overflow, pre-ping, and recycle explicitly.

```python
# src/database.py
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker

engine = create_async_engine(
    settings.database_url,  # postgresql+asyncpg://
    pool_size=20,           # persistent connections per worker process
    max_overflow=10,        # burst headroom; total = pool_size + max_overflow
    pool_pre_ping=True,     # SELECT 1 on checkout; survives DB restarts
    pool_recycle=3600,      # recycle after 1 h (critical for MySQL/MariaDB)
    pool_timeout=30,        # wait up to 30 s for a free connection
    echo=False,
)

async_session = async_sessionmaker(
    engine,
    expire_on_commit=False, # required for FastAPI; see sqlalchemy.md
    autoflush=False,
)
```

**Sizing rule of thumb**: for asyncio workers, `pool_size = expected_concurrent_DB_queries_per_worker`. That's usually 10–20 for a typical API service. With Gunicorn/N workers, the database sees `N * (pool_size + max_overflow)` total connections at peak; make sure your Postgres `max_connections` budget accommodates it (Postgres default is 100, leaving little headroom for `psql` debugging).

**DO**: Use **asyncpg** as the driver (`postgresql+asyncpg://`) for raw throughput, or **psycopg3 async** (`postgresql+psycopg://`) when you need its feature breadth (COPY support, better extensions). Either is a strict upgrade on the sync drivers for async services.
**DO**: Set `pool_pre_ping=True` in production. The per-checkout overhead is tiny; the cost of a stale-connection error storm after a DB failover is much worse.
**DO**: Document the pool math per environment. `workers * (pool_size + max_overflow) ≤ pg_max_connections - admin_headroom` is a rule you want to be able to point at during incident review.
**DO**: For asyncpg with many unique query shapes (literal values, not parameters), pass `connect_args={"statement_cache_size": 0}` to disable the per-connection prepared-statement cache; otherwise it grows unbounded.
**DON'T**: Pass a sync `QueuePool` to `create_async_engine`. `AsyncAdaptedQueuePool` is selected automatically for async engines and is the only pool implementation that's safe under asyncio.
**DON'T**: Set `pool_size=1, max_overflow=0` "to avoid leaks". That serializes every request behind one connection and destroys async throughput. Leak detection belongs in pool logging + tracing, not in starvation.
**DON'T**: Hold an `AsyncSession` across `await` boundaries that yield to long external I/O. You're pinning a pool connection while doing something else.

## Profiling: Which Tool for Which Symptom

Use the right tool for the symptom, in production-safe order:

| Symptom | Tool | Notes |
| --- | --- | --- |
| "API feels slow: where does it spend time?" | **py-spy** | `py-spy record --pid PID -o flame.svg`; samples externally, zero instrumentation, prod-safe |
| "Container OOM'd after 2 h" | **memray** | Tracks every allocation incl. C extensions; flame graph for memory |
| "Want CPU + GPU + memory + line-level in one pass" | **scalene** | Academic-grade; AI suggestions via LLM hook |
| "Production task is wedged; what's it doing right now?" | **`python -m asyncio ps PID` / `pstree PID`** (3.14+) | Live introspection via PEP 768 |
| "Need to attach a debugger to a running prod container" | **`sys.remote_exec(pid, script)`** (3.14+) | PEP 768; runs a script at next safe point |
| "I need ultimate sample rate for a tight loop" | **`python -m profiling.sampling`** (3.15+) | Tachyon, 1 M Hz, async-aware |
| "Multiprocessing / Gunicorn workers" | py-spy `--subprocesses` | Profiles all worker PIDs |
| "Async timeline; what runs concurrently with what?" | **viztracer** | Best visualization for asyncio scheduling |

**DO**: Make `py-spy record` part of your incident runbook: It is the single highest-leverage production diagnostic: no code changes, no restart, attaches by PID.
**DO**: Run with `PYTHONASYNCIODEBUG=1` (or `asyncio.run(..., debug=True)`) in dev/staging: It logs callbacks slower than `loop.slow_callback_duration` (default 100 ms): that's how you find blocking-in-async early.
**DON'T**: Reach for `cProfile` / `pstats` as the first tool: they require code changes, add overhead, and don't see C-extension time.
**DON'T**: Profile in a debugger or with `pdb` running; you'll measure the debugger, not the code.

## Free-Threaded Python (no-GIL) for Web Servers

PEP 779 (accepted 2025-06) made free-threaded Python an **officially supported build target** in 3.14, no longer experimental: The free-threaded interpreter is built with `--disable-gil` and distributed as the `python3.14t` binary.

**For a FastAPI/Starlette web server in 2026, free-threading is not yet the production default.** Two reasons:

1. Web workloads are I/O-bound; asyncio already releases the GIL on every `await` of a network/disk call, so the GIL is rarely the bottleneck. Removing it doesn't add throughput: but the per-operation atomic refcount overhead it introduces (~5–10 % single-threaded) does.
2. Many extensions still need work. Check `py-free-threading.github.io/tracking/` for status. FastAPI/Starlette declared support in their April 2026 releases, but many transitive deps haven't.

Use it when:
- You have **CPU-bound** work mixed into a Python service that you can't easily move to a separate process (image pre-processing, ML inference glue, scientific code).
- You're writing a **batch processor / data pipeline** where multiple cores per process actually help.

Don't use it when:
- Your service is a normal request/response API. Stick with one Uvicorn worker per core.

```bash
# Try it (3.14+)
uv python install 3.14t
uv run --python 3.14t python -X gil=0 -c "import sys; print(sys._is_gil_enabled)"
```

## InterpreterPoolExecutor (3.14+)

`concurrent.futures.InterpreterPoolExecutor` (PEP 734) gives you true CPU parallelism via subinterpreters: each has its own GIL, but they share the parent process's memory layout and are far cheaper to spawn than processes.

```python
from concurrent.futures import InterpreterPoolExecutor

with InterpreterPoolExecutor() as pool:
    results = list(pool.map(cpu_heavy_function, batches))
```

**Status (mid-2026):** available but with sharp edges. Quoting the 3.14 What's New page directly:

> *"starting each interpreter has not been optimized yet. each interpreter uses more memory than necessary. many third-party extension modules on PyPI are not yet compatible with multiple interpreters."*

Treat it as a tool to evaluate in 2026, not a default. `ProcessPoolExecutor` remains the safe choice for shipped CPU work.

## When to Reach for Rust (PyO3)

Pydantic v1→v2, ruff, polars, orjson, msgspec all moved hot inner loops to Rust (or C) and won 10–100×. The pattern is now established: **Python for the API surface and glue; Rust for the hot inner loop**.

**Reach for Rust (PyO3 + maturin) when:**
- Profiling proves Python interpreter overhead is the bottleneck (not DB, not network, not I/O).
- You have a tight inner loop with known types; string parsing, binary protocol decoding, numerical transforms.
- You need free-threading safety. Rust's ownership model enforces thread safety at compile time.

**Don't reach for Rust when:**
- The bottleneck is the database (optimize queries, indexes, pool first).
- The hot path is `<1 ms`. Python overhead is negligible at that scale.
- The team can't maintain Rust. Operational risk usually outweighs the perf win.
- You haven't profiled. Always profile before rewriting.

## Quick Reference

| Topic | 2026 recommendation |
| --- | --- |
| Python version | 3.14.5+ for production (avoid 3.14.0–3.14.4 due to incremental GC issues) |
| ASGI server | `uvicorn[standard]` (uvloop + httptools): the default |
| JSON | Return a Pydantic model with a typed return annotation; FastAPI 0.130+ Rust-serializes it |
| ORM | SQLAlchemy 2.0 async + asyncpg + `pool_pre_ping=True` + `selectinload`/`joinedload` |
| Free-threading on web servers | Not yet the default; revisit late 2026 |
| Profiling first stop | `py-spy record` (CPU), `memray` (memory) |
| `ORJSONResponse` | **Deprecated since FastAPI 0.131**. drop it |
| Settings hot path | `BaseSettings` instantiated once at module load: never per-request |

For the operational deprecation list (datetime.utcnow, get_event_loop, etc.) see [`2026-currency.md`](2026-currency.md). For event-loop discipline see [`async-patterns.md`](async-patterns.md).
