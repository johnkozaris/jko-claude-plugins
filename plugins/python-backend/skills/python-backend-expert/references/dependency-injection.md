# Dependency Injection

## Why DI Matters

Without DI, classes create their own dependencies. Testing becomes hard, swapping implementations is impossible, and coupling is tight. With DI, dependencies are passed in. Tests can substitute fakes; production can substitute the real thing; nothing changes in the class itself.

```python
# WITHOUT DI. tight coupling, untestable
class UserService:
    def __init__(self):
        self._repo = UserRepository(get_session) # Creates its own dep
        self._emailer = SMTPEmailer # Hard-coded

# WITH DI. loose coupling, testable
class UserService:
    def __init__(self, repo: IUserRepository, emailer: IEmailer):
        self._repo = repo
        self._emailer = emailer
```

## FastAPI Dependency Injection

FastAPI uses `Depends` with callable functions or classes. Since FastAPI 0.95 the idiomatic form is `Annotated[T, Depends(...)]`; the legacy `T = Depends(...)` default-argument form has gotchas with real default values and should be avoided.

### The `Annotated` Form (preferred)

Declare reusable dependency aliases once and reuse them across routes:

```python
from typing import Annotated
from collections.abc import AsyncIterator
from fastapi import Depends
from sqlalchemy.ext.asyncio import AsyncSession

async def get_db() -> AsyncIterator[AsyncSession]:
    async with SessionFactory() as session:
        yield session
        await session.commit()

DbSession = Annotated[AsyncSession, Depends(get_db)]

async def get_user_service(db: DbSession) -> UserService:
    return UserService(repository=SqlAlchemyUserRepository(db))

UserServiceDep = Annotated[UserService, Depends(get_user_service)]

@router.post("/users", response_model=UserResponse)
async def create_user(data: CreateUserRequest, service: UserServiceDep):
    return await service.create_user(data.email, data.password)
```

### Dependencies for Validation, Not Just Injection

Dependencies are the right place for checks that need the DB or external services and should short-circuit with the correct HTTP error. Keep routes thin and reuse the check across endpoints:

```python
async def valid_post_id(post_id: UUID) -> Post:
    post = await service.get_by_id(post_id)
    if post is None:
        raise PostNotFound  # → 404
    return post

async def valid_owned_post(
    post: Annotated[Post, Depends(valid_post_id)],
    user: Annotated[User, Depends(current_user)],
) -> Post:
    if post.owner_id != user.id:
        raise NotOwner  # → 403
    return post
```

### Dependency Caching: Chain Freely

FastAPI caches each `Depends(x)` result within a single request. If `parse_jwt_data` feeds three other dependencies, it runs once, not three times. This makes small single-responsibility dependencies cheap to compose. Pass `use_cache=False` to opt out.

### Make Dependencies `async` When They Do No I/O

A `def` dependency is dispatched to the 40-thread pool even for trivial computation, wasting a thread under load. Declare pure-compute dependencies `async def` so they run on the event loop directly.

### Class-Based Dependencies (parameterized)

```python
class RoleRequired:
    def __init__(self, *roles: str) -> None:
        self._roles = roles

    async def __call__(self, user: Annotated[User, Depends(current_user)]) -> User:
        if self._roles and user.role not in self._roles:
            raise HTTPException(status_code=403, detail="forbidden")
        return user

@router.get("/admin")
async def admin_panel(user: Annotated[User, Depends(RoleRequired("admin"))]):
    ...
```

## DI Anti-Patterns

### 1. Service Locator (anti-pattern)

```python
# BAD. asking a container for dependencies at runtime
class UserService:
    async def create(self):
        repo = container.resolve(IUserRepository) # Service locator
        # Hides dependencies, hard to test, hard to trace

# GOOD. constructor injection
class UserService:
    def __init__(self, repo: IUserRepository):
        self._repo = repo # Explicit dependency
```

### 2. Over-Injecting (too many deps)

If a class has >5 constructor parameters, it likely violates SRP:

```python
# BAD. too many responsibilities
class MegaService:
    def __init__(self, user_repo, order_repo, email, cache, auth, logger, metrics, queue):...
# 8 dependencies = 8 reasons to change

# GOOD. split by responsibility
class UserService:
    def __init__(self, repo: IUserRepository, encryption: IEncryption):...
class OrderService:
    def __init__(self, repo: IOrderRepository, notifier: INotifier):...
```

### 3. Injecting Session Instead of Repository

```python
# BAD. route gets a raw session and does DB work
@router.post("/users")
async def create(data: CreateUserRequest, db: DbSession):
    db.add(UserModel(**data.model_dump())) # naked ORM in the router

# GOOD. route gets a service
@router.post("/users", response_model=UserResponse)
async def create(data: CreateUserRequest, service: UserServiceDep):
    return await service.create_user(data.email, data.password)
```

### 4. `app.state` as a God Container

```python
# BAD: shoving everything into app.state, untyped, unbounded
app.state.user_repo = UserRepository()
app.state.order_repo = OrderRepository()
app.state.email = EmailService()
app.state.cache = RedisCache()

# GOOD: wire singletons in lifespan, request-scoped deps via Depends
async def get_user_service(db: DbSession) -> UserService:
    return UserService(SqlAlchemyUserRepository(db))

UserServiceDep = Annotated[UserService, Depends(get_user_service)]
```

Use lifespan state for genuinely app-lifetime singletons (one HTTP client, one engine), and `Depends` for request-scoped objects (session, repositories, services). Don't pile per-resource attributes onto `app.state`.
