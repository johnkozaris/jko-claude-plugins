# Anti-Patterns Catalog

Use these IDs during critique and hardening.

| ID | Name | Signal | Fix |
|---|---|---|---|
| DN-01 | Fat endpoint | route handler/controller does workflow, data access, and mapping | extract application service / use case |
| DN-02 | Fat hub | hub owns business logic or persistence | keep hub thin and delegate |
| DN-03 | God service | one service owns unrelated workflows and many dependencies | split by reason to change |
| DN-04 | Service locator | `IServiceProvider`, `GetService`, runtime dependency fishing | constructor injection or explicit factory |
| DN-05 | Scoped into singleton | singleton depends on scoped service | redesign lifetime or create scope/factory |
| DN-06 | Mutable singleton state | shared mutable state across requests | make immutable, thread-safe, or scoped |
| DN-07 | DbContext lifetime leak | `DbContext` cached, shared across threads, or used in worker singleton directly | scope it correctly or use `IDbContextFactory<T>` |
| DN-08 | Entity leakage | EF/domain entities returned over HTTP or SignalR | map to contracts |
| DN-09 | Transport validation in domain | DataAnnotations or HTTP concerns on domain model | move validation to boundary contracts |
| DN-10 | Generic repository theater | repository duplicates EF Core CRUD with little value | use `DbContext` directly or narrow the repo |
| DN-11 | SaveChanges everywhere | persistence commits scattered through many layers | define one use-case transaction boundary |
| DN-12 | Blocking async | `.Result`, `.Wait()`, `GetAwaiter().GetResult()` | make path async end-to-end |
| DN-13 | Fire-and-forget work | background tasks without ownership or shutdown semantics | queue and own the work |
| DN-14 | Ad hoc queueing | list + lock + polling loops | use `Channel<T>` or proper queue abstraction |
| DN-15 | In-memory SignalR truth | static dictionaries treated as durable connection state | externalize durable truth and rehydrate |
| DN-16 | AppHost business logic | AppHost contains workflow or data behavior | keep AppHost orchestration-only |
| DN-17 | Configuration leakage | business services read `IConfiguration` directly | bind typed options at composition root |
| DN-18 | Base-class maze | `BaseService`, `BaseRepository`, `BaseController` dominate design | prefer composition and narrow helpers |
| DN-19 | Distributed overkill | brokers/services added without stable boundary or ops story | return to modular monolith first |
| DN-20 | Swallowed exceptions | catch-and-hide or inconsistent error envelope | map deliberately and standardize `ProblemDetails` |
| DN-21 | Kitchen-sink naming | `Coordinator`, `Manager`, `Engine`, or `Orchestrator` type owns many workflows and many dependencies | split by use case and rename by concrete responsibility |

## How to Use the Catalog

When reporting an issue:

- cite the anti-pattern ID
- explain the concrete production risk
- propose the smallest fix that removes the risk
- avoid suggesting architecture rewrites unless the root cause truly demands it

Treat names as clues, not proof. A type called `FooCoordinator`, `FooManager`, or `WorkflowEngine` is not automatically wrong, but it deserves scrutiny for SRP, dependency count, and method sprawl.
