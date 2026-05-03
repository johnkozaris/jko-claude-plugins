# Real-Time Transports: SignalR, Raw WebSockets, and Server-Sent Events

This reference covers all three real-time transports a Kestrel-hosted .NET backend can host. SignalR is the most opinionated, but it is **not** the default for every push scenario. Choose intentionally.

## Pick the Transport First

| Use case                                                                                                 | Best transport               | Why                                                                                                                                                                                                                                   |
| -------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| One-way server → client streaming (notifications, AI/LLM token streams, log tails, dashboards, progress) | **Server-Sent Events (SSE)** | Plain HTTP, proxy/CDN-friendly, auto-reconnect in browsers, no client SDK, trivial to consume from `EventSource`, `fetch`, or any HTTP client. In .NET 10 use `TypedResults.ServerSentEvents` with an `IAsyncEnumerable<SseItem<T>>`. |
| Bidirectional hub semantics, groups, presence, transport fallback, official multi-language clients       | **SignalR**                  | Hub method invocation, group/user targeting, reconnect-with-state, JSON or MessagePack protocol, supported clients in JavaScript, .NET, Java, Swift.                                                                                  |
| Custom wire protocol, non-SignalR peers, full framing/backpressure control, binary protocols             | **Raw WebSockets**           | Direct `HttpContext.WebSockets.AcceptWebSocketAsync()`. You own framing, heartbeats, and reconnect. Right call when the peer is not a SignalR client.                                                                                 |

## SignalR Is Not the Default

A disproportionate amount of real-time .NET code reaches for SignalR by reflex. Most often that is wrong, because:

- the use case is one-way (server pushes to client) — SSE is simpler and proxy-friendly
- the peer is not a SignalR client — raw WebSockets are the honest choice
- the team does not need groups, hub methods, or transport fallback — SignalR's machinery is pure cost

Reach for SignalR when you genuinely need hubs, groups, presence, and the supported client ecosystem. Otherwise prefer SSE for one-way streams or raw WebSockets for custom protocols.

## Server-Sent Events on Kestrel (.NET 10)

.NET 10 added first-class SSE support via `TypedResults.ServerSentEvents`, available in both Minimal APIs and controllers.

When SSE fits:

- LLM/AI token streaming
- progress updates for long-running jobs
- notification feeds, presence updates, log tails
- dashboards that subscribe to one stream of updates

When SSE does not fit:

- the client must send messages back through the same connection (use WebSockets or SignalR)
- you need binary frames
- you need transport fallback for legacy proxies that strip long-lived HTTP responses

Operational notes:

- terminate at HTTP/1.1 or HTTP/2; HTTP/2 multiplexes many SSE streams over one connection
- ensure the reverse proxy does not buffer responses (`X-Accel-Buffering: no` on nginx, response buffering disabled on YARP)
- propagate `CancellationToken` through the `IAsyncEnumerable` so disconnects close the producer
- keep payloads small; use `event:`/`id:`/`data:` framing deliberately
- treat reconnect as normal: the client may reconnect with a `Last-Event-ID` and you should be able to resume

## Raw WebSockets on Kestrel

Raw WebSockets are the right choice when:

- the peer is not a SignalR client
- you need a custom wire protocol
- you want explicit control over framing and backpressure
- you are building a transport for a non-browser client (Rust, Go, embedded)

Use `app.UseWebSockets()` plus an endpoint that calls `HttpContext.WebSockets.AcceptWebSocketAsync()`. You own:

- ping/pong and keep-alive
- reconnect protocol
- message framing (JSON, protobuf, custom)
- authorization at accept time

## SignalR vs Raw WebSockets vs SSE

For most .NET real-time backend applications on Kestrel, decide in this order:

1. Is it one-way server → client? **Use SSE.**
2. Do you need bidirectional hub semantics, groups, presence, or the supported client ecosystem? **Use SignalR.**
3. Is the peer not a SignalR client, or do you need a custom protocol? **Use raw WebSockets.**

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

| Smell                      | Signal                                                           | Fix                                                   |
| -------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------- |
| Fat hub                    | hub contains workflow logic or persistence code                  | delegate to application service                       |
| In-memory truth            | static dictionary is treated as durable state                    | move durable state outward                            |
| `ConnectionId` as identity | code assumes one connection per user forever                     | use user/group model                                  |
| Protocol confusion         | generic WebSocket peer is treated as if it were a SignalR client | use raw WebSockets or implement a real SignalR client |
| Stringly contracts         | magic method names and anonymous payloads                        | introduce typed contracts                             |
| No scale plan              | multiple nodes but no backplane/service                          | design scale-out explicitly                           |
