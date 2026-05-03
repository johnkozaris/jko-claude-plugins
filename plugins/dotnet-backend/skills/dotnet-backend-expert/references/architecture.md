# Architecture & Boundaries

## Naming the Patterns

The community uses several overlapping names for what is essentially the same idea: keep the dependency direction inward and isolate the domain from infrastructure and transport. Treat them as variants, not religions.

| Pattern                          | Origin            | What it emphasizes                                                                                                                                         |
| -------------------------------- | ----------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Clean Architecture**           | Robert C. Martin  | Concentric layers; dependency rule points inward toward the domain.                                                                                        |
| **Onion Architecture**           | Jeffrey Palermo   | Same shape as Clean; emphasizes that infrastructure lives on the outside ring.                                                                             |
| **Hexagonal / Ports & Adapters** | Alistair Cockburn | Domain in the center; ports are interfaces defined inward; adapters (HTTP, EF Core, queues) implement them outward.                                        |
| **Vertical Slice Architecture**  | Jimmy Bogard      | Organize by feature, not by technical layer. Each request lives in one cohesive slice; minimize coupling between slices, maximize coupling within a slice. |

The default layered shape below is structurally a hexagonal/ports-and-adapters arrangement — application defines ports, infrastructure implements them. Inside `Application` and `Api`, the recommended folder structure (see "Feature Slices Inside Layers" below) is vertical slice. The two are complementary, not competing.

## When to Use Which

- **Default for new backends**: hexagonal/clean dependency direction at the project level + vertical slice organization inside `Application` and `Api`. This is what the rest of this reference assumes.
- **Vertical slice as the primary model** (no separate Application/Domain projects): excellent for command/query-heavy services where most requests stand alone. Trade off: weaker domain modeling discipline; only safe with a team that recognizes when business logic should be promoted into a real domain layer.
- **Strict layered/clean projects with thick interfaces and many abstractions**: only when the domain is genuinely complex and stable. For most CRUD-shaped backends this is ceremony.

## Default Shape: Modular Monolith First

Start with one deployable backend unless you can name a real reason to split it:

- stable bounded context
- independent deployment cadence
- isolated scaling requirement
- clear ownership boundary
- acceptable eventual consistency story

A large codebase is not itself a distributed-systems requirement.

## Layer Model

```
Kestrel Host / API / Hubs / Workers
    |
    v
Application Services / Use Cases
    |
    v
Domain Entities / Value Objects / Policies
    ^
    |
Infrastructure (EF Core, Dapper, queues, HTTP clients, cache)
```

The host owns transport, composition, auth wiring, and serialization. The application layer owns workflows and transaction boundaries. The domain owns invariants. Infrastructure adapts details.

## Core Rules

1. **Keep the host thin.** `Program.cs`, route groups, controllers, hubs, and workers should wire and delegate.
2. **Keep business rules out of transport types.** Domain and application code should not depend on `HttpContext`, `HubCallerContext`, `ProblemDetails`, or EF Core types.
3. **Keep infrastructure outward-facing.** Infrastructure implements interfaces or boundaries defined inward.
4. **Keep contracts separate from entities.** Wire models are not your domain model.
5. **Keep architecture proportional.** If the business is simple, the structure should be simple too.

## What "Orchestration-Heavy" Means

- request → endpoint → manager → orchestrator → engine → handler → repository
- many classes exist mainly to forward calls
- vague coordination names (`Coordinator`, `Manager`, `Engine`) instead of concrete use cases

That shape is overkill unless the system truly needs workflow engines or durable process coordination.

## Recommended Project Layout

```
src/
  MyApp.Api/               # Kestrel host, endpoints, hubs, DI composition root
  MyApp.Application/       # use cases, workflows, policies, ports
  MyApp.Domain/            # entities, value objects, domain services, invariants
  MyApp.Infrastructure/    # EF Core, clients, queues, cache, adapters
  MyApp.Contracts/         # optional shared request/response/event models

tests/
  MyApp.UnitTests/
  MyApp.IntegrationTests/
  MyApp.ApiTests/
```

## When to Collapse the Shape

For a small or medium backend, two or three projects are often enough:

- `Api` (host + transport)
- `Core` (application + domain)
- `Tests`

Only split `Application`, `Domain`, `Infrastructure`, or `Contracts` when the boundary becomes useful, not because templates do it.

## Feature Slices Inside Layers

Within `Application` and `Api`, prefer grouping by feature over giant technical buckets:

```
Application/
  Orders/
    CreateOrder.cs
    CancelOrder.cs
    OrderPolicy.cs
  Users/
    RegisterUser.cs
    SuspendUser.cs

Api/
  Orders/
    MapOrderEndpoints.cs
    OrderContracts.cs
  Users/
    MapUserEndpoints.cs
```

This keeps change local and reduces “one feature spread across five folders” drift.

## Local Orchestration Boundary

If the solution uses any local-orchestration tool to run multiple services together (Docker Compose, Tye, or any other dev-orchestration layer), keep it **above** service projects. Orchestration composes resources and startup relationships. It does not contain business rules, data-access logic, or endpoint behavior. Service projects must remain independently runnable and testable without it.

## Boundary Smells

| Smell                  | Signal                                                                                           | Fix                                                   |
| ---------------------- | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------- |
| Business logic in host | `Program.cs` or endpoint has branching workflow logic                                            | Move to application service/use case                  |
| EF Core in domain      | domain references `DbContext`, EF attributes, or migrations                                      | Move persistence concerns to infrastructure           |
| Contract leakage       | entity is returned directly over HTTP/SignalR                                                    | Introduce request/response contracts                  |
| Empty layers           | services or repositories only forward calls                                                      | Remove or merge the layer                             |
| Feature scattering     | one feature touches many unrelated folders for one change                                        | Slice by feature within the boundary                  |
| Coordination soup      | `Coordinator`, `Manager`, `Engine`, and `Orchestrator` types forward work across too many layers | Collapse layers and rename by concrete responsibility |

## Architecture Review Questions

- Could the host be replaced without rewriting the business rules?
- If EF Core changed, how much of the core would need edits?
- If one feature changes, how many projects and folders must be touched?
- Is a new layer protecting a real boundary or just adding indirection?
