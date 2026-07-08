---
name: python-backend-expert
description: This skill should be used when the user is writing, reviewing, debugging, or architecting Python backend services (FastAPI + SQLAlchemy 2.0 async + Pydantic v2) or long-running runtime processes (workers, ETL, daemons, CLI tools). Assumes Python 3.13+/3.14; detects installed libraries from pyproject.toml/uv.lock and adapts. Covers async correctness, dependency injection with Annotated[T, Depends], N+1 and MissingGreenlet, lifecycle and graceful shutdown, signals/subprocess/atomic writes, project structure, validation, testing, and AI-generated anti-patterns. Trigger phrases include "critique my FastAPI backend", "why am I getting MissingGreenlet", "is this blocking the event loop", "fix N+1 query", "structure my FastAPI project", "review my Pydantic models", "write a Python worker", "subprocess deadlock", and "process keeps growing in memory". Not for Django or Flask apps, notebooks, or data-science scripts — those stacks have different idioms this skill would misapply.
---

This skill guides expert Python development for two project shapes: **web backends** (FastAPI + SQLAlchemy 2.0 + Pydantic v2) and **runtime processes** (workers, CLIs, ETL, daemons, scheduled jobs). The patterns differ; classify the project first, apply the right subset.

Assume Python 3.13+ for all new code unless the project explicitly pins lower. 3.14 is the current production-recommended release (free-threaded build, deferred annotations, asyncio introspection). The skill notes when a feature requires a specific version. Detect the actual version from `requires-python` in `pyproject.toml` before recommending a feature.

Do not invent APIs. Verify a method or pattern exists in the project's installed library versions before suggesting it. Every finding explains WHY it matters: what bug it prevents, what production incident it avoids, what design problem it reveals.

## First, Classify the Project

Different defaults apply to different project shapes. Detect from the project before applying rules.

| Signal | Project shape | Apply |
| --- | --- | --- |
| Imports `fastapi` or `starlette`; has `app = FastAPI(...)` | **Web backend** | All references below |
| Has `pydantic-settings` and `sqlalchemy[asyncio]` and an HTTP framework | **Web backend** | Same |
| `[project.scripts]` entry point + no HTTP framework | **CLI / runtime** | Skip `fastapi.md` and `lifecycle.md`. Use `runtimes.md`. |
| Long-running script with `signal.signal`, `multiprocessing`, file processing, scheduled task | **Runtime process** | Same as CLI |
| Notebook, prototype, glue script | **Out of scope** | Most patterns are overkill; flag only blocking issues |

If both apply (a backend with a CLI sidecar, an admin script, a Celery worker), apply both: backend rules to the HTTP code, runtime rules to the workers and CLIs.

## How to Reason About Findings

Every finding follows the same shape. Discover, then evaluate, then propose with a verification plan.

**Discover.** Read the code before suggesting anything. Look for the shape of state, the I/O boundaries, what's already imported, whether there's a service layer, where transactions begin and end, what the tests cover. A finding that ignores what the code is actually doing is noise.

**Evaluate against named consequences.** When you see a blocking call inside `async def`, name the failure mode: the event loop stalls, every concurrent request on the worker times out, the sync `TestClient` doesn't catch it. When you see a relationship accessed without `selectinload`, name `MissingGreenlet` and where it raises. When you see `os.replace` across filesystems, name the `OSError(EXDEV)`. Findings backed by a concrete consequence get stated directly. Findings without one usually aren't worth reporting.

**Understand before recommending.** A pattern that looks wrong might be deliberate (a single-implementor `Protocol` may be the planned seam for a second adapter; an `Arc<Mutex<>>`-equivalent shared dict may be a cache with proven short critical sections). If you can't tell whether the pattern is justified, ask one specific question instead of inventing a confident wrong answer.

**Verify the fix.** Every fix should come with a way to confirm it worked: the test that now passes, the load-test behavior that now holds, the `py-spy` profile that no longer shows the hot path. A fix without verification is a guess. If a test doesn't exist, write it.

**Be opinionated, not hedgy.** "Consider whether X applies to your context" is a sentence with no information. The honest form is specific: "I can't tell whether the `UserRepository` is positioned for a second backend. If it is, the interface is justified; if not, it's overhead. Which is it?" Vague hedging wastes the developer's time.

## Three Questions to Ask Before Any Fix

1. **What concrete bug does this prevent?** If you can't name it (a class of production incident, a wrong-result scenario, a measurable perf cliff), the fix may not be worth the complexity.
2. **What would happen in production?** Reason in incidents, not in style preferences.
3. **Is the type system doing enough work?** Every runtime `assert` is a type waiting to be born.

## How to Think About Backend Problems

Trace every issue through three layers before fixing:

- **Layer 3, Domain (WHY)**: Business rules, consistency requirements, latency budget, deployment shape. These constrain everything below.
- **Layer 2, Design (WHAT)**: Module boundaries, error strategy, schema/DTO design, transaction boundaries, where async lives. Check against SOLID and the boundary rules.
- **Layer 1, Mechanics (HOW)**: The immediate bug; a blocking call, a lazy load, a missing `await`. Fix it, but always trace UP to the design decision that allowed it.

Reframe the common runtime errors as design questions:

| Symptom | Don't Just Say | Ask Instead |
| --- | --- | --- |
| `MissingGreenlet` / `DetachedInstanceError` | "Add `selectinload`" | Where is the load boundary? Should this be eager? |
| Endpoint freezes under load | "Add more workers" | Is a blocking call running inside `async def`? |
| `response_model` validates twice | "Ignore it" | Should the route return an ORM row/dict instead of a model? |
| 500s hidden as generic errors | "Catch `Exception`" | Which specific domain exception maps to which HTTP status? |
| Settings sprawl across modules | "One big `Settings`" | Which domain actually owns this config? |
| Subprocess hangs at random | "Add a timeout" | Is the parent reading both stdout and stderr while the child writes? |
| Worker leaks memory over days | "Restart it" | What's holding references across the loop? `lru_cache` on an instance method? Logger handler accumulation? |

## Async Correctness: Start Here

This is the **#1 source of FastAPI production bugs**. A single blocking call inside `async def` freezes the entire event loop; every concurrent request on that worker stalls. It passes every test (the sync `TestClient` hides it) and only melts down under production load.

→ _Consult [async-patterns reference](references/async-patterns.md) for the blocking taxonomy, threadpool limits, TaskGroup, and cancellation safety._

**DO**: Decide route color by what it does (see the table below).
**DO**: Use `await run_in_threadpool(sync_fn,...)` (or `asyncio.to_thread`) when you must call a sync SDK (boto3, a legacy client) from an `async def` route.
**DO**: Offload CPU-bound work (>~50ms: image processing, ML, crypto hashing) to a worker process (Celery/Arq/RQ) or `ProcessPoolExecutor`: the GIL means threads don't help.
**DO**: Use `asyncio.TaskGroup` (3.11+) for concurrent independent calls; structured concurrency cancels siblings on failure, unlike bare `gather`.
**DON'T**: Call `time.sleep`, `requests.*`, a sync DB driver, or blocking file I/O inside `async def`.
**DON'T**: Make a dependency `def` when it does no I/O. FastAPI runs sync dependencies in the 40-thread pool, wasting threads. Make pure-compute dependencies `async def`.
**DON'T**: Fire-and-forget with `asyncio.create_task` without catching exceptions: they vanish silently.

| Route does this | Use |
| --- | --- |
| `await`able non-blocking I/O (httpx, AsyncSession) | `async def` |
| Blocking I/O with no async client available | `def` (FastAPI runs it in the threadpool) |
| Mix of async I/O and a blocking call | `async def` + `run_in_threadpool` for the blocking part |
| CPU-bound > ~50ms | Offload to a Celery/Arq/RQ worker process |

## Dependency Injection (Depends)

→ _Consult [dependency-injection reference](references/dependency-injection.md) for `Annotated` aliases, chained validation dependencies, and DI anti-patterns._

**DO**: Use `Annotated[T, Depends(...)]`, the idiomatic form since FastAPI 0.95: not the legacy `T = Depends(...)` default-argument form.
**DO**: Put DB/ownership validation in dependencies (`valid_post_id`, `valid_owned_post`): they raise the right HTTP error and keep routes thin. Results are cached per request, so chain small dependencies freely.
**DO**: Yield the session from a `get_db` dependency and commit/rollback at that boundary.
**DON'T**: Use a service-locator/global container that classes reach into at runtime; inject explicitly via the constructor or `Depends`.
**DON'T**: Inject a raw `AsyncSession` into a route and do ORM work there; inject a service or a repository.
**DON'T**: Inject more than ~5 dependencies into one class: that's an SRP smell; split it.

## Pydantic v2 & API Schemas

→ _Consult [validation reference](references/validation.md) for request/response separation, schema organization, the validation-layers table, pagination, versioning, parse-don't-validate, smart constructors, NewType wrappers, and strict vs lax Pydantic._

Pydantic v1 is gone; LLMs hallucinate v1 constantly. Use `model_dump` not `.dict`, `model_validate` not `parse_obj`, `model_config = ConfigDict(...)` not `class Config`, `from_attributes=True` not `orm_mode`, `@field_validator`/`@field_serializer` not `@validator`/`json_encoders`.

**DO**: Separate request schemas from response schemas: never one model with half-optional fields.
**DO**: Set `model_config = ConfigDict(from_attributes=True)` on response models and return the ORM row; let `response_model` do the serialization.
**DO**: Keep Pydantic at the edges (API I/O, settings). Use `@dataclass(slots=True, frozen=True)` for domain entities; re-validating data read from a trusted DB is wasted work.
**DON'T**: Return a `Pydantic` instance AND set `response_model=` to the same class. FastAPI then constructs it twice. Return a dict or ORM row instead.
**DON'T**: Write `Field(ge=18, default=None)`: a constraint that contradicts its default. Pick `int = Field(ge=18)` or `int | None = Field(default=None, ge=18)`.
**DON'T**: Return `dict[str, Any]` from a handler; declare a typed `response_model`.

## SQLAlchemy 2.0 (Async)

→ _Consult [sqlalchemy reference](references/sqlalchemy.md) for 2.0 mapping, loading strategies, and session management._

Use `Mapped`/`mapped_column`, `select` (never `session.query`), `AsyncSession`, `async_sessionmaker`, `create_async_engine`.

**DO**: Set `expire_on_commit=False` on the async sessionmaker; otherwise attribute access after commit triggers implicit lazy I/O and raises `MissingGreenlet`.
**DO**: Eager-load every relationship you touch: `selectinload` for collections (one-to-many), `joinedload` for many-to-one. Default relationships to `lazy="raise"` so unintended loads fail loudly in tests, not in production.
**DO**: Use one `AsyncSession` per request; never share a session across tasks or store it on a long-lived object.
**DON'T**: Access a relationship that wasn't explicitly loaded in async: this is the #1 cause of `MissingGreenlet`/`DetachedInstanceError` in FastAPI deployments.
**DON'T**: Use a sync `Session` inside an `async def` route: it blocks the loop and can deadlock the pool.
**DON'T**: Write `session.execute(select(...))` in a router: that's naked ORM in the wrong layer.

## Project Structure & Settings

→ _Consult [project-structure](references/project-structure.md) for the canonical layouts and pyproject template; [architecture](references/architecture.md) for layer rules and when hexagonal is overkill._

The FastAPI-idiomatic default is **domain/module-based**, not file-type folders. Each domain owns its slice:

```
src/
  auth/    router.py schemas.py models.py service.py dependencies.py exceptions.py config.py
  posts/   router.py schemas.py models.py service.py dependencies.py exceptions.py
  core/    config.py database.py exceptions.py logging.py
  main.py
```

**DO**: Organize by domain. When one file grows a second concern, split into a package and re-export from `__init__.py`.
**DO**: Import across domains at the module level (`from src.auth import service as auth_service`), never deep (`from src.auth.service.user import create_access_token`).
**DO**: Split settings by domain. `AuthConfig(BaseSettings)` with `env_prefix="AUTH_"` in `src/auth/config.py`; a small `core/config.py` for cross-cutting (DB, Redis, CORS). Instantiate once at module load.
**DON'T**: Put business logic, auth, or schemas in `router.py`. Routers are HTTP-only.
**DON'T**: Funnel every environment variable through one god `Settings` that every module imports.
**DON'T**: Let a file pass ~400 LOC without a hard look; treat ~600 as a must-split.

## Application Lifecycle (Backend)

→ _Consult [fastapi.md](references/fastapi.md) for lifespan, middleware, response_model, runtime footguns; [lifecycle.md](references/lifecycle.md) for the full startup→ready→drain flow, request lifecycle, retries, idempotency, observability; [performance.md](references/performance.md) for Uvicorn tuning and JSON serialization._

**DO**: Use the `lifespan` async context manager for startup/shutdown. `@app.on_event("startup")` is deprecated. Yield shared resources as typed lifespan state.
**DO**: Install `uvicorn[standard]` (uvloop + httptools). It's the production default; alternatives are only worth evaluating if you've measured Uvicorn as the bottleneck.
**DO**: Return a typed Pydantic model from your route. FastAPI 0.130+ Rust-serializes via pydantic-core (fastapi/fastapi#14962). `ORJSONResponse`/`UJSONResponse` are deprecated as of 0.131.
**DO**: Use **PyJWT** for tokens, **pwdlib** (Argon2 default) or **argon2-cffi** for passwords. FastAPI's tutorial moved off `python-jose` long ago; `passlib` broke on Python 3.13.
**DO**: Separate **liveness** (trivial) from **readiness** (touches the DB). Otherwise a slow DB kills healthy pods.
**DO**: Set `terminationGracePeriodSeconds ≥ uvicorn --timeout-graceful-shutdown`. Otherwise SIGKILL mid-drain → 5xx spike on every deploy.
**DON'T**: Use `BaseHTTPMiddleware` on the hot path; Starlette #1012 (unbounded queue under back-pressure) is still open and the perf gap vs pure ASGI middleware remains.
**DON'T**: Expose `/docs` and `/redoc` in production; set `openapi_url=None` outside dev/staging.
**DON'T**: Run DB migrations from inside `lifespan`. A bad migration becomes a crash-loop with no migration owner. Run them in a separate one-shot job.

## Runtime Processes (Worker, CLI, ETL, Daemon)

→ _Consult [runtimes.md](references/runtimes.md) for subprocess pitfalls (pipe deadlock, timeout/check, kill the process group), filesystem (atomic writes, fsync, FD leaks, scandir), process model (signals, exit codes, restart, cleanup), CLI patterns, and a 10-item review checklist._

**DO**: Set `timeout=` AND `check=True` on every `subprocess.run`. Without timeout, a hung child blocks the worker forever; without check, a failing command is silently ignored.
**DO**: Read both `stdout` and `stderr` concurrently (or use `subprocess.run(capture_output=True)`). `Popen.wait()` without draining pipes deadlocks once the child writes more than the pipe buffer.
**DO**: Install signal handlers that just set a flag. Exit the loop in normal code; never call `logging` or acquire a lock inside a signal handler.
**DO**: Write atomic files with `os.replace()` and a temp file in the **same directory** as the destination. Across filesystems `os.replace` raises `OSError(EXDEV)`; `shutil.move` silently degrades to non-atomic copy-then-delete.
**DON'T**: Use `time.time()` to measure elapsed time. NTP can move the wall clock backward. `time.monotonic()` for durations.
**DON'T**: Use `shell=True` with any dynamic input. It's a remote-code-execution primitive. Use the list form `["cmd", "arg"]`.
**DON'T**: Open files in a loop without `with` (FD leak), or rely on `__del__` for cleanup (unreliable; use `weakref.finalize` or `try/finally`).

## Error Handling

→ _Consult the **Error design** section of [modern-python reference](references/modern-python.md) for the typed exception hierarchy, handler registration, error-response shape, and structured logging on failures._

**DO**: Define typed domain exceptions and register app-level handlers that map them to HTTP. keep routes free of try/except plumbing.
**DO**: Override `RequestValidationError` for a stable client-facing error contract.
**DON'T**: Catch bare `Exception` in a route to silence 500s; catch the specific error or let the handler do its job.
**DON'T**: Raise `HTTPException` deep inside a service: that couples business logic to the web framework. Raise a domain error; map it at the boundary.

## Testing

→ _Consult [testing reference](references/testing.md) for fixtures, real-DB integration, and override patterns._

**DO**: Test with `httpx.AsyncClient` + `ASGITransport`: the sync `TestClient` masks async bugs and skips lifespan state.
**DO**: Pair pytest with `anyio[trio]` (FastAPI/Starlette run on AnyIO) and set `asyncio_mode = "auto"`. Treat `filterwarnings = ["error"]` as the deprecation trip-wire.
**DO**: Use `app.dependency_overrides` to swap auth/external deps; run integration tests against a real database (testcontainers), not mocks.
**DON'T**: Use `async_asgi_testclient` (unmaintained) or `python-jose` (FastAPI moved its tutorial off it long ago; use `PyJWT`).
**DON'T**: Mock the database in integration tests; mock/prod divergence surfaces as a production incident.

## Modern Python & Tooling

→ _Consult [modern-python reference](references/modern-python.md) for pure-Python idioms (EAFP, truthiness, closures, scope, generators, match, walrus) and class/data design. See [2026-currency reference](references/2026-currency.md) for the per-version PEP table and the stop-doing list._

Target 3.13+ for new code; 3.14 brings deferred annotations, free-threaded build, asyncio introspection. `str | None` not `Optional[str]`; `list[int]` not `List[int]`; `class Repo[T]` (PEP 695) not `Generic[T]`; `StrEnum`; `@override`; `datetime.now(UTC)`. Prefer `TypeIs` over `TypeGuard` (3.13+); it narrows in both branches.

**DO**: Manage everything with `uv` (never global `pip install`); lint and format with `ruff`; type-check with mypy/pyright (or Astral's `ty`, Beta). Treat the **Astral monopoly** as real concentration risk; keep alternatives viable.
**DO**: Use `[dependency-groups]` (PEP 735) for dev tooling. Not `[project.optional-dependencies]`. Dev deps aren't a shipped feature.
**DO**: Use `@dataclass(slots=True, frozen=True, kw_only=True)` for value objects, `Protocol` for ports, `Self`/`TypeIs` where they sharpen types.
**DO**: Use `asyncio.run(main(), loop_factory=uvloop.new_event_loop)` (or `uvloop.run(main())`). The asyncio policy system is deprecated in 3.14, removed in 3.16. No more `uvloop.install()`.
**DON'T**: Use `assert` for production validation; stripped under `python -O`.
**DON'T**: Use mutable default args, bare `except:`, star imports, or `os.path` where `pathlib` fits.
**DON'T**: Use `asyncio.get_event_loop()` at module level; it raises `RuntimeError` in 3.14+ if no loop is running. Use `asyncio.get_running_loop()` inside a coroutine; `asyncio.run(main())` at the entry point.

## Anti-Patterns & AI Slop

→ _Consult [ai-slop.md](references/ai-slop.md) for the top 20 AI-generated patterns (10 code + 10 architectural) with WHY/BAD/GOOD examples._

**The AI Slop Test**: could a senior Python engineer immediately say "an AI wrote this"? The most common tells: `dict[str, Any]` tunneling through layers, blocking calls inside `async def`, lazy loads in async, ORM models doubling as response schemas, Pydantic v1 ghosts (`.dict()`, `class Config`, `@validator`, `orm_mode`), inline `if not user.is_admin: raise HTTPException(403)` instead of a router dependency, `BackgroundTasks` for work that needs a queue, generic variable names (`data`, `result`, `item`), and "compiles + passes the sync test" treated as the quality bar.

## Still in Motion (Mid-2026)

These are not yet stable as of 3.14. Use the workaround; don't recommend the new thing.

- **Free-threaded Python (`python3.14t`, PEP 779)**: officially supported but most C extensions aren't compatible yet. Use it only for measured CPU-bound workloads with verified extension support. Default to GIL builds.
- **`ty` (Astral type checker)**: Beta, fast, but not yet a drop-in replacement for mypy/pyright on large codebases. Keep mypy or pyright as the conservative default.
- **PEP 810 lazy imports**: accepted, shipping in 3.15. Until then, use manual function-local imports for heavy optional deps.
- **`InterpreterPoolExecutor` (PEP 734, 3.14)**: works, but third-party C extensions are mostly not yet subinterpreter-safe. Stick with `ProcessPoolExecutor` for shipped CPU work.
- **`httpx2` (Pydantic's stewardship continuation of `httpx`, github.com/pydantic/httpx2)**: real and active — same API, new package/import name — but `httpx` is still installed everywhere. Don't recommend migration without need.

## Hard-won opinions the references don't repeat

- **`def f(items: list = [])` is the oldest bug in Python and it still ships weekly.** The default is evaluated once at import; every call shares the list. `None` sentinel + `items = items or []` (or `[]` inline via `field(default_factory=...)` in dataclasses).
- **`except Exception: pass` is data loss wearing a seatbelt.** The narrowest exception you can name, or at minimum `logger.exception(...)` before any suppression. A bare `except:` also eats `KeyboardInterrupt`/`SystemExit` — never.
- **`assert` is not runtime validation.** `python -O` strips every assert in the process; an "assert-validated" boundary is unvalidated in any deployment that sets -O. Asserts state internal invariants; boundaries raise real exceptions or use Pydantic.
- **Never shadow builtins.** `list`, `dict`, `id`, `type`, `input` as variable names each break something subtle later in the same scope — and `id` as a parameter name is endemic in route handlers.
- **Threads don't speed up CPU-bound Python.** Under the GIL, threads buy you concurrency for I/O only; for CPU use processes (or a measured 3.14 free-threaded build). A `ThreadPoolExecutor` around numpy-less number crunching is a no-op with overhead.
- **`requests` inside `async def` freezes the event loop** — one of the most common review finds. `httpx`/`httpx2` async client, created once at lifespan, never per-request.
- **Import-time side effects are a testing tax.** Module-level DB connections, settings reads, and network calls make every import a deployment event. Construct at lifespan/entry-point, import stays free.

## Zoom out before you edit

Sessions that skip this produce split-brain code (a second `utils.parse_date`, a second settings loader) and orphans. Non-negotiable sequence for any change:

1. **Before adding a function, model, or helper: search for an existing one** (`rg -i` the concept — check `utils/`, `common/`, the service layer). A second implementation is a drift bug on a timer.
2. **Read the whole module and its callers before editing**, not just the flagged lines.
3. **After the change, grep the old symbol names** and delete anything now unreferenced in the same change.
4. **Say in one sentence where the change sits** (router / service / domain / infra). If you can't name the layer, read more first.
5. **Verification is output, not assertion.** Paste the `pytest` / `ruff` / `py_compile` output; "verified" without command output is not verification. If you didn't run it, write "unverified."

## Review Process

Five passes. Correctness first, style last. Different passes apply depending on what you classified the project as.

1. **AI slop sweep** (always): `references/ai-slop.md`. Check the detection checklist.
2. **Correctness** (always):
   - Backend: `async-patterns.md`, `sqlalchemy.md`, `fastapi.md`, `lifecycle.md`.
   - Runtime/CLI: `runtimes.md` (subprocess, filesystem, process lifecycle, IPC). Skip the FastAPI/lifecycle pair.
3. **Boundaries** (always): `validation.md` (includes request/response, pagination, versioning, validation layers), `dependency-injection.md` (backend), and the **Error design** section of `modern-python.md`.
4. **Design** (always): `architecture.md`, `project-structure.md`.
5. **Style and currency** (always): `modern-python.md` (start here for pure-Python issues), `performance.md`, `2026-currency.md` for deprecations, `testing.md`.

## Severity Levels

Label every finding:

- **blocking**: Guaranteed production bug (blocking call in `async def`, lazy load in async, session shared across tasks, secret or `/docs` exposed, SQL/auth flaw, data-loss on cancellation, subprocess pipe deadlock, atomic-write across filesystems). Must fix before merge.
- **important**: Wrong error handling on external input, N+1 in a real path, `BackgroundTasks` for critical work, unmaintained dependency, missing tests for non-trivial logic, FD leak under sustained load.
- **architecture**: Misfit structure, fat router, leaked boundary, premature abstraction, file over the LOC cap.
- **nit**: Style, naming, minor idiom. Fix if convenient.
- **polish**: Pre-merge cleanup (ruff warnings, dead code, debug prints, missing type hints).
- **suggestion**: Alternative worth considering. No action required.
- **praise**: Highlight well-built code. Reinforce good patterns.

## Output Format

Group findings by file. For each: file path and line, severity label, the rule or anti-pattern ID, **WHY it matters** (the concrete consequence), and a before/after block when the fix isn't obvious. Skip clean files. End with a prioritized summary.

Be direct. Be specific (line 42 of routers/posts.py, not "some routes"). Say what's wrong and why. Prioritize ruthlessly: if everything is important, nothing is. Verify patterns exist in the project's actual versions before recommending them. Pair every fix with how to verify it worked (the test that should now pass, the load behavior that should now hold). Verification means running the check and showing its output — a claim of "verified" without command output in the transcript is not verification.
