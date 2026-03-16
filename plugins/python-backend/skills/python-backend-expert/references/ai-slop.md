# The AI Slop Test

AI coding tools produce code that compiles, passes tests, and reads "right" -- but introduces hidden technical debt, security vulnerabilities, and architectural rot. This reference catalogs the fingerprints of LLM-generated Python backend code so you can detect and eliminate them.

## The Core Problem

- CodeRabbit found AI-generated code creates **1.7x more issues** than human code
- 40% of Copilot suggestions contain **security vulnerabilities** (Pearce et al.)
- **20% of AI package imports** reference non-existent libraries
- Code refactoring dropped from 25% to under 10% of changes since AI adoption (GitClear)
- AI optimizes for **consistency with training data**, not optimal design

## AI Slop Tells -- Python Backend Edition

### AS-01: The Monolithic God Handler

AI loves dumping everything into one function or one file. A single `main.py` with routes, models, business logic, database queries, and error handling all mixed together.

```python
# AI SLOP -- everything in one handler
@app.post("/users")
async def create_user(request: Request):
    data = await request.json()
    if not data.get("email"):
        return JSONResponse({"error": "Email required"}, 400)
    existing = await db.execute(select(User).where(User.email == data["email"]))
    if existing.scalar():
        return JSONResponse({"error": "Email exists"}, 409)
    hashed = bcrypt.hashpw(data["password"].encode(), bcrypt.gensalt())
    user = User(email=data["email"], password=hashed)
    db.add(user)
    await db.commit()
    return JSONResponse({"id": str(user.id), "email": user.email})

# DESIGNED -- thin handler, service owns logic, typed schemas
@post("/users")
async def create_user(self, data: CreateUserRequest, user_service: UserService) -> UserResponse:
    user = await user_service.create(data.email, data.password)
    return UserResponse.from_entity(user)
```

### AS-02: dict[str, Any] Everywhere

AI's favorite data structure. No type safety, no IDE support, no documentation. The lazy alternative to defining proper schemas.

```python
# AI SLOP
async def get_user(user_id: str) -> dict[str, Any]:
    user = await db.get(user_id)
    return {"id": str(user.id), "name": user.name, "email": user.email}

# DESIGNED -- typed response model
async def get_user(user_id: UUID) -> UserResponse:
    user = await user_service.get(user_id)
    return UserResponse.from_entity(user)
```

### AS-03: Over-Commenting the Obvious

AI generates a comment for every line. Comments should explain WHY, not WHAT. The code itself should explain WHAT.

```python
# AI SLOP
# Get the user from the database
user = await repo.get_by_id(user_id)
# Check if user exists
if user is None:
    # Raise not found error
    raise UserNotFoundError(user_id)
# Return the user
return user

# DESIGNED -- the code speaks for itself
user = await repo.get_by_id(user_id)
if user is None:
    raise UserNotFoundError(user_id)
return user
```

### AS-04: Hallucinated Imports and Deprecated APIs

AI confidently imports packages that don't exist, or uses APIs deprecated years ago.

```python
# AI SLOP -- hallucinated/deprecated
from datetime import datetime
created_at = datetime.utcnow()  # Deprecated in Python 3.12

from collections import OrderedDict  # Unnecessary since Python 3.7 (dicts are ordered)

import pkg_resources  # Deprecated, use importlib.resources

from fastapi_utils.tasks import repeat_every  # May not exist in current version

# DESIGNED -- verified, current APIs
from datetime import datetime, timezone
created_at = datetime.now(timezone.utc)

from importlib.resources import files
```

### AS-05: Async/Sync Confusion

AI mixes blocking calls into async handlers, or wraps sync code with `async def` without actually awaiting anything.

```python
# AI SLOP -- sync requests in async handler
import requests

async def fetch_external_data(url: str) -> dict:
    response = requests.get(url)  # BLOCKS THE EVENT LOOP
    return response.json()

# AI SLOP -- async def with no await (fake async)
async def hash_password(password: str) -> str:
    return bcrypt.hashpw(password.encode(), bcrypt.gensalt())  # Still blocking!

# DESIGNED
async def fetch_external_data(url: str) -> ExternalData:
    async with httpx.AsyncClient() as client:
        response = await client.get(url)
        return ExternalData.model_validate(response.json())

async def hash_password(password: str) -> str:
    return await asyncio.to_thread(bcrypt.hashpw, password.encode(), bcrypt.gensalt())
```

### AS-06: Hardcoded Credentials and Config

AI copies training data patterns that embed secrets directly in code.

```python
# AI SLOP
DATABASE_URL = "postgresql://admin:password123@localhost:5432/mydb"
SECRET_KEY = "super-secret-key-change-in-production"
API_KEY = "sk-1234567890abcdef"

# DESIGNED -- Pydantic Settings from environment
class Settings(BaseSettings):
    database_url: str
    secret_key: SecretStr
    api_key: SecretStr
    model_config = SettingsConfigDict(env_file=".env")
```

### AS-07: Cross-Language Pattern Leakage

AI trained on JavaScript, Java, and Ruby bleeds those patterns into Python.

```python
# AI SLOP -- JavaScript/Java patterns in Python
items.push(new_item)          # list.append() in Python
if items.length > 0:          # len(items) in Python
user.equals(other)            # == operator in Python
for item in items.forEach():  # just: for item in items:
final_result = None           # "final" is not a Python concept
items.map(lambda x: x.name)  # list comprehension: [x.name for x in items]

# DESIGNED -- idiomatic Python
items.append(new_item)
if items:  # truthy check, no len() needed
user == other
for item in items:
names = [x.name for x in items]
```

### AS-08: Placeholder and TODO Accumulation

AI generates stubs and placeholders that never get filled in. Functions with `pass`, methods with `# TODO: implement`, and dead conditional branches.

```python
# AI SLOP
class PaymentProcessor:
    def process_payment(self, amount: float) -> bool:
        # TODO: implement payment processing
        pass

    def refund(self, transaction_id: str) -> bool:
        # TODO: implement refund logic
        return True  # Placeholder

    def validate_card(self, card_number: str) -> bool:
        # Should work hopefully
        return len(card_number) == 16
```

**If it has a `pass` body or a hedging comment like "should work hopefully" -- it's AI slop.**

### AS-09: Copy-Paste Frankenstein

AI stitches together patterns from different codebases, frameworks, and Python versions. The result has inconsistent style, mixed naming conventions, and conflicting architectural assumptions.

```python
# AI SLOP -- mixed patterns in one file
from flask import Flask  # Flask patterns
from fastapi import FastAPI  # FastAPI patterns
from django.db import models  # Django ORM

# CamelCase Java-style here
class UserManager:
    # snake_case Pythonic here
    def get_user_by_id(self, user_id):
        # Callback-style Node.js pattern here
        def on_success(result):
            return result
```

### AS-10: The "Helpful" Docstring Wall

AI generates elaborate docstrings for trivial functions, sometimes longer than the function body. Sphinx-formatted, with type information that's already in the signature.

```python
# AI SLOP
async def get_user(user_id: UUID) -> User | None:
    """Get a user by their unique identifier.

    This function retrieves a user from the database using their
    unique identifier (UUID). It returns the user object if found,
    or None if no user with the given ID exists.

    Args:
        user_id (UUID): The unique identifier of the user to retrieve.

    Returns:
        User | None: The user object if found, otherwise None.

    Raises:
        DatabaseError: If there is an error connecting to the database.
    """
    return await self._repo.get_by_id(user_id)

# DESIGNED -- the signature IS the documentation
async def get_user(user_id: UUID) -> User | None:
    return await self._repo.get_by_id(user_id)
```

### AS-11: Exception Handling Theater

AI wraps everything in try/except to look "safe" -- but catches too broadly, swallows errors, or converts specific exceptions to generic ones.

```python
# AI SLOP -- exception theater
try:
    user = await repo.get_by_id(user_id)
    if user is None:
        raise ValueError("User not found")
    return user
except ValueError as e:
    raise HTTPException(status_code=404, detail=str(e))
except Exception as e:
    logger.error(f"Error getting user: {e}")
    raise HTTPException(status_code=500, detail="Internal server error")

# DESIGNED -- let domain exceptions propagate, handle at boundary
user = await repo.get_by_id(user_id)
if user is None:
    raise UserNotFoundError(str(user_id))
return user
# Exception handler at app level maps UserNotFoundError -> 404
```

### AS-12: Gratuitous Type: ignore and noqa

AI adds suppression comments to silence linters rather than fixing the actual problem.

```python
# AI SLOP
result = some_function()  # type: ignore
data = process(input)  # noqa: E501
config = load_config()  # type: ignore[assignment]

# DESIGNED -- fix the actual type error
result: MyType = some_function()
data = process(validated_input)
config: AppConfig = load_config()
```

## Detection Checklist

When reviewing code, check for these AI slop signals:

- [ ] Are there comments on >30% of lines? (over-commenting)
- [ ] Are `dict[str, Any]` or untyped dicts used for data passing?
- [ ] Is the entire backend in one or two files?
- [ ] Are there `# TODO`, `pass` bodies, or hedging comments?
- [ ] Do imports include packages you can't find on PyPI?
- [ ] Are there deprecated API calls (`datetime.utcnow()`, `pkg_resources`)?
- [ ] Is `requests` used in async handlers?
- [ ] Are there hardcoded URLs, keys, or credentials?
- [ ] Are there JavaScript/Java patterns (`.push()`, `.length`, `.equals()`)?
- [ ] Do docstrings repeat the type signature?
- [ ] Are there broad `except Exception` blocks that swallow errors?
- [ ] Are there `# type: ignore` or `# noqa` suppressions?
- [ ] Does the code feel stitched together from different style guides?
- [ ] Is the architecture flat with no layer separation?

**If 3+ of these are true, the code needs architectural review, not just linting.**

## What AI-Free Backend Code Looks Like

- **Deliberate architecture** -- layers, boundaries, one responsibility per module
- **The type system communicates intent** -- no `Any`, no untyped dicts, no string-typed enums
- **Sparse comments** -- code explains WHAT, rare comments explain WHY
- **Domain-specific abstractions** -- `OrderService`, `PaymentGateway`, not `process_data()`
- **Consistent style** -- one naming convention, one error handling pattern, one DI approach
- **No dead code** -- no TODOs, no pass stubs, no commented-out blocks
- **Verified dependencies** -- every import exists, every API call uses the current version
