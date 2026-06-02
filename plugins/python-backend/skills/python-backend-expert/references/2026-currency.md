# Versions, Deprecations, Tooling

Two sources of truth outrank anything here:

1. The project's `pyproject.toml` and `uv.lock` (what's actually installed).
2. `uv pip audit` or `pip-audit` in CI (what has known CVEs).

What follows is per-version language facts (which Python release shipped which feature) and a deprecation list for things AI assistants still emit out of habit. Both are stable historical facts.

## Default toolchain

| Concern | Use | Replaces |
| --- | --- | --- |
| Packaging, venv, Python install, scripts | **uv** | pip, virtualenv, pyenv, pipx, poetry, pipenv |
| Lint + format | **ruff** | black, isort, flake8, pyupgrade, pylint |
| Type check | **mypy** or **pyright** (`ty` from Astral is Beta) | -- |
| Test runner | **pytest** with `asyncio_mode="auto"` and `anyio[trio]` | -- |
| ASGI server | **uvicorn[standard]** (uvloop + httptools) | uvicorn without `[standard]` is the slow default |
| JSON serialization | Return a Pydantic model from your FastAPI route. For non-route JSON, **orjson**. | stdlib `json`, ujson |
| Structured logging | **structlog** | `logging.info(f"...")` with no structure |
| JWT | **PyJWT** | python-jose |
| Password hashing | **pwdlib** (Argon2) or **argon2-cffi** directly | passlib (broke on Python 3.13) |

**DO**: Commit `uv.lock` for apps and services. Use `uv sync --frozen --no-dev` in Docker.
**DO**: Run `pip-audit` (or `uv pip audit`) in CI against your lockfile. Let the tool catch CVEs; don't pin to chase them manually.
**DO**: Pin tool versions in `pyproject.toml`. Astral controls uv, ruff, and ty; spreading that risk across `mypy`/`pyright` as well keeps you covered if one tool stalls.

## Per-version features (stable facts)

Use what the project's Python version offers. Don't push a 3.14 feature onto a 3.11 project.

### Python 3.11
- `asyncio.TaskGroup`: structured concurrency. Replaces manual `create_task` + `gather`.
- `asyncio.timeout()`: context-manager timeouts. Cleaner than `wait_for`.
- `ExceptionGroup` / `except*` (PEP 654): concurrent error propagation.
- `typing.Self` (PEP 673): for builder/classmethod returns.
- `datetime.UTC` alias for `timezone.utc`.

### Python 3.12
- PEP 695 type parameter syntax: `def f[T](...)`, `class Stack[T]`, `type Vector = list[float]`. Drops `TypeVar` boilerplate.
- PEP 698 `@override`: type checker errors if the base method was renamed.
- `distutils` removed (PEP 632).
- `datetime.utcnow()` / `utcfromtimestamp()` deprecated.

### Python 3.13 (Oct 2024)
- `typing.TypeIs` (PEP 742): prefer over `TypeGuard`. Narrows in both branches.
- `typing.ReadOnly` for `TypedDict` keys (PEP 705).
- TypeVar defaults (PEP 696): `type T = TypeVar("T", default=str)`.
- 19 stdlib modules removed (PEP 594): `cgi`, `crypt`, `telnetlib`, `nntplib`, ...
- Experimental free-threaded build and JIT (both opt-in).
- New REPL with multiline editing.

### Python 3.14 (Oct 2025)
- Deferred annotations (PEP 649 + 749): forward refs work without `from __future__ import annotations`. The future import still works and is not yet deprecated; deprecation is scheduled for after Python 3.13 EOL (~2029). Don't churn existing files to remove it.
- Template strings (PEP 750): `t"..."` returns a `Template`. Safe HTML/SQL primitives.
- Free-threaded build officially supported (PEP 779). Single-thread overhead a few percent; varies by workload.
- `concurrent.interpreters` and `InterpreterPoolExecutor` (PEP 734).
- Safe external debugger interface (PEP 768). Backs `python -m asyncio ps PID` for live introspection.
- `multiprocessing` POSIX default changed from `fork` to `forkserver`. Code that pre-loads state in the parent and expects copy-on-write sharing must now set `mp.set_start_method("fork")` explicitly.
- asyncio event-loop policy system deprecated; removed in 3.16. Use `asyncio.run(main(), loop_factory=uvloop.new_event_loop)`.

### Python 3.15 (preview, expected Oct 2026)
- Lazy imports accepted (PEP 810).
- `python -m profiling.sampling` sampling profiler in the stdlib.

## Stop / use instead

Stable list of patterns AI assistants still emit but shouldn't.

| Stop | Use | Since | Ruff rule |
| --- | --- | --- | --- |
| `datetime.utcnow()` / `datetime.utcfromtimestamp()` | `datetime.now(UTC)` / `datetime.fromtimestamp(t, UTC)` | 3.12 dep. | `DTZ003` |
| `Optional[X]`, `Union[X, Y]` | `X \| None`, `X \| Y` | 3.10 | `UP045`, `UP007` |
| `typing.List/Dict/Tuple/Set/Type` | `list[...]` etc. | 3.9 | `UP006`, `UP035` |
| `asyncio.get_event_loop()` outside a coroutine | `asyncio.run(main())` at entry; `asyncio.get_running_loop()` inside | 3.10 dep. | -- |
| `asyncio.wait_for(coro, timeout)` | `async with asyncio.timeout(timeout): await coro` | 3.11 | -- |
| `uvloop.install()` / `set_event_loop_policy(...)` | `asyncio.run(main(), loop_factory=uvloop.new_event_loop)` or `uvloop.run(main())` | 3.14 dep. | -- |
| `@app.on_event("startup")` | `lifespan` async context manager | FastAPI 0.95 | -- |
| `ORJSONResponse` / `UJSONResponse` | Return a typed Pydantic model | FastAPI 0.131 | -- |
| `python-jose` | `PyJWT` | -- | -- |
| `passlib` | `pwdlib[argon2]` or `argon2-cffi` | -- | -- |
| `async_asgi_testclient` | `httpx.AsyncClient` + `ASGITransport` | -- | -- |
| Pydantic v1 API (`.dict()`, `class Config`, `@validator`, `orm_mode`, `json_encoders`) | v2 API (`model_dump()`, `ConfigDict`, `@field_validator`, `from_attributes`, `@field_serializer`) | Pydantic 2.0 | -- |
| `setup.py` / `setup.cfg` | `pyproject.toml` (PEP 621) | -- | -- |
| `[project.optional-dependencies.dev]` | `[dependency-groups]` (PEP 735) | uv 0.5 | -- |
| `os.path.join/exists/...` | `pathlib.Path` | -- | `PTH118` etc. |
| `assert` for runtime validation | `raise ValueError(...)` | -- | `S101` |
| Global `pip install` | `uv` | -- | -- |
| `requests` in async code | `httpx.AsyncClient` | -- | -- |
| Mutable default args | `def f(x=None): x = x or []` | -- | `B006` |
| Bare `except:` | `except Exception:` (or the specific type) | -- | `E722` |

## Supply-chain hygiene

- Use **PyPI trusted publishing + attestations** (PEP 740) for anything you publish.
- Run **`pip-audit`** (or `uv pip audit` when GA) in CI against your lockfile. Fail the build on HIGH/CRITICAL advisories.
- **Always store timezone-aware datetimes in UTC**. The classic silent incident is `utcnow()` written, local time read.

Specific package versions and current CVE numbers age in days, not months. They belong in your lockfile and your security scanner, not in this skill.
