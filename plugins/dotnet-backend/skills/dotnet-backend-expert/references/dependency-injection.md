# Dependency Injection

## Default Rule

Use the built-in `.NET` DI container unless a proven requirement demands more.

Constructor injection first. Explicit dependencies first. Centralized registration first.

## Lifetime Rules

| Lifetime | Use For | Avoid When |
|---|---|---|
| Singleton | stateless services, caches with thread-safe design, expensive app-wide collaborators | dependency is scoped, mutable, request-specific, or not thread-safe |
| Scoped | `DbContext`, request-bound services, unit-of-work collaborators | object must outlive a request or run across threads |
| Transient | cheap helpers, mappers, lightweight policies, stateful-per-call objects | disposable/transient object is created excessively or should really be scoped |

## Composition Root

Keep registrations obvious:

- `Program.cs` for small services
- extension methods like `AddApplication()` or `AddInfrastructure()` for larger ones
- service registration should remain readable end-to-end

Do not hide lifetimes behind vague helper names.

## Background Work and Scopes

Hosted services are typically singletons. Scoped dependencies must be created inside a scope:

- use `IServiceScopeFactory`
- or use `IDbContextFactory<TContext>` for EF Core

Never inject a scoped service directly into a hosted service or singleton and hope it behaves.

## Options and Configuration

Inject typed options, not raw `IConfiguration`, into business services.

- `IOptions<T>` for stable settings
- `IOptionsSnapshot<T>` for per-request reload semantics
- `IOptionsMonitor<T>` only when change notifications matter

Validate critical options at startup.

## Factories

Use a factory when:

- runtime values affect construction
- a short-lived disposable object must be created on demand
- keyed or named resolution is genuinely required

Do not create a factory just to avoid constructor injection.

## Keyed and Named Services

Use keyed or named services only when there are truly multiple variants with a real selection policy. If there is only one implementation, a key is noise.

## DI Anti-Patterns

| Anti-pattern | Why it hurts | Fix |
|---|---|---|
| `IServiceProvider` in app code | hides dependencies, complicates tests | constructor injection |
| `BuildServiceProvider()` during registration | creates duplicate containers and lifetime bugs | let the host build the container |
| scoped into singleton | captured invalid lifetime, concurrency bugs | redesign or create a scope/factory |
| mega registration files | unreadable composition root | split by boundary with clear extension methods |
| interface per class | abstraction tax | keep abstractions at seams only |

## Review Questions

- Is every singleton safe under concurrent requests?
- Could the dependency list be simplified by splitting responsibilities?
- Are lifetimes explicit and easy to reason about from registration code?
- Is `IConfiguration` leaking into places that should receive typed settings instead?
