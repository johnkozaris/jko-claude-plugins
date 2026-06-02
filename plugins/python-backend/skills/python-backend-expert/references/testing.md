# Testing

## Testing Pyramid for Python Backends

```
/ E2E \ Few: Full HTTP requests against running app
/ Integra \ Some: Service + real DB, API client tests
/ Unit \ Many: Pure functions, services with mock repos
```

## Unit Tests

Test services and domain logic with mocked dependencies:

```python
import pytest
from unittest.mock import AsyncMock

@pytest.fixture
def mock_repo() -> AsyncMock:
    return AsyncMock(spec=IUserRepository)

@pytest.fixture
def user_service(mock_repo: AsyncMock) -> UserService:
    return UserService(repository=mock_repo, encryption=MockEncryption())

async def test_create_user_checks_duplicate(user_service, mock_repo):
    mock_repo.get_by_email.return_value = User(email="exists@test.com")

    with pytest.raises(DuplicateEmailError):
        await user_service.create_user("exists@test.com", "password123")

    mock_repo.create.assert_not_called()

async def test_create_user_success(user_service, mock_repo):
    mock_repo.get_by_email.return_value = None
    mock_repo.create.return_value = User(id=uuid4(), email="new@test.com")

    user = await user_service.create_user("new@test.com", "password123")

    assert user.email == "new@test.com"
    mock_repo.create.assert_called_once()
```

## Integration Tests

Test with real database (SQLite in-memory or testcontainers):

```python
import pytest
from sqlalchemy.ext.asyncio import create_async_engine, async_sessionmaker

@pytest.fixture
async def db_session():
    engine = create_async_engine("sqlite+aiosqlite://", echo=False)
    async with engine.begin() as conn:
        await conn.run_sync(Base.metadata.create_all)

    session_factory = async_sessionmaker(engine, expire_on_commit=False)
    async with session_factory() as session:
        yield session

    await engine.dispose()

async def test_user_repository_create(db_session):
    repo = UserRepository(db_session)
    user = User(email="test@example.com", password_hash="hashed")

    created = await repo.create(user)
    await db_session.commit()

    assert created.id is not None
    fetched = await repo.get_by_id(created.id)
    assert fetched is not None
    assert fetched.email == "test@example.com"
```

## API Tests

Use `httpx.AsyncClient` + `ASGITransport`. The sync `TestClient` masks async bugs (it runs the app in a sync wrapper) and doesn't exercise lifespan state; use the async client from day 0. `async_asgi_testclient` is unmaintained; don't use it.

```python
# conftest.py
import pytest
from httpx import AsyncClient, ASGITransport
from src.main import app

@pytest.fixture
async def client():
    async with AsyncClient(transport=ASGITransport(app=app), base_url="http://test") as ac:
        yield ac

# test_users.py
async def test_create_user_endpoint(client: AsyncClient):
    response = await client.post("/api/users", json={
    "email": "test@example.com",
    "password": "secure123",
    })
    assert response.status_code == 201
    data = response.json()
    assert data["email"] == "test@example.com"
    assert "password" not in data
```

### Override Dependencies: Don't Monkeypatch

Use FastAPI's `app.dependency_overrides` to swap auth or external services. Run integration tests against a **real database** (testcontainers), overriding only what's external; mocking the DB lets mock/prod divergence reach production.

```python
from src.auth.dependencies import current_user

@pytest.fixture(autouse=True)
def _override_auth():
    app.dependency_overrides[current_user] = lambda: User(id=UUID(int=1), email="t@t.io")
    yield
    app.dependency_overrides.clear()
```

## Test Data with Factories

```python
# Use factory_boy or simple factory functions
from dataclasses import dataclass
from uuid import uuid4

def make_user(**overrides) -> User:
    defaults = {
    "id": uuid4(),
    "email": f"user-{uuid4().hex[:8]}@test.com",
    "password_hash": "hashed",
    "is_active": True,
    }
    return User(**(defaults | overrides))

# Usage
user = make_user(email="specific@test.com", is_active=False)
```

## Async Test Configuration

FastAPI/Starlette run on AnyIO under the hood. `pytest` + `anyio[trio]` is the natural fit, and many modern templates use it. `pytest-asyncio` still works and is simpler if you only target asyncio.

```toml
# pyproject.toml (pytest-anyio style, FastAPI-native)
[tool.pytest.ini_options]
asyncio_mode = "auto"           # picks up async def test_* automatically
testpaths = ["tests"]
addopts = ["--strict-config", "--strict-markers", "-ra"]
filterwarnings = [
 "error",                       # promote warnings to failures
 "ignore::DeprecationWarning:httpx",  # known noise from upstream
]

[dependency-groups]
test = ["pytest>=8", "anyio[trio]>=4", "httpx>=0.27", "testcontainers[postgres]>=4"]
```

With `asyncio_mode = "auto"`, any `async def test_*` function is automatically treated as an async test. No `@pytest.mark.asyncio` decorator needed.

**Promote warnings to failures.** `filterwarnings = ["error"]` makes a `DeprecationWarning` (the kind that says "your code will break in Python 3.16") fail the suite, so the build (not production) surfaces it.

## What to Test

| Layer | What to Test | How |
|---|---|---|
| Domain entities | Validation, business rules | Unit (pure functions) |
| Services | Orchestration, error paths | Unit (mocked repos) |
| Repositories | CRUD, queries, edge cases | Integration (real DB) |
| Controllers | Request/response, auth, status codes | API (test client) |
| Middleware | Auth flow, error handling | API (test client) |

## Testing Anti-Patterns

1. **Testing implementation, not behavior** -- Don't assert mock call counts; assert outcomes.
2. **Shared test state** -- Each test should create its own data. Don't rely on test ordering.
3. **Testing the framework** -- Don't test that FastAPI routing or Pydantic validation works. Test YOUR logic.
4. **Slow integration tests for pure logic** -- If a test doesn't need a DB, don't use one.
5. **No negative tests** -- Always test error paths, not just happy paths.