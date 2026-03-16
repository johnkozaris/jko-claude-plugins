# API Design

Targets Pydantic v2, msgspec, Litestar DTOs, and FastAPI response models.

## Schema Design (DTOs)

### Separate Request and Response Schemas

```python
# BAD -- one schema for everything
class UserSchema(BaseModel):
    id: UUID | None = None
    email: str
    password: str | None = None
    created_at: datetime | None = None

# GOOD -- separate schemas per purpose
class CreateUserRequest(BaseModel):
    email: EmailStr
    password: str = Field(min_length=8)

class UpdateUserRequest(BaseModel):
    email: EmailStr | None = None
    display_name: str | None = None

class UserResponse(BaseModel):
    id: UUID
    email: str
    display_name: str | None
    created_at: datetime

    @staticmethod
    def from_entity(user: User) -> "UserResponse":
        return UserResponse(
            id=user.id, email=user.email,
            display_name=user.display_name, created_at=user.created_at,
        )
```

### Schema Organization

Place schemas near their controllers, not in a monolithic `schemas.py`:

```
entrypoints/http/schemas/
    users.py       # CreateUserRequest, UpdateUserRequest, UserResponse
    services.py    # CreateServiceRequest, ServiceResponse
    auth.py        # LoginRequest, TokenResponse
```

### msgspec Structs (Default for Litestar)

Litestar uses msgspec natively. Prefer `msgspec.Struct` over Pydantic for all request/response schemas in Litestar projects -- 5-12x faster, strict typing, automatic `__slots__`.

```python
import msgspec

class CreateUserRequest(msgspec.Struct):
    email: str
    password: str

class UserResponse(msgspec.Struct, frozen=True):
    id: UUID
    email: str
    created_at: datetime

# Litestar handlers accept and return msgspec Structs natively -- no conversion
@post("/users")
async def create_user(self, data: CreateUserRequest, user_service: UserService) -> UserResponse:
    user = await user_service.create(data.email, data.password)
    return UserResponse(id=user.id, email=user.email, created_at=user.created_at)
```

### Pydantic (Default for FastAPI, Settings)

Use Pydantic when FastAPI requires it, or when you need rich validators (`@field_validator`, `@model_validator`), `BaseSettings`, or `from_attributes` ORM mode.

```python
from pydantic import BaseModel, Field, ConfigDict

class UserResponse(BaseModel):
    model_config = ConfigDict(from_attributes=True)
    id: UUID
    email: str
    created_at: datetime
```

### Litestar DTOs (Auto-Generated from ORM Models)

For CRUD-heavy endpoints, Litestar can auto-generate DTOs directly from SQLAlchemy models:

```python
from advanced_alchemy.extensions.litestar import SQLAlchemyDTO, SQLAlchemyDTOConfig as DTOConfig

class UserReadDTO(SQLAlchemyDTO[UserModel]):
    config = DTOConfig(exclude={"password_hash"})

class UserWriteDTO(SQLAlchemyDTO[UserModel]):
    config = DTOConfig(include={"email", "display_name"})
```

## Error Responses

### Standardized Error Format

```python
class ErrorResponse(BaseModel):
    detail: str
    code: str          # Machine-readable error code
    status_code: int   # HTTP status code

# Example response:
# {"detail": "User not found", "code": "USER_NOT_FOUND", "status_code": 404}
```

### Error Code Enum

```python
from enum import StrEnum

class ApiMessageCode(StrEnum):
    USER_NOT_FOUND = "USER_NOT_FOUND"
    EMAIL_ALREADY_EXISTS = "EMAIL_ALREADY_EXISTS"
    INVALID_CREDENTIALS = "INVALID_CREDENTIALS"
    SERVICE_CONNECTION_FAILED = "SERVICE_CONNECTION_FAILED"
    INTERNAL_ERROR = "INTERNAL_ERROR"
```

### Domain Exception to HTTP Response Mapping

```python
# Register exception handlers in composition root
exception_handlers={
    UserNotFoundError: lambda _, exc: Response(
        content={"detail": str(exc), "code": "USER_NOT_FOUND"},
        status_code=404,
    ),
    DuplicateEmailError: lambda _, exc: Response(
        content={"detail": str(exc), "code": "EMAIL_EXISTS"},
        status_code=409,
    ),
}
```

## Validation Layers

Three distinct validation layers, each with a clear responsibility:

| Layer | Responsibility | Example |
|---|---|---|
| **Schema validation** | Shape, types, format | Email format, string length, required fields |
| **Domain validation** | Business rules | "User must have unique email", "Order total > 0" |
| **Database constraints** | Data integrity | Unique index, foreign keys, NOT NULL |

```python
# Schema validation (Pydantic -- entrypoint layer)
class CreateUserRequest(BaseModel):
    email: EmailStr          # Format validation
    password: str = Field(min_length=8)  # Length validation

# Domain validation (Service -- application layer)
class UserService:
    async def create_user(self, email: str, password: str) -> User:
        if await self._repo.get_by_email(email):
            raise DuplicateEmailError(email)  # Business rule

# Database constraint (ORM model -- infrastructure layer)
email: Mapped[str] = mapped_column(String(255), unique=True)  # Safety net
```

## API Versioning

```python
# URL-based versioning (simplest)
v1_router = Router(path="/api/v1", route_handlers=[UserControllerV1])
v2_router = Router(path="/api/v2", route_handlers=[UserControllerV2])

# Header-based (cleaner URLs, more complex)
# Use middleware to route based on Accept header or custom version header
```

## Pagination

```python
class PaginatedResponse(BaseModel, Generic[T]):
    items: list[T]
    total: int
    page: int
    page_size: int
    has_next: bool

@get("/users")
async def list_users(
    self,
    user_service: UserService,
    page: int = Parameter(ge=1, default=1),
    page_size: int = Parameter(ge=1, le=100, default=20),
) -> PaginatedResponse[UserResponse]:
    users, total = await user_service.list_paginated(page, page_size)
    return PaginatedResponse(
        items=[UserResponse.from_entity(u) for u in users],
        total=total,
        page=page,
        page_size=page_size,
        has_next=(page * page_size) < total,
    )
```
