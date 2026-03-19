# Kestrel Hosting

## Kestrel Is Part of the Architecture

Do not treat Kestrel as “ops will figure it out.” Listener posture, reverse-proxy trust, request limits, and protocol choices affect correctness, exposure, and incident behavior.

## Default Hosting Stance

For most backend services:

- put Kestrel behind a reverse proxy or managed edge
- bind privately by default when the service is not meant to be directly internet-facing
- make forwarded-header trust explicit
- choose HTTP protocols intentionally

If the service is directly internet-facing, the review bar should be higher.

## Listener and Exposure Rules

- prefer explicit listeners over wildcard exposure
- know whether the process is listening on localhost, private addresses, or public interfaces
- know whether TLS terminates at Kestrel or a proxy
- know whether multiple protocols are needed or just enabled by habit

## Reverse Proxy and Forwarded Headers

Only trust proxy metadata from known proxies or networks.

Common review questions:

- are `X-Forwarded-*` headers trusted too broadly?
- are `KnownProxies` / `KnownNetworks` configured when needed?
- is host filtering explicit enough for the deployment?

## Request and Connection Limits

Defaults are not a security policy.

For exposed services, review whether the host sets or deliberately accepts defaults for:

- request body size
- header limits
- request timeouts
- connection limits
- HTTP/2 or HTTP/3 stream behavior

These choices matter more for:

- uploads
- high-volume public APIs
- gRPC
- WebSockets
- large SignalR messages

## WebSockets on Kestrel

Raw WebSockets are a different decision from SignalR.

Use raw WebSockets when:

- the peer is not a SignalR client
- you need a custom protocol
- you want direct control over framing and backpressure

Use SignalR when:

- you want hub semantics and a supported SignalR client ecosystem
- you want transport fallback and higher-level client targeting

## Kestrel Smells

| Smell | Signal | Fix |
|---|---|---|
| Wildcard exposure by habit | `ListenAnyIP` or broad URLs with no topology explanation | make listener intent explicit |
| Blind proxy trust | forwarded headers enabled with no known proxy/network policy | restrict trust boundary |
| Protocol sprawl | HTTP/1.1, HTTP/2, HTTP/3, WebSockets all enabled without need | enable only what the app uses |
| No limits story | internet-facing service relies on defaults with uploads or large messages | review and set limits deliberately |
| Socket confusion | SignalR and raw WebSockets treated as interchangeable | choose one protocol model intentionally |
