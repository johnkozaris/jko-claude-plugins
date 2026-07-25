---
name: backend-validation
description: >-
  This skill should be used when validating an authenticated HTTP or WebSocket
  backend with Hurl, websocat, or oauth2c; acquiring or refreshing an OIDC
  access token; diagnosing a 401; or converting chained API checks into
  repeatable smoke tests. Not for unauthenticated one-off requests, UI
  validation, or repositories standardized on Postman, Bruno, or Insomnia.
---

# Backend Validation

Use the project's existing validation stack. This skill commits to Hurl for
HTTP, websocat for WebSockets, and oauth2c for interactive OIDC when the project
has not already chosen another tool. Check installed `--help` before preserving
version-sensitive flags.

## Token contract

Discover provider metadata and inspect the configured client before choosing a
grant. Authorization code + PKCE is the usual interactive choice for a public
client; machine and headless callers may need client credentials or device
authorization instead.

Treat an access token as opaque unless it has JWT shape. Decoding JWT claims is
inspection, not signature verification. Compare issuer, resource audience,
expiry, identity, and policy claims with the backend's configuration; signature
or key-rotation failures require real JWT/JWKS verification.

The bundled macOS reference script:

- validates exact issuer metadata and HTTPS endpoints;
- requires explicit approval for a cross-origin token endpoint;
- caches refresh tokens in Keychain and persists rotation;
- writes only the access token to stdout and diagnostics to stderr;
- rejects token types other than Bearer.

It lives at `references/token-script.sh`. Direct skill installs can copy it from
the skill directory; plugin commands resolve it through
`${CLAUDE_PLUGIN_ROOT}/skills/backend-validation/references/token-script.sh`.
Linux and Windows need an equivalent secret-store adapter; the shipped script
is intentionally macOS-only.

Load `references/oidc-token-flows.md` for grant selection, provider notes, and
401 diagnosis. Use `scripts/decode-jwt.sh` only for JWT-shaped tokens.

## HTTP contract

Keep token acquisition outside Hurl scenarios and pass the access token through
Hurl's secret-variable interface. Keep environment-specific URLs as ordinary
variables. Use captures for values created inside a scenario, such as resource
IDs used by later requests.

Load `references/hurl-patterns.md` for secret variables, retries, parallel
isolation, and report leakage.

## WebSocket contract

A WebSocket handshake does not define the application protocol. Discover the
endpoint, Origin policy, subprotocol, first message, and expected response from
project code/config or ask before sending arbitrary JSON. Bound the probe with
the agent's command timeout or a verified websocat option.

Verbose handshake output can expose the Authorization header. Use it only when
necessary and redact evidence. Confirm before contacting production.

Load `references/websocat-patterns.md` for the minimal one-message interface,
subprotocol/Origin handling, and failure interpretation.

CI should acquire credentials through its secret manager rather than a
developer keychain.
