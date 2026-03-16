# Error Handling

## Domain Exception Hierarchy

Every backend needs a structured exception hierarchy rooted in the domain.

```python
# domain/exceptions.py
class DomainError(Exception):
    """Base for all domain errors. Never catch this in handlers -- let
    exception handlers in the composition root translate to HTTP responses."""

class NotFoundError(DomainError):
    def __init__(self, entity: str, identifier: str) -> None:
        super().__init__(f"{entity} not found: {identifier}")
        self.entity = entity
        self.identifier = identifier

class ConflictError(DomainError):
    def __init__(self, detail: str) -> None:
        super().__init__(detail)

class ValidationError(DomainError):
    def __init__(self, field: str, reason: str) -> None:
        super().__init__(f"Validation failed for {field}: {reason}")
        self.field = field
        self.reason = reason

class AuthorizationError(DomainError):
    pass

class ServiceConnectionError(DomainError):
    def __init__(self, service_name: str, reason: str) -> None:
        super().__init__(f"Connection to '{service_name}' failed: {reason}")
        self.service_name = service_name
        self.reason = reason
```

### Mapping to HTTP

Register exception handlers at the app level:

```python
# Litestar
exception_handlers={
    NotFoundError: lambda _, exc: Response(
        content={"detail": str(exc), "code": f"{exc.entity.upper()}_NOT_FOUND"},
        status_code=404,
    ),
    ConflictError: lambda _, exc: Response(
        content={"detail": str(exc), "code": "CONFLICT"},
        status_code=409,
    ),
    AuthorizationError: lambda _, exc: Response(
        content={"detail": "Forbidden", "code": "FORBIDDEN"},
        status_code=403,
    ),
}

# FastAPI
@app.exception_handler(NotFoundError)
async def not_found_handler(request: Request, exc: NotFoundError):
    return JSONResponse(status_code=404, content={"detail": str(exc)})
```

## Error Handling Rules

### 1. Never Swallow Exceptions

```python
# BAD -- exception swallowed, bug hidden
try:
    await send_notification(user_id)
except Exception:
    pass  # Silent failure

# BAD -- logged but not handled
try:
    result = await service.process()
except Exception as e:
    logger.error(f"Error: {e}")
    # What happens to the caller? They get None or garbage

# GOOD -- handle, log, and communicate
try:
    await send_notification(user_id)
except NotificationError:
    logger.warning("notification_failed", user_id=str(user_id))
    # Non-critical: continue without notification
except Exception:
    logger.exception("unexpected_notification_error", user_id=str(user_id))
    # Re-raise if critical, or degrade gracefully
```

### 2. Catch Specific Exceptions

```python
# BAD -- catches everything including KeyboardInterrupt, SystemExit
try:
    result = await repo.get_by_id(id)
except Exception:
    raise ClientException("Not found")

# GOOD -- catch what you expect
try:
    result = await repo.get_by_id(id)
except SQLAlchemyError as e:
    logger.error("db_error", error=str(e))
    raise ServiceUnavailableError() from e
```

### 3. Use Exception Chaining

```python
# BAD -- original traceback lost
try:
    user = await repo.get_by_id(id)
except RepositoryError:
    raise UserNotFoundError(id)  # Where did the original error go?

# GOOD -- chain preserves traceback
try:
    user = await repo.get_by_id(id)
except RepositoryError as e:
    raise UserNotFoundError(id) from e  # Original error in __cause__
```

### 4. Domain Exceptions, Not HTTP Exceptions

```python
# BAD -- HTTP concerns in service layer
class UserService:
    async def get_user(self, id: UUID) -> User:
        user = await self._repo.get_by_id(id)
        if user is None:
            raise HTTPException(404, "User not found")  # Framework leak!

# GOOD -- domain exception, mapped to HTTP at boundary
class UserService:
    async def get_user(self, id: UUID) -> User:
        user = await self._repo.get_by_id(id)
        if user is None:
            raise UserNotFoundError(str(id))  # Pure domain
```

### 5. Structured Logging on Errors

```python
# BAD -- unstructured, hard to query
logger.error(f"Failed to create user {email}: {error}")

# GOOD -- structured, queryable
logger.error(
    "user_creation_failed",
    email=email,
    error_type=type(error).__name__,
    error_detail=str(error),
)
```

## Error Response Best Practices

### Include Machine-Readable Codes

```python
# Clients can switch on code, not parse human-readable messages
{
    "detail": "A user with this email already exists",
    "code": "EMAIL_ALREADY_EXISTS",
    "status_code": 409
}
```

### Never Expose Internal Details

```python
# BAD -- leaks internal state
return {"detail": f"SQLAlchemyError: UNIQUE constraint failed: users.email"}

# GOOD -- user-friendly, internal logged separately
logger.error("duplicate_email", email=email, error=str(db_error))
return {"detail": "A user with this email already exists", "code": "EMAIL_EXISTS"}
```
