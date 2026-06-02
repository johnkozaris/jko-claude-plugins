# SQLAlchemy & ORM

## SQLAlchemy 2.0+ Patterns

### Model Definition (Mapped Column Style)

```python
from datetime import datetime, UTC
from uuid import UUID, uuid4
from sqlalchemy import String, func
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship

class Base(DeclarativeBase):
    pass

class UserModel(Base):
    __tablename__ = "users"

    id: Mapped[UUID] = mapped_column(primary_key=True, default=uuid4)
    email: Mapped[str] = mapped_column(String(255), unique=True, index=True)
    password_hash: Mapped[str] = mapped_column(String(255))
    is_active: Mapped[bool] = mapped_column(default=True)
    created_at: Mapped[datetime] = mapped_column(server_default=func.now)

    # Relationships default to lazy="raise" so unintended loads fail loudly
    orders: Mapped[list["OrderModel"]] = relationship(back_populates="user", lazy="raise")
```

### Reusable Column Types and Mixins

Factor shared columns (UUID pk, timestamps) into a mixin or `Annotated` aliases rather than repeating them. SQLAlchemy 2.0 supports `Annotated`-based `mapped_column` reuse:

```python
from typing import Annotated

uuid_pk = Annotated[UUID, mapped_column(primary_key=True, default=uuid4)]

class TimestampMixin:
    created_at: Mapped[datetime] = mapped_column(server_default=func.now)
    updated_at: Mapped[datetime] = mapped_column(server_default=func.now, onupdate=func.now)
```

> If the project already uses **Advanced Alchemy** (its `UUIDAuditBase`, `BigIntAuditBase`, repository helpers) with FastAPI, that's fine; stay consistent with it: It is optional, not the default. Don't introduce it just to get a base class.

### Async Engine Setup

```python
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

engine = create_async_engine(url=settings.database_url, # postgresql+asyncpg://user:pw@host/db
echo=False, # Never True in production
pool_pre_ping=True, # SELECT 1 on checkout; survives DB restarts
pool_size=20, # persistent connections per process
max_overflow=10, # burst headroom; total = pool_size + max_overflow
pool_recycle=3600, # recycle after 1h (critical for MySQL/MariaDB)
pool_timeout=30, # wait up to 30s for a free connection
)

session_factory = async_sessionmaker(bind=engine,
class_=AsyncSession,
expire_on_commit=False, # recommended for async. prevents implicit IO on post-commit attribute access
autoflush=False, # Explicit flush for predictable behavior
)
```

`create_async_engine` automatically uses `AsyncAdaptedQueuePool` (the asyncio-safe pool). Passing the sync `QueuePool` raises.

**Pool sizing rule of thumb**: for asyncio, `pool_size = expected_concurrent_DB_queries_per_worker`; usually 10–20 for a typical API. With N Gunicorn workers the DB sees `N × (pool_size + max_overflow)` connections; keep `≤ pg_max_connections − admin_headroom`. For asyncpg with many literal-value query shapes, also pass `connect_args={"statement_cache_size": 0}` to bound memory. For drivers: **asyncpg** and **psycopg3 async** are both production-ready in 2026; asyncpg has a slight raw-throughput edge, psycopg3 has the broader feature set (COPY, more extensions). Pick by what you need; either is a strict upgrade on sync drivers for async services.

See [`performance.md`](performance.md) for the full sizing + driver-choice discussion (asyncpg vs psycopg3 async).

## N+1 Query Prevention

The #1 performance killer in ORMs: Every lazy-loaded relationship in a loop generates a separate query.

### Detection

```python
# BAD: N+1 (1 query for users + N queries for orders)
users = await session.execute(select(UserModel))
for user in users.scalars():
    print(user.orders)  # Each access = 1 query

# GOOD: eager load with selectinload
from sqlalchemy.orm import selectinload

users = await session.execute(
    select(UserModel).options(selectinload(UserModel.orders))
)
```

### Loading Strategy Guide

| Strategy | Use When | SQL Pattern |
|---|---|---|
| `selectinload` | One-to-many, predictable size | `SELECT ... WHERE id IN (...)` |
| `joinedload` | Many-to-one, small relations | `LEFT JOIN` in single query |
| `subqueryload` | Large collections | Separate subquery |
| `lazyload` | **NEVER in async** | Implicit query on access |
| `noload` | Default, explicit opt-in | No loading at all |
| `raiseload` | Catch unintended loads | Raises error on access |

**Rule:** Set `lazy="noload"` or `lazy="raise"` on all relationships by default. Explicitly opt-in to loading in each query.

```python
# Model: noload by default
orders: Mapped[list["OrderModel"]] = relationship(lazy="noload")

# Query: explicit loading
stmt = select(UserModel).options(selectinload(UserModel.orders))
```

## Session Management

### Per-Request Sessions (HTTP)

In FastAPI, yield one `AsyncSession` per request from a dependency and commit/rollback at that boundary:

```python
from collections.abc import AsyncIterator
from fastapi import Depends
from typing import Annotated

async def get_db() -> AsyncIterator[AsyncSession]:
    async with session_factory() as session:
        try:
            yield session
            await session.commit()
        except Exception:
            await session.rollback()
            raise

DbSession = Annotated[AsyncSession, Depends(get_db)]
```

Repositories receive this session and `flush` (never `commit`); the commit happens at the dependency boundary. See the repository reference.

### Per-Operation Sessions (Background Tasks)

```python
class BackgroundWorker:
    def __init__(self, session_factory: async_sessionmaker) -> None:
        self._session_factory = session_factory

    async def process(self) -> None:
        async with self._session_factory() as session:
            repo = MyRepository(session)
            result = await repo.get_all()
            # business logic
            await session.commit()
            # Session closed automatically
```

### Session Rules

1. **Never share sessions across tasks.** Each async task gets its own session.
2. **`expire_on_commit=False`**. Always in async. Prevents implicit lazy loads.
3. **Commit at the boundary.** Controllers commit after the service call completes.
4. **Rollback on error.** Use try/except or let the framework handle it.
5. **Don't hold sessions open.** Close promptly after use.

## Query Patterns

### Select with Filtering

```python
# Modern 2.0 style
stmt = (select(UserModel).where(UserModel.is_active.is_(True)).order_by(UserModel.created_at.desc).limit(20)
)
result = await session.execute(stmt)
users = result.scalars().all
```

### Exists Check (Without Loading)

```python
stmt = select(exists.where(UserModel.email == email))
result = await session.execute(stmt)
email_exists = result.scalar
```

### Pagination

```python
# Offset-based (simple but slow on large datasets)
stmt = select(UserModel).offset(skip).limit(limit)

# Count total
count_stmt = select(func.count).select_from(UserModel)
total = (await session.execute(count_stmt)).scalar

# Cursor-based (efficient for large datasets)
stmt = (select(UserModel).where(UserModel.id > cursor_id).order_by(UserModel.id).limit(limit)
)
```

### Bulk Operations

```python
# BAD: N individual inserts
for item in items:
    session.add(ItemModel(**item))

# GOOD: bulk insert
from sqlalchemy import insert
await session.execute(insert(ItemModel), [item.model_dump() for item in items])
```

## Model Design Rules

1. **ORM models live in infrastructure.** Not in domain. Map to domain entities in the repository.
2. **Use `__tablename__` explicitly.** Don't rely on auto-generation.
3. **Index frequently queried columns.** Especially foreign keys and filter columns.
4. **Use server-side defaults.** `server_default=func.now` over a Python `default=lambda: datetime.now(UTC)`.
5. **Prefer `mapped_column` over `Column`.** SQLAlchemy 2.0 style.
6. **Don't use `backref`.** Use explicit `relationship(..., back_populates="...")` on both sides.
