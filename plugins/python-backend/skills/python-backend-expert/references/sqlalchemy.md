# SQLAlchemy & ORM

## SQLAlchemy 2.0+ Patterns

### Model Definition (Mapped Column Style)

```python
from advanced_alchemy.base import UUIDAuditBase
from sqlalchemy.orm import Mapped, mapped_column, relationship
from sqlalchemy import String, ForeignKey, Text
from uuid import UUID

class UserModel(UUIDAuditBase):
    __tablename__ = "users"

    email: Mapped[str] = mapped_column(String(255), unique=True, index=True)
    password_hash: Mapped[str] = mapped_column(String(255))
    is_active: Mapped[bool] = mapped_column(default=True)

    # Relationships
    orders: Mapped[list["OrderModel"]] = relationship(back_populates="user", lazy="noload")
```

### Base Classes (Advanced Alchemy)

| Base Class | Provides |
|---|---|
| `UUIDAuditBase` | UUID pk, created_at, updated_at |
| `UUIDBase` | UUID pk only |
| `BigIntAuditBase` | BigInt pk, created_at, updated_at |
| `BigIntBase` | BigInt pk only |
| `SlugKey` | Mixin adding slug column |

### Async Engine Setup

```python
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker, AsyncSession

engine = create_async_engine(
    url=settings.database_url,
    echo=False,            # Never True in production
    pool_pre_ping=True,    # Detect stale connections
    pool_size=10,          # Tune per workload
    max_overflow=20,
)

session_factory = async_sessionmaker(
    bind=engine,
    class_=AsyncSession,
    expire_on_commit=False,  # CRITICAL for async — prevents lazy loads after commit
    autoflush=False,         # Explicit flush for predictable behavior
)
```

## N+1 Query Prevention

The #1 performance killer in ORMs. Every lazy-loaded relationship in a loop generates a separate query.

### Detection

```python
# BAD — N+1 (1 query for users + N queries for orders)
users = await session.execute(select(UserModel))
for user in users.scalars():
    print(user.orders)  # Each access = 1 query

# GOOD — eager load with selectinload
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

Litestar's SQLAlchemy plugin provides a session per request automatically. With Advanced Alchemy:

```python
from advanced_alchemy.extensions.litestar import SQLAlchemyAsyncConfig, SQLAlchemyPlugin

db_config = SQLAlchemyAsyncConfig(engine_instance=engine)
db_plugin = SQLAlchemyPlugin(config=db_config)

# Session injected as `db_session: AsyncSession` in handlers
```

### Per-Operation Sessions (Background Tasks)

```python
class BackgroundWorker:
    def __init__(self, session_factory: async_sessionmaker) -> None:
        self._session_factory = session_factory

    async def process(self) -> None:
        async with self._session_factory() as session:
            repo = MyRepository(session)
            result = await repo.get_all()
            # ... business logic ...
            await session.commit()
        # Session closed automatically
```

### Session Rules

1. **Never share sessions across tasks.** Each async task gets its own session.
2. **`expire_on_commit=False`** — Always in async. Prevents implicit lazy loads.
3. **Commit at the boundary.** Controllers commit after the service call completes.
4. **Rollback on error.** Use try/except or let the framework handle it.
5. **Don't hold sessions open.** Close promptly after use.

## Query Patterns

### Select with Filtering

```python
# Modern 2.0 style
stmt = (
    select(UserModel)
    .where(UserModel.is_active.is_(True))
    .order_by(UserModel.created_at.desc())
    .limit(20)
)
result = await session.execute(stmt)
users = result.scalars().all()
```

### Exists Check (Without Loading)

```python
stmt = select(exists().where(UserModel.email == email))
result = await session.execute(stmt)
email_exists = result.scalar()
```

### Pagination

```python
# Offset-based (simple but slow on large datasets)
stmt = select(UserModel).offset(skip).limit(limit)

# Count total
count_stmt = select(func.count()).select_from(UserModel)
total = (await session.execute(count_stmt)).scalar()

# Cursor-based (efficient for large datasets)
stmt = (
    select(UserModel)
    .where(UserModel.id > cursor_id)
    .order_by(UserModel.id)
    .limit(limit)
)
```

### Bulk Operations

```python
# BAD — N individual inserts
for item in items:
    session.add(ItemModel(**item))

# GOOD — bulk insert
from sqlalchemy import insert
await session.execute(insert(ItemModel), [item.model_dump() for item in items])

# With Advanced Alchemy
repo = ItemRepository(session=session)
await repo.create_many(items)
```

## Model Design Rules

1. **ORM models live in infrastructure.** Not in domain. Map to domain entities in the repository.
2. **Use `__tablename__` explicitly.** Don't rely on auto-generation.
3. **Index frequently queried columns.** Especially foreign keys and filter columns.
4. **Use server-side defaults.** `server_default=func.now()` over Python `default=datetime.utcnow`.
5. **Prefer `mapped_column` over `Column`.** SQLAlchemy 2.0 style.
6. **Don't use `backref`.** Use explicit `relationship(..., back_populates="...")` on both sides.
