# Architecture & Boundaries

## Hexagonal Architecture (Ports & Adapters)

The gold standard for Python backends. The domain (business logic) sits at the center, protected by ports (interfaces) and adapters (implementations). Dependencies always point inward.

### Layer Model

```
Entrypoints (HTTP controllers, CLI, MCP, WebSocket)
    |
    v
Application Services (orchestration, use cases)
    |
    v
Domain (entities, value objects, domain services, ports)
    ^
    |
Infrastructure (repositories, clients, adapters, ORM models)
```

### Rules

1. **Domain layer imports NOTHING from infrastructure or entrypoints.** Domain defines ports (Protocol/ABC); infrastructure implements them.
2. **Entrypoints are thin.** Controllers validate input, call services, format output. No business logic.
3. **Application services orchestrate.** They coordinate domain objects and call ports. They own transaction boundaries.
4. **Infrastructure adapts.** Repositories implement port interfaces. ORM models live here, not in domain.

### Recommended Directory Structure

```
src/
  app_name/
    domain/
      entities/          # Domain models (dataclasses, not ORM models)
      ports/             # Protocol/ABC interfaces
      exceptions.py      # Domain exception hierarchy
      validation.py      # Domain validation rules
    application/
      services/          # Use-case orchestrators
      protocols/         # Application-layer protocols (if distinct from domain)
    entrypoints/
      http/
        controllers/     # Litestar Controllers or FastAPI routers
        schemas/         # Request/response Pydantic models
        dependencies.py  # DI wiring
        middleware.py     # Auth, CORS, rate limiting
      cli/               # CLI commands
      mcp/               # MCP server entrypoint
    infrastructure/
      persistence/
        orm_models.py    # SQLAlchemy ORM models
        repositories/    # Repository implementations
      clients/           # External API clients
      encryption/        # Encryption adapters
    composition/
      container.py       # DI container / composition root
      settings.py        # Pydantic Settings
      bootstrap.py       # App startup orchestration
    shared/
      errors.py          # Shared error utilities
      logging.py         # Logging setup
```

### The Framework Swap Test

Ask: "If I replaced Litestar with FastAPI tomorrow, what would change?"

**Good answer:** Only `entrypoints/http/` and `composition/` change. Domain, services, repositories untouched.
**Bad answer:** Business logic scattered across controllers means rewriting everything.

### Common Boundary Violations

| Violation | Signal | Fix |
|---|---|---|
| ORM model in domain | `from infrastructure.persistence import UserModel` in domain | Create domain entity, map in repository |
| Business logic in controller | `if user.role == "admin" and ...` in handler | Move to service or domain method |
| Infrastructure in domain port | Port method returns `AsyncSession` | Port returns domain entity |
| Framework types in service | Service accepts `Request` object | Service accepts plain values or DTOs |
| Direct DB access in handler | `db.execute(select(Model))` in controller | Use repository via DI |

### Composition Root

Wire all dependencies in ONE place — the composition root. This is where abstractions meet their concrete implementations.

**Litestar pattern:**
```python
# app.py — composition root
app = Litestar(
    dependencies={
        "encryption": Provide(provide_encryption),
        "service_manager": Provide(provide_service_manager),
        "audit_service": Provide(provide_audit_service),
    },
)
```

**dependency-injector pattern (for complex apps with background tasks):**
```python
# container.py — composition root
class Container(containers.DeclarativeContainer):
    config = providers.Singleton(Settings)
    session_factory = providers.Singleton(async_sessionmaker, ...)
    service = providers.Singleton(MyService, repo=repo, ...)
```

Both are valid. The key: **dependencies flow from composition root, not from imports.**
