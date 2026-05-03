# backend-validator

Backend API and WebSocket validation via Hurl, websocat, and oauth2c. Acquires OIDC tokens with auth-code + PKCE, caches refresh tokens in OS-native secret storage, and runs Hurl test suites or websocat probes against authenticated endpoints.

## What It Does

- **Hurl** for REST/HTTP validation — `.hurl` files with captures, assertions, retries
- **websocat** for WebSocket probes — one-shot or streaming with bearer auth
- **oauth2c** for OIDC token acquisition — auth-code + PKCE with cached refresh tokens

## Installation

```bash
claude --plugin-dir /path/to/myClaudeSkills/plugins/backend-validator
```

## Commands

| Command | Purpose |
|---------|---------|
| `/validate-api` | Run Hurl-based API validation, acquiring OIDC token if needed |
| `/validate-ws` | Smoke-test a WebSocket endpoint with bearer auth |
| `/get-dev-token` | Acquire an OIDC access token via auth-code + PKCE |

## Skill

**backend-validation** — teaches Claude the Hurl + websocat + oauth2c workflow for backend validation behind OIDC auth. Activates when asked to test, validate, or smoke-check a backend API or WebSocket endpoint.

## Hook

No active runtime hooks. Reserved for future command-based hooks.

## References

- **hurl-patterns** — captures, assertions, retries, GraphQL, parallel runs, CI output
- **websocat-patterns** — subprotocols, mTLS, reconnect, debugging
- **oidc-token-flows** — grant-type comparison, 401 debugging, discovery endpoint
- **token-script.sh** — complete bash script for token acquisition + refresh caching
