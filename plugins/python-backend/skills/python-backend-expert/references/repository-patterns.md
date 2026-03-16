# Repository & Service Patterns

Targets SQLAlchemy 2.0+, Advanced Alchemy, Litestar, and FastAPI.

## Repository Pattern

The repository abstracts data access behind a domain-oriented interface. The controller never sees SQLAlchemy -- it sees a clean interface.

### Port (Protocol)

Use `Protocol` for ports -- no inheritance required, structural subtyping, works with third-party classes. Use `ABC` only when you need shared method implementations.

```python
# domain/ports/user_repository.py
from typing import Protocol
from uuid import UUID

class UserRepository(Protocol):
    async def get_by_id(self, id: UUID) -> User | None: ...

    async def get_by_email(self, email: str) -> User | None: ...

    async def create(self, entity: User) -> User: ...

    async def update(self, entity: User) -> User: ...

    async def delete(self, id: UUID) -> None: ...
```

### Implementation

```python
# infrastructure/persistence/user_repository.py
# No inheritance needed -- Protocol is structural
class SqlAlchemyUserRepository:
    def __init__(self, session: AsyncSession) -> None:
        self._session = session

    async def get_by_id(self, id: UUID) -> User | None:
        result = await self._session.execute(
            select(UserModel).where(UserModel.id == id)
        )
        row = result.scalar_one_or_none()
        return self._to_entity(row) if row else None

    @staticmethod
    def _to_entity(model: UserModel) -> User:
        return User(id=model.id, email=model.email, ...)

    @staticmethod
    def _to_model(entity: User) -> UserModel:
        return UserModel(email=entity.email, ...)
```

### Key Rules

1. **Repositories return domain entities, not ORM models.** The `_to_entity` / `_to_model` mapping is essential for decoupling.
2. **Repositories do NOT commit.** They `flush()` to get IDs but leave `commit()` to the service or controller layer.
3. **One repository per aggregate root.** Don't create a repo for every table — create them for domain concepts.
4. **No business logic in repositories.** Repositories are data access, not business rules.

### Advanced Alchemy Repositories

If using Advanced Alchemy, you get the repository pattern for free:

```python
from advanced_alchemy.repository import SQLAlchemyAsyncRepository

class UserRepository(SQLAlchemyAsyncRepository[UserModel]):
    model_type = UserModel

# Advanced Alchemy provides: list, get, create, update, delete,
# count, exists, list_and_count, upsert, and bulk operations
```

Advanced Alchemy repositories handle:
- Optimized bulk insert/upsert operations
- Pagination (limit/offset, cursor-based)
- Filtering with FilterTypes
- Exists checks without loading the full object

### When to Use Advanced Alchemy vs Custom Repos

| Scenario | Recommendation |
|---|---|
| Standard CRUD with filtering | Advanced Alchemy — already optimized |
| Complex custom queries | Custom repository methods |
| Domain entity mapping needed | Custom repo wrapping Advanced Alchemy |
| Clean domain isolation (hexagonal) | Custom repo with ABC/Protocol port |
| Rapid prototyping | Advanced Alchemy directly |

## Service Layer Pattern

Services orchestrate domain logic. They coordinate repositories, apply business rules, and own transaction boundaries.

### Structure

```python
# application/services/user_service.py
class UserService:
    def __init__(
        self,
        repository: IUserRepository,
        encryption: IEncryption,
    ) -> None:
        self._repo = repository
        self._encryption = encryption

    async def create_user(self, email: str, password: str) -> User:
        existing = await self._repo.get_by_email(email)
        if existing is not None:
            raise DuplicateEmailError(email)

        hashed = self._encryption.hash_password(password)
        user = User(email=email, password_hash=hashed)
        return await self._repo.create(user)

    async def change_password(self, user_id: UUID, old: str, new: str) -> None:
        user = await self._repo.get_by_id(user_id)
        if user is None:
            raise UserNotFoundError(user_id)
        if not self._encryption.verify_password(old, user.password_hash):
            raise InvalidPasswordError()
        user.password_hash = self._encryption.hash_password(new)
        await self._repo.update(user)
```

### Service Rules

1. **Services own business logic.** Validation, authorization checks, orchestration.
2. **Services are framework-agnostic.** No `Request`, `Response`, `Controller` imports.
3. **Services accept primitives or domain types.** Not Pydantic schemas (those are entrypoint concerns).
4. **One service per bounded context.** `UserService`, `OrderService`, not `AppService`.

### Transaction Boundaries

Repositories flush (to get IDs), but never commit. The commit happens at the boundary.

**Litestar + Advanced Alchemy:** The `SQLAlchemyPlugin` with `before_send_handler="autocommit"` commits automatically on successful response and rolls back on exception. You only need explicit `commit()` for mid-request transaction control (e.g., commit-then-refresh):

```python
# Litestar -- auto-commit via plugin (most common)
@post("/users")
async def create(self, data: CreateUserRequest, user_service: UserService) -> UserResponse:
    user = await user_service.create_user(data.email, data.password)
    return UserResponse.from_entity(user)
    # Plugin commits after response sent

# Litestar -- explicit commit when needed (e.g., refresh before returning)
@post("/users")
async def create(self, data: CreateUserRequest, db_session: AsyncSession, user_service: UserService) -> UserResponse:
    user = await user_service.create_user(data.email, data.password)
    await db_session.commit()
    return UserResponse.from_entity(user)
```

**FastAPI:** Use a dependency that yields a session and commits/rolls back:

```python
async def get_db() -> AsyncGenerator[AsyncSession, None]:
    async with session_factory() as session:
        yield session
        await session.commit()
```

For background tasks without HTTP context, services manage their own sessions:

```python
class BackgroundService:
    def __init__(self, session_factory: async_sessionmaker) -> None:
        self._session_factory = session_factory

    async def process(self) -> None:
        async with self._session_factory() as session:
            repo = UserRepository(session)
            # ... business logic ...
            await session.commit()
```

## Anti-Pattern: Anemic Domain Model

Services that just pass through to repositories with zero logic:

```python
# BAD — anemic service
class UserService:
    async def get_user(self, id: UUID) -> User:
        return await self._repo.get_by_id(id)  # Just a proxy

    async def create_user(self, data: dict) -> User:
        return await self._repo.create(User(**data))  # Just a proxy
```

If the service adds no value, either:
- Move the repo call directly to the controller (for simple CRUD)
- Add the business logic that should be there (validation, authorization, side effects)
