# Testing

## Keep the Pyramid Broad at the Bottom

Use:

- many fast unit tests for core logic
- targeted integration tests for boundaries
- a smaller set of API tests for HTTP/auth/middleware behavior
- focused SignalR integration tests where transport behavior matters

Do not let slow end-to-end tests become the primary safety net.

## Unit Tests

Good for:

- domain invariants
- application services/use cases
- policies and pure transformations

Avoid network, file system, or real database access in unit tests.

## Integration Tests

Good for:

- HTTP pipeline
- auth and middleware
- DI wiring
- data access against a real or close relational provider
- hosted service orchestration pieces
- SignalR connection/group/auth behavior

Prefer real providers or close substitutes. EF Core InMemory is not proof of relational correctness.

## Data Access Tests

- test query behavior against the real provider when possible
- keep migrations part of the story
- use testcontainers or another reliable local strategy when warranted
- verify projections, transactions, and concurrency behavior where it matters

## SignalR Tests

Mocking a hub does not prove connection behavior.

Use a real host plus real client connections when testing:

- auth
- groups
- reconnect logic
- transport-level behavior

## Test Smells

- unit tests that mostly verify mocks were called
- integration tests that duplicate every unit-test case
- one fake data provider standing in for real relational behavior forever
- flaky slow tests covering too much surface at once
- “happy path only” tests for error or contract-heavy endpoints
