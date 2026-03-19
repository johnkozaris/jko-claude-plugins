# SignalR

## Use SignalR for the Right Problems

SignalR fits:

- notifications
- dashboards
- collaboration
- presence updates
- low-latency server push to connected clients

It is not your general-purpose event bus, database, or cross-service source of truth.

## What SignalR Officially Connects To

SignalR is not generic WebSockets. Officially supported clients:

- JavaScript
- .NET
- Java
- Swift

If the peer is not a SignalR client, assume it does **not** speak SignalR until proven otherwise.

## Transports and Protocols

SignalR can use:

- WebSockets (preferred)
- Server-Sent Events
- Long Polling

SignalR hub payloads use JSON (default) or MessagePack. Choose MessagePack only when the payload contract is stable and both sides support it.

## SignalR vs Raw WebSockets

For most `.NET` real-time backend applications on Kestrel, prefer SignalR over raw WebSockets.

Choose SignalR for hub method semantics, connection/group management, transport fallback, and the supported client ecosystem.

Choose raw WebSockets for generic socket peers, full protocol control, custom wire contracts, or transport without the hub abstraction.

## Hub Rules

A hub is the backend real-time boundary — “controller for connected clients,” not “mini application layer.”

A hub owns:

- connection lifecycle hooks
- auth and authorization at the boundary
- mapping transport messages into application-service calls
- group join/leave orchestration
- outbound fan-out to connections, users, or groups
- cancellation tied to connection lifetime

A hub does **not** own:

- durable workflow or session truth
- multi-step business orchestration
- direct persistence choreography
- singleton-like mutable state
- long-running loops or background work

Hub method rules:

- keep methods short, async, and fully awaited
- one use-case-sized payload per method
- use typed contracts and typed hubs when client method contracts matter
- pass connection cancellation downstream
- target users or groups over individual connection IDs
- make group membership reconnect-safe and idempotent
- keep JSON as the default hub protocol for mixed ecosystems
- move complex notification orchestration into application services

Use `IHubContext<THub>` when background services, workers, or endpoints need to push to clients. Keep that send path explicit and reviewable.

## Connections and State

Connections are ephemeral:

- one user may have multiple connections
- `ConnectionId` is not a durable identity
- groups are app-managed and need rehydration after reconnect
- reconnects produce new connection IDs

Prefer user-targeting or group-targeting over manual connection bookkeeping.

Treat reconnect as normal:

- clients reconnect with a new connection ID
- ephemeral subscriptions need rejoin or rehydration
- auth tokens may need refresh during reconnect
- UI state may need resync after transport recovery

In-memory connection tracking is acceptable only on intentionally single-node deployments. The moment scale-out matters, store durable truth outside hub instances and rehydrate as clients reconnect.

## Scale-Out

Pick a model deliberately:

- Azure SignalR Service for Azure-hosted scale-out
- Redis backplane plus sticky-session awareness for self-hosted scale-out
- plain in-proc only when the topology is intentionally single-node

## Security

- authenticate every connection
- authorize hub methods deliberately
- do not trust client-provided identity or tenant context
- handle token expiry and reconnect behavior

## TypeScript and React Client Guidance

For browser clients and React applications, use the official `@microsoft/signalr` client.

**DO**:

- keep one shared connection per user/session/feature slice
- centralize the connection in a service, hook, or context
- register handlers before `start()`
- use automatic reconnect
- use `accessTokenFactory` for fresh tokens on reconnect
- unsubscribe handlers and stop the connection during cleanup
- rejoin groups or resync state after reconnect

**DON'T**:

- create one connection per component render
- leak duplicate event handlers across remounts
- assume local UI state is authoritative after reconnect
- mix transport logic into rendering code

## Rust Client Guidance

No Microsoft-supported Rust SignalR client exists (as of 2026).

A Rust client must implement the full SignalR protocol:

1. negotiate flow
2. transport establishment (usually WebSockets)
3. hub protocol handshake
4. invocation, completion, ping, close, and reconnect
5. auth token flow compatible with the backend

Rules for Rust interop:

- prefer JSON hub protocol
- keep contracts small, explicit, and versioned
- write wire-level integration tests against the actual backend
- prove reconnect, auth refresh, and group rejoin before production
- use a separate raw WebSocket or gRPC surface when protocol control matters more than hub semantics

Community clients exist (`signalrs`, `rust_signalr_client`). Verify maintenance and protocol coverage before standardizing.

## Mixed Stack Rules

**DO**:

- use SignalR when clients benefit from hub methods, groups, and reconnect
- keep one explicit contract model shared across backend and client teams
- version event names and payloads deliberately
- use JSON first for TypeScript/React and Rust interop
- test the real connection lifecycle end to end

**DON'T**:

- assume a generic WebSocket crate can call hub methods
- let unofficial client quirks define the backend contract
- let every React component manage its own connection
- make Rust interop a hidden science project without tests

## SignalR Smells

| Smell | Signal | Fix |
|---|---|---|
| Fat hub | hub contains workflow logic or persistence code | delegate to application service |
| In-memory truth | static dictionary is treated as durable state | move durable state outward |
| `ConnectionId` as identity | code assumes one connection per user forever | use user/group model |
| Protocol confusion | generic WebSocket peer is treated as if it were a SignalR client | use raw WebSockets or implement a real SignalR client |
| Stringly contracts | magic method names and anonymous payloads | introduce typed contracts |
| No scale plan | multiple nodes but no backplane/service | design scale-out explicitly |
