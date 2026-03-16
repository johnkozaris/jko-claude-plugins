# Project Structure

## Recommended Layout

### Hexagonal / Clean Architecture (Preferred)

```
project_root/
  backend/
    pyproject.toml
    alembic/
      env.py
      versions/
    src/
      app_name/
        __init__.py
        __main__.py            # Entry point
        main.py                # App creation (or app.py)

        domain/
          __init__.py
          entities/             # Domain models (dataclasses)
            __init__.py
            user.py
            order.py
          ports/                # Protocol/ABC interfaces
            __init__.py
            user_repository.py
            encryption.py
            email_service.py
          exceptions.py         # Domain exception hierarchy
          validation.py         # Domain validation rules

        application/
          __init__.py
          services/             # Business logic orchestrators
            __init__.py
            user_service.py
            order_service.py

        entrypoints/
          __init__.py
          http/
            __init__.py
            controllers/        # Litestar Controllers or FastAPI routers
              __init__.py
              users.py
              orders.py
              health.py
            schemas/            # Request/response Pydantic models
              __init__.py
              users.py
              orders.py
            dependencies.py     # DI wiring for HTTP layer
            middleware.py
            exception_handlers.py
          cli/
            __init__.py
            commands.py

        infrastructure/
          __init__.py
          persistence/
            __init__.py
            orm_models.py       # SQLAlchemy models (single file or split)
            repositories/
              __init__.py
              user_repository.py
              order_repository.py
          clients/              # External HTTP/API clients
            __init__.py
            base_client.py
            github_client.py
          encryption/
            __init__.py
            fernet_encryption.py

        composition/            # Wiring layer
          __init__.py
          settings.py           # Pydantic Settings
          container.py          # DI container (if using dependency-injector)
          bootstrap.py          # App startup sequence

        shared/                 # Cross-cutting utilities
          __init__.py
          logging.py
          errors.py

    tests/
      conftest.py
      unit/
        test_user_service.py
      integration/
        test_user_repository.py
      api/
        test_user_endpoints.py
```

## File Size Guidelines

| Component Type | Target | Max | Split Signal |
|---|---|---|---|
| Controller | 50-150 | 300 | Multiple unrelated resource groups |
| Service | 50-200 | 400 | Methods spanning multiple sub-domains |
| Repository | 30-100 | 200 | >10 custom query methods |
| Schema file | 20-100 | 200 | Schemas for unrelated endpoints |
| ORM models | 20-80 per model | 300 total | Always one model per file if >3 models |
| Settings | 30-80 | 150 | Split into sub-settings classes |
| **Any module** | --- | **500** | **Always split above 500 lines** |

## Module Organization Rules

### 1. One Class Per File (For Major Classes)

```
# BAD -- everything in one file
services.py  # UserService, OrderService, PaymentService (800 lines)

# GOOD -- one service per file
services/
  __init__.py
  user_service.py
  order_service.py
  payment_service.py
```

### 2. Group by Layer, Then by Domain

```
# BAD -- flat structure
controllers.py
services.py
repositories.py
models.py
schemas.py

# GOOD -- layered with domain grouping
entrypoints/http/controllers/users.py
entrypoints/http/controllers/orders.py
application/services/user_service.py
application/services/order_service.py
infrastructure/persistence/repositories/user_repository.py
```

### 3. __init__.py as Public API

```python
# infrastructure/persistence/__init__.py
# Re-export what consumers need
from .repositories.user_repository import UserRepository
from .repositories.order_repository import OrderRepository

__all__ = ["UserRepository", "OrderRepository"]
```

### 4. Avoid Deep Nesting

Max 4 levels deep: `src/app/layer/sublayer/module.py`. If deeper, flatten.

```
# BAD -- too deep
src/app/infrastructure/persistence/postgres/repositories/users/queries/complex.py

# GOOD -- flatten
src/app/infrastructure/persistence/user_repository.py
```

## Import Order Convention (PEP 8 + isort)

```python
# 1. Standard library
import asyncio
from collections.abc import Sequence
from uuid import UUID

# 2. Third-party
import structlog
from litestar import Controller, get, post
from sqlalchemy.ext.asyncio import AsyncSession

# 3. Local application
from domain.entities.user import User
from domain.exceptions import UserNotFoundError
from domain.ports.user_repository import IUserRepository
```

## pyproject.toml -- The Single Source of Truth

`pyproject.toml` replaces `setup.py`, `setup.cfg`, `MANIFEST.in`, and `requirements.txt`. One file. Declarative. No executable code.

### Production Backend Template

```toml
[project]
name = "my-backend"
version = "0.1.0"
description = "My backend service"
requires-python = ">=3.14"
dependencies = [
    "litestar[sqlalchemy]>=2.21",   # or "fastapi>=0.135"
    "advanced-alchemy>=1.8",
    "pydantic-settings>=2.7",
    "structlog>=25.1",
    "httpx>=0.28",
    "uvicorn>=0.34",
]

[project.optional-dependencies]
# User-facing feature extras (installed via pip install my-backend[postgres])
postgres = ["asyncpg>=0.30"]
mysql = ["asyncmy>=0.2"]

[dependency-groups]
# Dev-only -- never published to PyPI
dev = [
    "pytest>=8.0",
    "pytest-asyncio>=1.0",
    "ruff>=0.9",
    "mypy>=1.14",
    "testcontainers[postgresql]>=4.0",
]

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.ruff]
line-length = 100
target-version = "py314"

[tool.ruff.lint]
select = ["E", "F", "I", "N", "W", "UP", "B", "A", "C4", "SIM", "TCH", "RUF"]

[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]

[tool.mypy]
python_version = "3.14"
strict = true
plugins = ["pydantic.mypy", "sqlalchemy.ext.mypy.plugin"]
```

### pyproject.toml Rules

**DO**: Always include `[build-system]` -- without it, uv won't install the project itself
**DO**: Use `[project]` table for all metadata (PEP 621) -- not `[tool.poetry]`
**DO**: Use `[dependency-groups]` for dev tooling (pytest, ruff, mypy)
**DO**: Use `[project.optional-dependencies]` for user-facing feature extras (postgres, redis)
**DO**: Use SPDX license expressions (`license = "MIT"`) per PEP 639
**DO**: Pin `requires-python = ">=3.14"` to prevent running on unsupported versions
**DON'T**: Use `setup.py` or `setup.cfg` for new projects -- `pyproject.toml` is the standard
**DON'T**: Use `[tool.poetry]` for metadata on new projects -- Poetry 2.0+ supports `[project]`
**DON'T**: Mix `requirements.txt` and `pyproject.toml` -- use one source of truth
**DON'T**: Put dev dependencies in `[project.dependencies]` -- they bloat production installs
**DON'T**: Use `..` parent directory paths in `pyproject.toml` -- paths are relative to the file
**DON'T**: Forget `build-backend` value must match `requires` (e.g., `hatchling` -> `hatchling.build`)

### `optional-dependencies` vs `dependency-groups`

| Feature | `[project.optional-dependencies]` | `[dependency-groups]` |
|---|---|---|
| Purpose | User-facing feature extras | Dev-only tooling |
| Published to PyPI | Yes | No |
| Install syntax | `pip install pkg[extra]` | `uv sync --group dev` |
| Examples | `postgres`, `redis`, `aws` | `dev`, `test`, `lint`, `docs` |

## uv -- Package Management

Use `uv` exclusively. Never `pip install`. Never install packages globally. uv manages Python versions, virtual environments, dependencies, and lockfiles in one tool.

### Project Lifecycle

```bash
# Start a new project
uv init my-backend               # Scaffolds pyproject.toml, .python-version, .gitignore
uv python pin 3.14               # Pin Python version
uv add litestar[sqlalchemy]      # Add production dependency
uv add --dev pytest ruff mypy    # Add dev-only dependency
uv add --optional postgres asyncpg  # Add to optional-dependencies

# Day-to-day development
uv run python -m my_app          # Run with correct env (auto-syncs)
uv run pytest                    # Run tests with correct env
uv run ruff check src/           # Run linter with correct env
uv sync                          # Sync env from lockfile explicitly

# Dependency management
uv lock                          # Update lockfile from pyproject.toml
uv lock --upgrade-package httpx  # Upgrade one package only
uv remove old-package            # Remove a dependency
uv add -r requirements.txt       # Migrate from legacy requirements.txt

# Docker
uv sync --frozen --no-dev        # Production install (no dev deps, no lockfile update)
```

### uv Rules

**DO**: Commit `uv.lock` to version control -- it ensures reproducible builds across machines
**DO**: Use `uv run` for everything -- it auto-syncs the env before execution
**DO**: Use `.python-version` file -- collaborators auto-get the right interpreter on `uv sync`
**DO**: Use `uv sync --frozen --no-dev` in Docker -- fast, reproducible, no dev bloat
**DO**: Use `uv lock --upgrade-package <pkg>` for targeted updates without touching the rest
**DO**: Use `uv add --dev` for test/lint/type-check tooling
**DON'T**: Edit `uv.lock` manually -- it's machine-managed
**DON'T**: Run `python script.py` directly -- you might hit system Python, not the project venv
**DON'T**: Use `pip install` in uv-managed projects -- it bypasses the lockfile entirely
**DON'T**: Use `pip install --user` or global installs -- isolate everything in projects
**DON'T**: Skip `[build-system]` -- uv needs it to install the project package itself
**DON'T**: Manually activate venvs (`source .venv/bin/activate`) -- `uv run` handles it
**DON'T**: Nest `uv init` inside a repo that already has a `pyproject.toml` -- it creates an unintended workspace

