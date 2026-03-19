# SOLID & OOP for .NET Backends

Use SOLID as a **maintainability heuristic**, not a religion. The goal is clear responsibilities, strong boundaries, and simple extension points.

## Single Responsibility Principle

A type should have one reason to change.

In backend code that usually means:

- endpoints/hubs handle transport concerns only
- services handle workflow orchestration
- entities/value objects protect invariants
- repositories/adapters handle persistence or external systems
- factories exist only when object creation needs runtime inputs or policy

### SRP Signals

- constructor has 6+ dependencies
- class mixes validation, persistence, mapping, notifications, and business rules
- one file contains multiple unrelated endpoint groups
- a background worker both schedules work and contains domain logic

## Open/Closed Principle

Add new behavior by adding new code at the boundary, not by editing stable core logic every time.

Good extension points:

- strategy or policy interfaces at real seams
- decorators or pipeline behaviors for cross-cutting concerns
- new endpoint groups or handlers by feature
- new adapter implementations in infrastructure

Bad extension points:

- abstract base classes that all subclasses fight
- giant switch statements that grow forever
- one interface per class with no real substitution need

## Dependency Inversion Principle

High-level code depends on abstractions at the boundary, not infrastructure details.

Good:

- application depends on `IEmailSender`, not SMTP implementation
- domain logic receives values or interfaces, not `HttpContext` or `DbContext`
- infrastructure implements ports from inward layers

Bad:

- endpoint constructs concrete collaborators directly
- service takes `IServiceProvider` and resolves what it wants
- domain imports EF Core, cache, or HTTP client types

## Composition Over Inheritance

Prefer composition and DI to inheritance for backend services.

Use inheritance sparingly:

- framework base types such as `ControllerBase` or `Hub`
- true `is-a` hierarchies that are stable and small

Avoid inheritance for code reuse when helper services or policies would be clearer.

## Interface Segregation, Practically

Keep interfaces narrow and boundary-driven. A repository or client interface should reflect how callers actually use it, not every method that might exist someday.

## File and Type Size Guidance

| Component | Target | Max | Split Signal |
|---|---|---|---|
| Endpoint file | 50-150 | 300 | multiple unrelated route groups |
| Hub | 50-150 | 250 | contains domain or persistence logic |
| Service / use case | 40-200 | 300 | handles multiple domains or workflows |
| Repository / adapter | 30-150 | 250 | many ad hoc query knobs |
| Contract file | 20-120 | 200 | unrelated request/response models |
| Any file | --- | 500 | split urgently |

## Review Heuristics

- If a class takes many dependencies, question SRP before adding more abstractions.
- If new behavior requires modifying old stable classes instead of adding a new policy or adapter, question OCP.
- If an abstraction exists only because “every class needs an interface,” remove it.
- If a base class exists mostly for shared helpers, prefer composition.
