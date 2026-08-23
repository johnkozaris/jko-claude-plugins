# backend-validator

Backend API and WebSocket validation via Hurl, websocat, and oauth2c. Acquires OIDC tokens with auth-code + PKCE, caches refresh tokens in macOS Keychain, and runs Hurl test suites or websocat probes against authenticated endpoints.

## What It Does

- **Hurl** for REST/HTTP validation — `.hurl` files with captures, assertions, retries
- **websocat** for WebSocket probes — one-shot or streaming with bearer auth
- **oauth2c** for OIDC token acquisition — auth-code + PKCE with cached refresh tokens

## Installation

```bash
claude --plugin-dir /path/to/myClaudeSkills/plugins/backend-validator
```

## Skills

| Skill | Purpose |
|---------|---------|
| `validate-api` | Run Hurl-based API validation, acquiring OIDC token if needed |
| `validate-ws` | Smoke-test a WebSocket endpoint with bearer auth |
| `get-dev-token` | Acquire an OIDC access token via auth-code + PKCE |

## Hook

No active runtime hooks. Reserved for future command-based hooks.

## References

- `validate-api/references/hurl-patterns.md` — captures, assertions, retries, parallel runs, CI output
- `validate-ws/references/websocat-patterns.md` — subprotocols, mTLS, reconnect, debugging
- `get-dev-token/references/oidc-token-flows.md` — grant selection, 401 diagnosis, discovery
- `get-dev-token/references/token-script.sh` — macOS token acquisition with Keychain-backed refresh caching
