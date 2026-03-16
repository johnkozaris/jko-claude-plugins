# Dependency Injection

## Why DI Matters

Without DI, classes create their own dependencies — making testing hard, swapping impossible, and coupling tight. With DI, dependencies are provided from outside.

```python
# WITHOUT DI — tight coupling, untestable
class UserService:
    def __init__(self):
        self._repo = UserRepository(get_session())  # Creates its own dep
        self._emailer = SMTPEmailer()                # Hard-coded

# WITH DI — loose coupling, testable
class UserService:
    def __init__(self, repo: IUserRepository, emailer: IEmailer):
        self._repo = repo
        self._emailer = emailer
```

## Litestar Dependency Injection

Litestar uses `Provide()` with callable factories, layered from app -> router -> controller -> handler.

### Provider Functions

```python
# composition root (app.py)
async def provide_user_service(
    db_session: AsyncSession,
    state: State,
) -> UserService:
    return UserService(
        repository=UserRepository(db_session),
        encryption=state.encryption,
    )

app = Litestar(
    dependencies={
        "user_service": Provide(provide_user_service),
    },
)
```

### Controller Injection

```python
class UserController(Controller):
    path = "/api/users"

    @get("/")
    async def list_users(self, user_service: UserService) -> list[UserResponse]:
        users = await user_service.list_all()
        return [UserResponse.from_entity(u) for u in users]
```

### Layered Dependencies

```python
# App-level: shared across all routes
app = Litestar(dependencies={"encryption": Provide(provide_encryption)})

# Router-level: shared within router
router = Router(path="/api", dependencies={"auth": Provide(provide_auth)})

# Controller-level: shared within controller
class UserController(Controller):
    dependencies = {"user_service": Provide(provide_user_service)}
```

### Two-Lifetime Pattern

For apps with both HTTP and background tasks (like BobeCrossPlat):

```
App-Lifetime Singletons (DI container)     Request-Scoped (Litestar Provide)
  - LLM providers                            - db_session (from SQLAlchemy plugin)
  - Config/Settings                          - Repositories (from session)
  - Background task managers                 - Services (from repos)
  - Event streams                            - Auth context
```

## FastAPI Dependency Injection

FastAPI uses `Depends()` with callable functions or classes.

### Depends Pattern

```python
async def get_db() -> AsyncGenerator[AsyncSession, None]:
    async with async_session() as session:
        yield session

async def get_user_service(
    db: AsyncSession = Depends(get_db),
) -> UserService:
    return UserService(repository=UserRepository(db))

@router.post("/users")
async def create_user(
    data: CreateUserRequest,
    service: UserService = Depends(get_user_service),
) -> UserResponse:
    user = await service.create_user(data.email, data.password)
    return UserResponse.from_entity(user)
```

### Dependency Caching

FastAPI caches dependencies within a request scope. If `get_db` is used by multiple sub-dependencies, the same session is returned. Use `use_cache=False` to disable.

### Class-Based Dependencies

```python
class AuthRequired:
    def __init__(self, roles: list[str] | None = None):
        self._roles = roles

    async def __call__(self, request: Request, db: AsyncSession = Depends(get_db)):
        token = request.headers.get("Authorization")
        user = await verify_token(token, db)
        if self._roles and user.role not in self._roles:
            raise HTTPException(403)
        return user

# Usage
@router.get("/admin")
async def admin_panel(user: User = Depends(AuthRequired(roles=["admin"]))):
    ...
```

## DI Anti-Patterns

### 1. Service Locator (anti-pattern)

```python
# BAD — asking a container for dependencies at runtime
class UserService:
    async def create(self):
        repo = container.resolve(IUserRepository)  # Service locator
        # Hides dependencies, hard to test, hard to trace

# GOOD — constructor injection
class UserService:
    def __init__(self, repo: IUserRepository):
        self._repo = repo  # Explicit dependency
```

### 2. Over-Injecting (too many deps)

If a class has >5 constructor parameters, it likely violates SRP:

```python
# BAD — too many responsibilities
class MegaService:
    def __init__(self, user_repo, order_repo, email, cache, auth, logger, metrics, queue):
        ...  # 8 dependencies = 8 reasons to change

# GOOD — split by responsibility
class UserService:
    def __init__(self, repo: IUserRepository, encryption: IEncryption): ...

class OrderService:
    def __init__(self, repo: IOrderRepository, notifier: INotifier): ...
```

### 3. Injecting Session Instead of Repository

```python
# BAD — controller gets raw session
@post("/users")
async def create(self, db: AsyncSession, data: CreateUserRequest):
    db.add(UserModel(**data.model_dump()))  # Controller does DB work

# GOOD — controller gets service
@post("/users")
async def create(self, user_service: UserService, data: CreateUserRequest):
    user = await user_service.create(data.email, data.password)
```

### 4. State Objects as God Containers

```python
# BAD — shoving everything into app.state
app.state.user_repo = UserRepository()
app.state.order_repo = OrderRepository()
app.state.email = EmailService()
app.state.cache = RedisCache()
# No type safety, no IDE support, grows unbounded

# GOOD — use proper DI with typed providers
dependencies={
    "user_service": Provide(provide_user_service),
    "order_service": Provide(provide_order_service),
}
```
