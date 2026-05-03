# Testing

## Keep the Pyramid Broad at the Bottom

Use:

- many fast unit tests for core logic
- targeted integration tests for boundaries
- a smaller set of API tests for HTTP/auth/middleware behavior
- focused real-time tests (SignalR / raw WebSockets / SSE) where transport behavior matters

Do not let slow end-to-end tests become the primary safety net.

## Tooling Defaults (.NET 10)

- **xUnit or NUnit** — either is fine; pick one and stay consistent.
- **`Microsoft.Testing.Platform` (MTP)** — supported in `dotnet test` for .NET 10. Faster than the legacy `VSTest` runner; worth adopting on new projects.
- **`WebApplicationFactory<TEntryPoint>`** — standard way to spin up the real app in-process for HTTP/auth/middleware tests. Pairs with the .NET 10 source generator that emits `public partial class Program` automatically (no more manual `public partial class Program {}` line in `Program.cs`).
- **TestContainers (`Testcontainers.PostgreSql`, `Testcontainers.MsSql`, etc.)** — default for relational integration tests. Spins real databases in Docker; cheap and reliable. Beats EF Core InMemory and SQLite-as-fake-Postgres approaches that hide bugs.
- **`FluentAssertions` or `Shouldly`** — optional but common for readable assertions.

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

Prefer real providers or close substitutes via `WebApplicationFactory` and TestContainers. EF Core InMemory is not proof of relational correctness.

## Data Access Tests

- test query behavior against the real provider when possible
- keep migrations part of the story
- use TestContainers for a real Postgres/MSSQL instance per test class or fixture
- verify projections, transactions, and concurrency behavior where it matters

## Real-Time Tests

Mocking a hub or socket does not prove connection behavior.

Use a real host plus real client connections when testing:

- auth
- groups (SignalR)
- reconnect logic
- transport-level behavior
- SSE: stream lifecycle, cancellation propagation, `Last-Event-ID` resume

## Test Smells

- unit tests that mostly verify mocks were called
- integration tests that duplicate every unit-test case
- one fake data provider standing in for real relational behavior forever
- flaky slow tests covering too much surface at once
- “happy path only” tests for error or contract-heavy endpoints
