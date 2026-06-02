# Architecture & Boundaries

Pick one of two layouts. Resist the third (full hexagonal) unless there's a written reason. Apply the boundary rules below to whichever layout fits.

## The Two Layouts That Actually Work

### Layout A: File-type, flat. Use for ≤ ~5 domains.

What `fastapi/full-stack-fastapi-template` (the official template, 28k+ stars) actually uses. Routes per resource, a flat `models.py`, a flat `crud.py`. No `service.py`, no per-domain folders. This is correct for two- to five-domain apps. It is *not* an under-architected mistake; it's the right choice at that scale.

```
app/
  api/
    deps.py             # Annotated[T, Depends(...)] aliases
    routes/
      users.py
      items.py
      login.py
  core/
    config.py
    security.py
  models.py             # ORM models (flat)
  schemas.py            # Pydantic DTOs (flat)
  crud.py               # data-access functions (flat)
  main.py
tests/
```

### Layout B: Per-domain. Use for >5 domains.

Each domain owns a folder; cross-domain access goes through the module's public name.

```
src/
  core/                 # cross-cutting only: config, database, exceptions, logging
  auth/                 # router.py service.py models.py schemas.py dependencies.py exceptions.py
  posts/
  payments/
  main.py
tests/
```

The cognitive cost argument is concrete: in file-type layout, finding "the code that handles incident deletion" means navigating three top-level directories (`controllers/`, `services/`, `models/`). In domain-module, it's one (`incident/`). For 30 domains, that's O(1) vs O(k=3) per lookup, multiplied by every onboarding engineer, forever.

**Cross-domain imports go through the module, never deep into it.**

```python
# CORRECT
from src.auth import service as auth_service

# WRONG. welds the caller to internal file structure
from src.auth.service.user import create_access_token
```

### Layout C: Full Hexagonal / Clean. Almost always overkill.

Hexagonal (Cockburn) and Clean (Martin) are correct architecture for a specific situation: domain logic that must run unchanged under multiple entrypoints (HTTP + CLI + workers + gRPC) **with a team that has explicit DDD experience**. For a typical FastAPI CRUD-heavy service, the full implementation costs more than it pays back. The concrete cost: a single "create incident" command travels through `inbound/http/routers/incident_router.py → application/commands/create_incident.py → core/commands/handlers/create_incident_handler.py → outbound/persistence/repositories/sqlalchemy_incident_repository.py`. Versus Layout B's `incident/router.py → incident/service.py`. Both are correct; only one is boring, fast to navigate, and trivial to onboard.

**Use Layout C only when you can name a second entrypoint that already exists in production** (not "we might add a CLI later"). Otherwise the layering ceremony pays no dividend.

For teams that genuinely need it: pair it with `import-linter` (1k+ stars, actively maintained). Without machine-enforced layer contracts, hexagonal rots in six months; someone imports `infrastructure` from `domain`, the dependency arrow inverts, and the architecture is gone with no warning.

## The Hexagonal Misapplication You'll See

Two patterns get cargo-culted from "hexagonal in Python" tutorials. Reject both.

**1. "Make Pydantic schemas framework-free."** They cannot be. Pydantic is a hard dependency. The only consistent hexagonal-in-Python answer is to keep `core/` entirely free of both FastAPI and Pydantic, and have plain dataclass domain types that get mapped to Pydantic at the inbound adapter. This costs a translation step on every request and a duplicated type definition for every domain object. For a typical CRUD service, this is pure overhead. Repos that do it correctly (e.g. `ivan-borovets/fastapi-clean-example`) end up much larger than the same app would be in Layout B.

**2. "Wrap SQLAlchemy `Session` in `AbstractRepository`."** SQLAlchemy 2.0's `AsyncSession` *is* the repository. The `async with session.begin():` block *is* the Unit of Work. Adding a `class SQLAlchemyUserRepository(AbstractUserRepository)` that delegates every method to the session is layer ceremony with no abstraction value; you cannot swap SQLAlchemy out without rewriting the query DSL anyway. Use the session directly inside service functions. The Repository + Unit-of-Work pattern from architecture books is correct for the case it describes (multiple persistence backends, ORM-independent domain). Overkill for the 95th-percentile FastAPI app, which has one Postgres and one ORM forever.

## Layer Rules (Apply to Both Layouts)

| Rule | What it means |
| --- | --- |
| **Routes are thin.** | Validate input (Pydantic does this), call a service, shape output. No `if/else` business logic in a route. |
| **Services own transactions and orchestration.** | They accept primitives or domain types; never `Request`/`Response`/raw `AsyncSession` leaking in from above. |
| **Domain never imports infrastructure.** | Domain defines what it needs as a `Protocol`; infrastructure implements it. |
| **Infrastructure adapts.** | Repositories implement ports, map ORM rows to domain entities. ORM models live in infrastructure, not in domain. |
| **Auth lives at the dependency boundary.** | Use `APIRouter(prefix="/admin", dependencies=[Depends(require_admin)])`. Never `if not current_user.is_admin: raise` inline in every route. |
| **Cross-domain imports at module level.** | `from src.auth import service as auth_service`. No reaching into other domains' internals. |

## The Boundary Violations to Reject on Sight

| Violation | Smell | Fix |
| --- | --- | --- |
| ORM model imported from domain | `from infrastructure.persistence import UserModel` in `domain/` | Map to a domain entity in the repository |
| Business logic in a route | `if user.role == "admin" and order.total >...` in a handler | Move to service or a domain method |
| Service accepts `Request` | `def create_user(self, request: Request)` | Service accepts plain values or DTOs |
| Service raises `HTTPException` | `raise HTTPException(404,...)` in `service.py` | Raise typed domain error; map in `@app.exception_handler` |
| Direct DB access in a router | `session.execute(select(Model))` in a handler | Service or repository, injected via `Depends` |
| Pydantic models as domain entities | Domain `class Order(BaseModel)` with `@field_validator` for business rules | `@dataclass` for the domain; Pydantic only at the boundary |
| Layered structure with no second adapter | `AbstractUserRepository` with exactly one implementation | Delete the abstract class; use the concrete one directly |

## Composition Root

For both layouts, the composition root is `main.py` plus the `lifespan` context manager (for app-lifetime singletons) and the `Depends` graph (for request-scoped objects).

```python
# main.py. the only place app-lifetime objects are constructed
from contextlib import asynccontextmanager
from collections.abc import AsyncIterator
from fastapi import FastAPI

@asynccontextmanager
async def lifespan(app: FastAPI) -> AsyncIterator[dict]:
    engine = create_async_engine(settings.database_url, pool_pre_ping=True)
    http = httpx.AsyncClient
    try:
        yield {"http": http} # app-lifetime → request.state
    finally:
        await http.aclose()
        await engine.dispose()

app = FastAPI(lifespan=lifespan)
app.include_router(auth_router)
app.include_router(posts_router)
```

Request-scoped wiring (session → repository → service) lives in each domain's `dependencies.py` via `Annotated[T, Depends(...)]`. **Do not** install a third-party DI container (`dependency-injector`, `punq`, `dishka`) unless the app genuinely outgrows `Depends`. `Protocol` ports + `Depends` covers the vast majority of cases and keeps tests trivial (`app.dependency_overrides`).

## When to Split a Domain

Concrete signals:

1. The domain's `service.py` passes ~400 lines and has functionally distinct sections.
2. Two features of the domain share zero models. That's an aggregate-root split waiting to happen.
3. Another domain imports from this domain more than from any other. That's a coupling signal; the imported code may belong somewhere else.

The mechanical move: create the new package, move files in, update the imports. No business-logic rewrite required.

## Real Repos Look Boring, and That's Correct

The production FastAPI services you can read on GitHub are boring: flat service functions, explicit DB session parameters, flat route files, minimal abstraction. They're not under-engineered. They're structured exactly as complex as the problem demands and not one layer more.

The trendy alternatives (full CQRS with event sourcing, hexagonal with `Abstract*` ports everywhere, actor-model domain events) require sophisticated engineers to maintain and add real latency to every onboarding. New engineer's question: "where does `POST /incidents` write to the database?" In Layout B the answer is `incident/service.py → create`. In full hexagonal it's a four-file trace through commands, handlers, repositories, and adapters. Both are architecturally correct. Only one is boring, and boring scales.

Pick boring by default. Earn the complexity.

## Quick Reference

| Decision | Default |
| --- | --- |
| Layout for ≤ 5 domains | A (file-type, flat models.py / crud.py) |
| Layout for > 5 domains | B (per-domain) |
| Layout C (hexagonal) | Only with multiple existing entrypoints AND `import-linter` |
| Repository class wrapping `AsyncSession` | Skip; the session is the repository |
| Service signature | `async def create(db: AsyncSession, …)` with plain types |
| Auth check | Router-level `dependencies=[Depends(require_admin)]` |
| Domain → HTTP error mapping | `@app.exception_handler(DomainError)`, never in services |
| Domain entity type | `@dataclass(slots=True, frozen=True)` (not Pydantic) |
| DI container | None. `Depends` + `Protocol` |
| Settings | Per-domain `BaseSettings` with `env_prefix=`, or one `core/config.py` if small |

For code-level DOs/DON'Ts (class design, data structure choice, error hierarchies, naming) see [`modern-python.md`](modern-python.md). For validation patterns, [`validation.md`](validation.md). For folder structure, [`project-structure.md`](project-structure.md).
