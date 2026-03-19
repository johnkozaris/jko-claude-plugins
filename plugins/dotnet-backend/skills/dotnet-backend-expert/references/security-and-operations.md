# Security and Operations

## Auth and Authorization Boundaries

Authenticate at the host boundary. Authorize at the endpoint or hub boundary, and keep business rules separate from transport auth plumbing.

- validate tokens before work reaches application services
- use claims and policies deliberately
- prefer explicit authorization over scattered ad hoc checks
- keep tenant and user context trustworthy and derived from authenticated identity, not client payloads

Do not let handlers, hubs, or services parse raw auth headers or manually decode tokens in arbitrary places.

## JWT and Token Flow

For token-based backends:

- validate issuer, audience, signing key, and expiration explicitly
- treat clock skew and refresh behavior as operational concerns, not UI trivia
- design reconnect and retry behavior with token expiry in mind for long-lived clients such as SignalR connections

If auth is complicated enough to need many exceptions, the boundary is probably muddy.

## CORS

CORS is a backend concern when browser clients call the service.

- allow only the origins, methods, and headers the service actually needs
- be deliberate about credentials
- keep SignalR and REST CORS posture aligned when both are browser-facing

Do not ship permissive wildcard CORS on a public backend unless the use case truly demands it and the consequences are understood.

## Rate Limiting

Public-facing backends need an explicit abuse posture.

- use rate limiting when endpoints or hubs can be hammered
- shape limits by route, caller, tenant, or identity where useful
- make limits observable so production behavior can be tuned

Do not bolt rate limits on as an afterthought after traffic incidents.

## Health Checks and Shutdown

Operational readiness is part of backend quality.

- expose health or readiness checks for dependencies that actually matter
- distinguish startup/readiness concerns from liveness concerns
- make shutdown graceful for requests, workers, and long-lived connections
- ensure background work has ownership and stop semantics

Health checks should tell operators something useful, not just return 200 forever.

## Smells

| Smell | Signal | Fix |
|---|---|---|
| Auth leakage | services or handlers parse tokens manually | keep auth at the boundary |
| Permissive CORS by default | `AllowAnyOrigin` on a public backend without justification | restrict origins and methods |
| No abuse posture | public endpoints have no rate limits or throttling story | add explicit rate limiting |
| Fake health checks | readiness always returns healthy regardless of dependencies | check real critical dependencies |
| Ungraceful shutdown | workers or connections drop without ownership or drain semantics | wire graceful stop behavior deliberately |
