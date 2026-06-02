# Project Structure

Pick one of two layouts. The full hexagonal/clean layout lives in `architecture.md` and is a last resort.

## Layout A: file-type (≤ 5 domains)

What `fastapi/full-stack-fastapi-template` ships with. Right for small apps:

```
project_root/
pyproject.toml
uv.lock
alembic/
env.py
versions/
alembic.ini
app/
api/
deps.py # Annotated[T, Depends(...)] aliases
routes/
users.py
items.py
login.py
core/
config.py # Pydantic BaseSettings()
security.py
models.py # ORM models (flat)
schemas.py # Pydantic DTOs (flat)
crud.py # data-access functions (flat)
main.py
tests/
conftest.py
api/test_users.py
```

## Layout B: per-domain (> 5 domains)

Per-domain modules at scale: Each domain owns a folder:

```
project_root/
pyproject.toml
uv.lock
alembic/
env.py
versions/
alembic.ini
src/
core/ # cross-cutting only: config, database, exceptions, logging
config.py
database.py
exceptions.py
logging.py
auth/ # one folder per domain
router.py # APIRouter for /auth
service.py # business logic
models.py # ORM models for auth
schemas.py # Pydantic request/response
dependencies.py # FastAPI deps (require_admin, current_user,...)
exceptions.py # domain exceptions
posts/
router.py service.py models.py schemas.py dependencies.py exceptions.py
payments/...
main.py # FastAPI app + lifespan
tests/
conftest.py
auth/test_router.py test_service.py
posts/test_router.py test_service.py
```

### When to switch from A to B

- A domain's `models.py` or `schemas.py` is mostly one logical thing: it wants its own folder.
- `crud.py` exceeds ~300 lines or has two service-ish functions per domain: split into per-domain `service.py`.
- New people can't locate where a feature lives in under 30 seconds.

Moving from A to B is mechanical: create the domain folder, move files in, update imports: No business-logic rewrite.

## Cross-domain imports go through the module name

Whichever layout you pick, this rule keeps coupling shallow:

```python
# GOOD
from src.auth import service as auth_service

# BAD: welds the caller to internal file structure
from src.auth.service.tokens.jwt import create_access_token
```

`auth/__init__.py` can rename internal files freely. Callers depend only on `src.auth.service`.

## Anti-patterns no real production repo uses

These folders show up in AI-generated FastAPI projects and in no real production codebase:

| AI generates | Use instead |
| --- | --- |
| `controllers/` | `routers/` or per-domain `router.py` |
| `helpers/` at root | `utils/` per-domain, or `core/utils.py` |
| `models/` mixing ORM and Pydantic | `models.py` (ORM) and `schemas.py` (Pydantic) per domain |
| `services/` at root | per-domain `service.py`, or `crud.py` for small apps |
| `repositories/` at root | data-access in `service.py` directly, or nested under a persistence package |
| `middleware/` package | register middleware in `main.py`; define inline |

## File size

Treat these as the time to look hard, not as hard limits.

| File | Comfortable | Look hard | Must split |
| --- | --- | --- | --- |
| `router.py` | 50-200 | 300 | 400 |
| `service.py` | 100-300 | 400 | 600 |
| `models.py` (per domain) | 50-200 | 250 | 350 |
| `schemas.py` (per domain) | 30-150 | 200 | 300 |
| Any single file | -- | 400 | 600 |

When a file hits "must split", turn it into a package and re-export the public names from `__init__.py`. Callers don't change.

## src/ layout

For an application that's only deployed in Docker and never installed via `pip install`, `src/` is optional: The most-starred FastAPI template (`fastapi/full-stack-fastapi-template`) doesn't use it.

For anything published to PyPI, use `src/`. Without it, `pytest` can import your local checkout directory instead of the installed package, hiding packaging bugs.

## pyproject.toml: the single source of truth

`pyproject.toml` replaces `setup.py`, `setup.cfg`, `MANIFEST.in`, and `requirements.txt`. One file. Declarative: No code at install time.

Canonical structure. **Don't copy the version pins literally**; let `uv add <pkg>` resolve them. The lockfile (`uv.lock`) is the single source of truth for versions, and `pip-audit` in CI is the source of truth for CVEs.

```toml
[build-system]
requires = ["uv_build"]
build-backend = "uv_build"

[project]
name = "myapp"
version = "1.0.0"
description = "A production FastAPI backend service."
readme = "README.md"
license = "MIT"
requires-python = ">=3.12"
dependencies = [
    "fastapi",
    "uvicorn[standard]",
    "pydantic",
    "pydantic-settings",
    "sqlalchemy[asyncio]",
    "asyncpg",                     # or "psycopg[binary,pool]"
    "alembic",
    "httpx",
    "pyjwt[crypto]",
    "pwdlib[argon2]",              # or "argon2-cffi" directly
    "structlog",
    "orjson",
]

[project.scripts]
myapp = "myapp.cli:main"

[dependency-groups]                # PEP 735 dev deps; not shipped to PyPI
dev = [
    "pytest",
    "anyio[trio]",
    "pytest-cov",
    "httpx",
    "testcontainers[postgres]",
    "ruff",
    "pip-audit",
    "mypy",
]

[tool.uv]
default-groups = ["dev"]

[tool.ruff]
line-length = 100
target-version = "py312"

[tool.ruff.lint]
select = ["E","W","F","I","UP","B","SIM","C4","DTZ","S","RUF","FAST"]
ignore = ["E501","B008","S101"]

[tool.ruff.lint.per-file-ignores]
"__init__.py" = ["F401","F403"]
"tests/**/*.py" = ["S101","SIM"]
"scripts/**/*.py" = ["T201"]

[tool.pytest.ini_options]
minversion = "8.0"
testpaths = ["tests"]
asyncio_mode = "auto"
addopts = ["--strict-config","--strict-markers","-ra"]
filterwarnings = ["error","ignore::DeprecationWarning:httpx"]

[tool.mypy]
python_version = "3.12"
strict = true
plugins = ["pydantic.mypy"]
```

### Rules

**DO**: Always include `[build-system]`. Without it, uv won't install the project itself.
**DO**: Use `[project]` (PEP 621) for metadata: Not `[tool.poetry]`.
**DO**: Use `[dependency-groups]` (PEP 735) for dev tooling: Not `[project.optional-dependencies]`.
**DO**: Use `[project.optional-dependencies]` only for user-facing feature extras (postgres, redis, aws).
**DO**: Pin `requires-python = ">=3.12"` (or your floor). Prevents installs on unsupported versions.
**DON'T**: Use `setup.py` or `setup.cfg` for new projects.
**DON'T**: Use `[tool.uv.dev-dependencies]`. Deprecated in favor of `[dependency-groups]`.
**DON'T**: Mix `requirements.txt` and `pyproject.toml`.
**DON'T**: Commit `uv.lock` for libraries (only for apps/services).

## uv basics

```bash
uv init my-backend # scaffold
uv python pin 3.12
uv add fastapi "uvicorn[standard]" sqlalchemy pydantic-settings
uv add --dev pytest ruff mypy

uv sync # install from pyproject + lock
uv lock # refresh the lock
uv run pytest # run in the project env

# CI / Docker
uv sync --frozen --no-dev
```

Never `pip install` directly inside a uv project: it bypasses the lockfile.
