# REST Endpoints & Contracts

## Choose an Endpoint Style Intentionally

### Minimal APIs

Prefer for:

- new small-to-medium backend services
- thin HTTP boundaries
- explicit route-group organization
- simple request/response mapping

### Controllers

Prefer when you need:

- mature MVC features and filters
- more built-in organization for large teams
- controller-specific behavior such as advanced model binding or conventions

Either style is acceptable if the endpoint layer stays thin.

## Route Design

- model resources, not RPC verbs
- group routes by feature
- keep route names predictable
- define versioning deliberately for public APIs

Good:

- `/orders`
- `/orders/{orderId}`
- `/users/{userId}/sessions`

Bad:

- `/createOrder`
- `/doThing`
- `/GetAllUsers`

## Contracts

Use separate request and response contracts per use case.

- do not expose EF Core entities directly
- do not reuse one giant DTO for create/update/read
- keep boundary validation on request models
- keep domain invariants in domain code

## Validation

Validate at the boundary:

- DataAnnotations or explicit validation services for transport-level shape
- consistent validation errors via `ValidationProblemDetails`
- domain rejects business-invalid states even if the request passed transport validation

## ProblemDetails

Use a consistent RFC 7807 shape.

- one global error policy
- stable machine-readable codes when helpful
- sanitize details for production
- avoid endpoint-specific JSON error inventions

## Pagination, Idempotency, Versioning

### Pagination

- paginate large collections
- project only needed columns
- keep default page sizes bounded
- prefer stable ordering

### Idempotency

- GET, PUT, DELETE should be safely repeatable
- use idempotency keys or repeatability strategies for retryable POST flows that can double-charge or double-create

### Versioning

- decide early for external/public APIs
- avoid silent breaking changes
- keep contract evolution visible in review

## Endpoint Template

A healthy endpoint usually does this:

1. bind request
2. validate boundary
3. call application service / use case
4. map result to HTTP response
5. return typed contract

If the endpoint also decides transactions, coordinates five dependencies, emits events, and shapes DB queries, it is too fat.

## Endpoint Smells

- endpoint directly uses `DbContext`
- endpoint directly publishes to a broker and updates the database
- endpoint returns domain entity graph
- endpoint performs business rule branching
- `Program.cs` contains dozens of inline lambdas with no feature structure
