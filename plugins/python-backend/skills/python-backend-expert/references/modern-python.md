# Modern Python

Python 3.14 brings deferred annotations, free-threaded builds, and more. Target the latest stable Python for new projects. Check `requires-python` in `pyproject.toml` to determine the project's minimum version and only suggest features available at that version. Use **uv** for all package management.

## Type Hints -- Use Them Everywhere

### Basic Patterns

```python
# Function signatures -- always typed
async def get_user(user_id: UUID) -> User | None: ...

# Class attributes
class Config:
    debug: bool
    db_url: str
    max_connections: int = 10

# Collections -- use built-in generics (3.9+)
names: list[str]           # Not List[str]
mapping: dict[str, int]    # Not Dict[str, int]
ids: set[UUID]             # Not Set[UUID]
pair: tuple[str, int]      # Not Tuple[str, int]
```

### Union Types (3.10+)

```python
# Modern syntax
def process(value: str | int | None) -> str: ...

# Not the old way
def process(value: Union[str, int, Optional[str]]) -> str: ...
```

### New Generic Syntax (3.12+)

```python
# Modern -- clean, no TypeVar import
class Repository[T]:
    async def get_by_id(self, id: UUID) -> T | None: ...
    async def create(self, entity: T) -> T: ...

# Type alias (3.12+)
type UserID = UUID
type Result[T] = T | None

# Old way (still works, needed for 3.11-)
from typing import TypeVar, Generic
T = TypeVar("T")
class Repository(Generic[T]): ...
```

### TypeVar Defaults (3.13+)

TypeVar, ParamSpec, and TypeVarTuple now support default values. Simplifies generic APIs where most callers use the same type.

```python
# 3.13+ -- default type parameter
class Repository[T = dict[str, Any]]:
    async def list_all(self) -> list[T]: ...

# Callers can omit T when the default is fine
repo: Repository = ...          # T defaults to dict[str, Any]
repo: Repository[User] = ...   # T is User
```

### Deferred Annotations (3.14)

Python 3.14 evaluates annotations lazily by default (PEP 649). No more `from __future__ import annotations` needed. Forward references work naturally.

```python
# 3.14 -- this just works, no import needed
class Parent:
    children: list[Child]   # Forward reference -- no quotes, no __future__

class Child:
    parent: Parent

# Access annotations explicitly when needed
import annotationlib
annotations = annotationlib.get_annotations(Parent, format=annotationlib.Format.VALUE)
```

**Impact on frameworks:** Litestar, FastAPI, Pydantic, and SQLAlchemy all benefit. Type-based frameworks that inspect annotations at import time get reduced startup cost. `from __future__ import annotations` is deprecated on 3.14+ -- remove it.

### ReadOnly TypedDict (3.13+)

```python
from typing import ReadOnly, TypedDict

class UserConfig(TypedDict):
    name: str
    api_key: ReadOnly[str]  # Type checkers flag attempts to modify this
```

### TypeIs for Type Narrowing (3.13+)

```python
from typing import TypeIs

def is_admin(user: User | AnonymousUser) -> TypeIs[User]:
    return isinstance(user, User) and user.is_admin

# After is_admin() returns True, type checkers narrow to User
if is_admin(current_user):
    current_user.admin_panel()  # Type-safe, no cast needed
```

## Protocol -- Structural Subtyping

Prefer Protocol over ABC when you want duck typing with type safety:

```python
from typing import Protocol, runtime_checkable

@runtime_checkable
class Encryptable(Protocol):
    def encrypt(self, data: str) -> str: ...
    def decrypt(self, data: str) -> str: ...

# Any class with encrypt/decrypt methods satisfies this
# No inheritance required!
class FernetEncryption:
    def encrypt(self, data: str) -> str: ...
    def decrypt(self, data: str) -> str: ...

# Works without inheriting from Encryptable
def process(enc: Encryptable) -> None:
    enc.encrypt("data")  # Type-checked
```

### Protocol vs ABC

| Feature | Protocol | ABC |
|---|---|---|
| Requires inheritance | No (structural) | Yes (nominal) |
| Runtime checkable | Optional (`@runtime_checkable`) | Always |
| Abstract methods | Implicit (all methods) | Explicit (`@abstractmethod`) |
| Best for | Ports, interfaces | Shared base behavior |

**Rule of thumb:** Use Protocol for ports in hexagonal architecture. Use ABC when you need shared method implementations.

## Dataclasses vs Pydantic vs msgspec

### When to Use Each

| Tool | Use For | Strengths |
|---|---|---|
| `msgspec.Struct` | **Default for Litestar** request/response schemas | 5-12x faster than Pydantic, native Litestar support, strict typing, `__slots__` auto |
| `dataclass` | Domain entities, value objects, internal DTOs | Zero dependencies, stdlib, fast |
| Pydantic | FastAPI schemas, settings (`BaseSettings`), complex validators | Rich validation, ecosystem, ORM mode |
| `NamedTuple` | Immutable value types | Tuple performance, pattern matching |

**Preference order for Litestar:** `msgspec.Struct` > `dataclass` > Pydantic. Use Pydantic only for `BaseSettings` or when you need `@field_validator` / `@model_validator` logic that msgspec can't express.

**Preference order for FastAPI:** Pydantic (required by framework) > `dataclass` for domain > `msgspec` for internal serialization.

### Dataclass Best Practices

```python
from dataclasses import dataclass, field
from uuid import UUID
from datetime import datetime, timezone

@dataclass(slots=True, frozen=True)  # slots=True for memory, frozen for immutability
class User:
    id: UUID
    email: str
    created_at: datetime
    roles: list[str] = field(default_factory=list)

@dataclass(slots=True)  # Mutable when state changes needed
class ServiceConnection:
    id: UUID | None
    name: str
    base_url: str
    is_enabled: bool = True
    health_status: HealthStatus = HealthStatus.UNKNOWN
```

### Pydantic 2.12+ for API Boundaries

```python
from pydantic import BaseModel, Field, EmailStr, ConfigDict

class CreateUserRequest(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8, max_length=128)

class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: UUID
    email: str
    created_at: datetime
```

### msgspec 0.20+ for High-Throughput

```python
import msgspec

class UserResponse(msgspec.Struct, frozen=True):
    id: UUID
    email: str
    created_at: datetime

# Litestar natively supports msgspec Structs -- no conversion needed
# 5-12x faster than Pydantic for serialization/deserialization
```

## Pattern Matching (3.10+)

```python
# Clean error handling dispatch
match exc:
    case ServiceConnectionError():
        detail = f"Connection to '{exc.service_name}' failed"
        code = ApiMessageCode.SERVICE_CONNECTION_FAILED
    case UnsupportedServiceError():
        detail = f"Unsupported service: {exc.service_type}"
        code = ApiMessageCode.SERVICE_TYPE_UNSUPPORTED
    case _:
        detail = "Request failed"
        code = ApiMessageCode.HTTP_BAD_REQUEST

# Type narrowing in match
match event:
    case UserCreated(user_id=uid):
        await notify_admins(uid)
    case UserDeleted(user_id=uid, reason=reason):
        await archive_user_data(uid, reason)
```

## StrEnum (3.11+)

```python
from enum import StrEnum

class ServiceType(StrEnum):
    GITHUB = "github"
    GITLAB = "gitlab"
    PORTAINER = "portainer"

# Auto-serializes to string in JSON, no .value needed
```

## Modern Collections

```python
# TypedDict for structured dicts (avoid dict[str, Any])
from typing import TypedDict

class HealthCheckResult(TypedDict):
    healthy: bool
    latency_ms: float
    message: str

# collections.abc for type hints on iterables
from collections.abc import Sequence, Mapping, AsyncGenerator

async def stream_results() -> AsyncGenerator[User, None]:
    async for row in result_stream:
        yield row
```

## Pydantic Settings (Configuration)

```python
from pydantic_settings import BaseSettings, SettingsConfigDict
from pydantic import Field, SecretStr

class Settings(BaseSettings):
    model_config = SettingsConfigDict(
        env_file=".env",
        env_prefix="APP_",
        case_sensitive=False,
    )

    database_url: str
    secret_key: SecretStr          # Never printed in logs
    debug: bool = False
    encryption_key: str = Field(min_length=32)
    max_connections: int = 10

settings = Settings()  # Auto-loads from env vars
```

## Datetime -- Use Timezone-Aware Always

```python
from datetime import datetime, timezone

# CORRECT (3.12+)
now = datetime.now(timezone.utc)

# DEPRECATED since 3.12 (no confirmed removal date -- avoid in all new code)
now = datetime.utcnow()  # Returns naive datetime, causes bugs
```

## Deprecated Patterns to Replace

| Old Pattern | Modern Replacement | Since |
|---|---|---|
| `datetime.utcnow()` | `datetime.now(timezone.utc)` | deprecated 3.12 |
| `from __future__ import annotations` | Native deferred annotations | 3.14 |
| `typing.Optional[X]` | `X \| None` | 3.10 |
| `typing.Union[X, Y]` | `X \| Y` | 3.10 |
| `typing.List`, `Dict`, `Tuple` | `list`, `dict`, `tuple` | 3.9 |
| `TypeVar("T"); Generic[T]` | `class Foo[T]:` | 3.12 |
| `collections.OrderedDict` | `dict` (ordered since 3.7) | 3.7 |
| `pkg_resources` | `importlib.resources` / `importlib.metadata` | 3.9 |
| `asyncio.get_event_loop()` | `asyncio.get_running_loop()` | 3.10 |
| `@asyncio.coroutine` | `async def` | 3.5 (removed 3.11) |

## Free-Threaded Python (3.14)

Python 3.14 officially supports the free-threaded build (no GIL). Performance penalty is ~5-10% on single-threaded code. The `concurrent.interpreters` module enables true multi-core parallelism via subinterpreters.

**For backend developers:** This matters most for CPU-bound work that currently uses `multiprocessing` or `asyncio.to_thread()`. I/O-bound backends (most web APIs) see minimal benefit yet, but the ecosystem is preparing. SQLAlchemy 2.1 beta ships free-threaded wheels.
