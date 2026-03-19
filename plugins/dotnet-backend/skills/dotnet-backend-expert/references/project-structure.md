# Project Structure

## Preferred Solution Layout

Use physical structure to make responsibilities easy to find.

```
project-root/
  global.json
  Directory.Build.props
  Directory.Packages.props
  src/
    MyApp.AppHost/         # optional Aspire orchestration only
    MyApp.Api/             # Kestrel host, route groups/controllers, hubs, DI
    MyApp.Application/     # use cases, workflows, ports, policies
    MyApp.Domain/          # entities, value objects, invariants
    MyApp.Infrastructure/  # EF Core, clients, messaging, cache
    MyApp.Contracts/       # optional shared wire contracts
  tests/
    MyApp.UnitTests/
    MyApp.IntegrationTests/
    MyApp.ApiTests/
```

## Small-Service Variant

For a simple backend, this is enough:

```
src/
  MyApp.Api/
  MyApp.Core/
tests/
  MyApp.Tests/
```

Only split further when a boundary is earning its keep.

## Folder Rules Inside Projects

### Api / Host

Group by feature, not by giant buckets:

```
Api/
  Orders/
    MapOrderEndpoints.cs
    OrderContracts.cs
  Users/
    MapUserEndpoints.cs
    UserContracts.cs
  Realtime/
    NotificationsHub.cs
```

### Application

Keep use cases and workflow logic near the feature:

```
Application/
  Orders/
    CreateOrder.cs
    CancelOrder.cs
    OrderPolicy.cs
```

### Infrastructure

Keep adapters and persistence separate from core code:

```
Infrastructure/
  Persistence/
    AppDbContext.cs
    Configurations/
    Repositories/
  Clients/
  Messaging/
  Caching/
```

## Structure Rules

- one major type per file when the type matters
- no `Common`, `Helpers`, or `Utilities` dumping ground unless it is tiny and specific
- do not let `Program.cs` become the only place a feature exists
- do not let AppHost become a second application layer
- keep contracts close to the boundary they serve

## File Size Guidance

| File Type | Target | Max |
|---|---|---|
| `Program.cs` | 50-150 | 250 |
| endpoint group | 50-150 | 300 |
| service/use case | 40-200 | 300 |
| hub | 50-150 | 250 |
| contract file | 20-120 | 200 |
| any `.cs` file | --- | 500 |

## Smells

- a single project contains host, domain, persistence, and tests
- every feature spreads across `Controllers`, `Services`, `Repositories`, `Dtos`, `Validators` folders with no cohesion
- `Base*` folders exist everywhere
- `Program.cs` knows business workflows
- `Contracts` is shared when nothing is actually shared across boundaries

## Structure Review Questions

- Can a new developer find one feature quickly?
- Can a feature change stay mostly within one project and folder slice?
- Does the host reference only what it needs to host and compose?
- Are boundaries visible from the solution graph?
