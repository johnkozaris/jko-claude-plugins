# OIDC token flows and diagnosis

Read the provider discovery document and client configuration before choosing a
grant. Public interactive clients usually use authorization code + PKCE.
Machine callers may use client credentials; headless user authorization may use
device authorization when the provider enables it. Avoid password grants for
new integrations.

Refresh-token lifetime, scope, and rotation are provider policy. A refresh
request may omit scope; when supplied it cannot expand the original grant.
Persist a rotated refresh token returned by the endpoint.

OIDC requires the discovered issuer identifier to match the configured issuer
exactly. Token endpoints can legitimately use another HTTPS origin; the bundled
script rejects that by default and requires
`ALLOW_CROSS_ORIGIN_TOKEN_ENDPOINT=1` after the metadata is verified.

Access tokens may be opaque. For a JWT-shaped token, `scripts/decode-jwt.sh`
inspects claims using Base64url decoding, but does not verify signature,
algorithm, key rotation, or authorization.

For a 401, compare the backend's configured issuer and resource audience,
expiry/leeway, required roles/groups/tenant/identity claims, and JWT/JWKS
verification errors. Client-credentials tokens can contain `sub`; its meaning is
provider-specific.

Provider notes -- verify against the tenant:

- Authentik clients require an authentication-flow binding; the exact flow is
  tenant configuration.
- When a backend requires `email_verified: true`, Authentik user attributes use
  a plain YAML scalar rather than a one-item list.
- Authentik and Auth0 deployments may rotate refresh tokens when rotation is
  enabled.

Provider metadata describes server capability, not necessarily grants enabled
for one client.
