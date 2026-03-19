# AppHost & .NET Aspire

## What AppHost Is

`AppHost` is an orchestration layer for running related services and resources together. It is a composition root above service projects.

Use it to describe:

- service startup relationships
- local/distributed development wiring
- resource dependencies such as databases, brokers, caches
- shared telemetry, health, discovery, or resilience setup through service defaults

## What AppHost Is Not

AppHost is not:

- a domain layer
- the place for business rules
- the place for request handlers, hub methods, or data access
- a substitute for production infrastructure design by itself

## Good Uses

- several services or workers must run together locally
- onboarding or local orchestration is painful today
- the team benefits from consistent service defaults and observability
- the backend is truly multi-process or distributed enough to justify orchestration

## When Not to Add Aspire

Skip it when:

- the app is a simple single-service backend
- existing Docker Compose or platform tooling already solves local orchestration cleanly
- AppHost would add more moving parts than it removes

## Boundary Rules

- keep service projects independently runnable and testable
- keep domain and application code unaware of AppHost
- keep AppHost focused on wiring resources and process relationships
- keep service defaults generic and cross-cutting, not domain-specific

## Smells

- business workflow logic in AppHost
- AppHost becomes the only place a service can be understood
- service projects depend inward on AppHost abstractions
- Aspire added because it feels modern, not because the architecture needs it
