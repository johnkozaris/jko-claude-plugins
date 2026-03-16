# SOLID Principles for Python Backends

## Single Responsibility Principle (SRP)

A class should have one reason to change. In Python backends, this means:

### Controllers: Thin Entry Points Only

```python
# BAD — controller does validation, business logic, and formatting
class UserController(Controller):
    @post("/users")
    async def create(self, data: CreateUserRequest, db: AsyncSession) -> UserResponse:
        if await db.execute(select(User).where(User.email == data.email)):
            raise ClientException("Email exists")
        hashed = bcrypt.hash(data.password)
        user = User(email=data.email, password=hashed)
        db.add(user)
        await db.commit()
        await send_welcome_email(user.email)
        return UserResponse(id=user.id, email=user.email)

# GOOD — controller delegates to service
class UserController(Controller):
    @post("/users")
    async def create(self, data: CreateUserRequest, user_service: UserService) -> UserResponse:
        user = await user_service.create_user(data.email, data.password)
        return UserResponse.from_entity(user)
```

### Services: One Business Domain

```python
# BAD — service handles users AND email AND encryption
class UserService:
    async def create_user(self, ...): ...
    async def send_email(self, ...): ...
    async def encrypt_token(self, ...): ...

# GOOD — separate responsibilities
class UserService:
    def __init__(self, repo: IUserRepository, encryption: IEncryption):
        self._repo = repo
        self._encryption = encryption

    async def create_user(self, email: str, password: str) -> User: ...

class EmailService:
    async def send_welcome(self, email: str) -> None: ...
```

### File Size Guideline

| Component | Target Lines | Max Lines | Signal to Split |
|---|---|---|---|
| Controller | 50-150 | 300 | Multiple unrelated endpoint groups |
| Service | 50-200 | 400 | Methods for different sub-domains |
| Repository | 30-100 | 200 | Custom queries growing beyond CRUD |
| Schema file | 20-100 | 200 | Schemas for unrelated endpoints |
| Any module | --- | 500 | Always split above 500 lines |

## Open/Closed Principle (OCP)

Open for extension, closed for modification. In Python:

```python
# BAD — adding a new service type requires modifying existing code
def create_client(service_type: str):
    if service_type == "github":
        return GitHubClient()
    elif service_type == "gitlab":
        return GitLabClient()
    # Must modify this function for every new type

# GOOD — use Protocol + immutable registry
from types import MappingProxyType

class ServiceClient(Protocol):
    async def health_check(self) -> bool: ...

_CLIENT_REGISTRY: MappingProxyType[str, type[ServiceClient]] = MappingProxyType({
    "github": GitHubClient,
    "gitlab": GitLabClient,
})

def create_client(service_type: str) -> ServiceClient:
    cls = _CLIENT_REGISTRY.get(service_type)
    if cls is None:
        raise UnsupportedServiceError(service_type)
    return cls()
```

## Liskov Substitution Principle (LSP)

Subtypes must be substitutable for their base types. In Python backends:

```python
# BAD — subclass changes method signature semantics
class BaseRepository:
    async def get_by_id(self, id: UUID) -> Entity | None: ...

class CachingRepository(BaseRepository):
    async def get_by_id(self, id: UUID) -> Entity:  # Never returns None!
        # Raises if not found — violates LSP
        result = await super().get_by_id(id)
        if result is None:
            raise NotFoundError()
        return result

# GOOD — maintain the contract
class CachingRepository(BaseRepository):
    async def get_by_id(self, id: UUID) -> Entity | None:
        cached = self._cache.get(id)
        if cached is not None:
            return cached
        result = await super().get_by_id(id)
        if result is not None:
            self._cache.set(id, result)
        return result
```

## Interface Segregation Principle (ISP)

Prefer small, focused interfaces over large ones. Use Python's Protocol for structural subtyping:

```python
# BAD — one massive repository interface
class IRepository(Protocol):
    async def get_all(self) -> list[Entity]: ...
    async def get_by_id(self, id: UUID) -> Entity | None: ...
    async def create(self, entity: Entity) -> Entity: ...
    async def update(self, entity: Entity) -> Entity: ...
    async def delete(self, id: UUID) -> None: ...
    async def search(self, query: str) -> list[Entity]: ...
    async def export_csv(self) -> bytes: ...
    async def import_bulk(self, data: list[Entity]) -> int: ...

# GOOD — segregated interfaces
class Readable(Protocol):
    async def get_by_id(self, id: UUID) -> Entity | None: ...
    async def get_all(self) -> list[Entity]: ...

class Writable(Protocol):
    async def create(self, entity: Entity) -> Entity: ...
    async def update(self, entity: Entity) -> Entity: ...
    async def delete(self, id: UUID) -> None: ...

class Searchable(Protocol):
    async def search(self, query: str) -> list[Entity]: ...
```

Pragmatic note: For most CRUD repos, a single IRepository interface with 5-7 methods is fine. Only segregate when different consumers genuinely need different subsets.

## Dependency Inversion Principle (DIP)

High-level modules should not depend on low-level modules. Both should depend on abstractions.

```python
# BAD — service depends on concrete repository
from infrastructure.persistence.user_repository import UserRepository

class UserService:
    def __init__(self, session: AsyncSession):
        self._repo = UserRepository(session)  # Concrete dependency

# GOOD — service depends on protocol/ABC
from domain.ports.user_repository import IUserRepository

class UserService:
    def __init__(self, repository: IUserRepository):
        self._repo = repository  # Abstract dependency
```

### The DI Litmus Test

For any class, check:
1. Does the constructor accept abstractions (Protocol/ABC), not concretions?
2. Can you write a unit test without importing infrastructure?
3. Is the concrete wiring done in the composition root?

If all three are yes, DIP is satisfied.
