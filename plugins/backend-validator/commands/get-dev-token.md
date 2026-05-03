---
description: Acquire a development OIDC access token via authorization-code + PKCE, caching the refresh token in macOS keychain.
allowed-tools:
  - Read
  - Write
  - Edit
  - Bash
user-invocable: true
---

# /get-dev-token

Acquire a bearer JWT for local backend validation. First run opens a browser; subsequent runs refresh silently until the refresh token expires.

## Steps

1. **If a token script already exists in the project**, run it. Otherwise, scaffold one from `references/token-script.sh` in the `backend-validation` skill. Pick a path that fits the repo's conventions; gitignore any file that holds the real issuer URL / client ID.

2. **One-time OIDC provider setup** (do this once per project, then document it):
   - Redirect URI `http://localhost:9876/callback` whitelisted (Strict mode)
   - Authentication flow attached to the provider (Authentik: `default-authentication-flow`)
   - Test user has `email_verified: true` on their profile attributes

3. **Run and verify.** Decode the returned JWT's claims and confirm `preferred_username`, `email_verified`, `aud`, and `exp` are what you expect — especially the username, in case the browser picked up the wrong session.

## Guardrails

- Don't print the full token to stdout by default. `export TOKEN=$(/get-dev-token)` is the ergonomic shape.
- If the first run caches the wrong identity, `security delete-generic-password -s <project>-refresh-token` and re-run.
- Never commit the file that holds the issuer URL / client ID.

## When to defer to the skill

For the full flow landscape (auth code vs client credentials vs device), 401 root-cause debugging, or a new OIDC provider, consult the `backend-validation` skill + `references/oidc-token-flows.md`.
