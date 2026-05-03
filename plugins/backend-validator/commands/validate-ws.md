---
description: Smoke-test a WebSocket endpoint with a cached OIDC bearer token via websocat.
argument-hint: "[ws-path]"
allowed-tools:
  - Read
  - Bash
user-invocable: true
---

# /validate-ws

One-shot WebSocket smoke test. Sends one message, reads one message back, reports whether the handshake + auth succeeded.

## Steps

1. **Path argument.** If provided, use it (e.g. `/stream`, `/events`, `/ws`). Otherwise ask the user which endpoint.

2. **Base URL** from project config. Respect `WS_BASE_URL` if set.

3. **Acquire token.** Use `$TOKEN` if set, otherwise shell out to `scripts/get-backend-token.sh`.

4. **Send the probe.**

   ```bash
   echo '{"op":"ping"}' | websocat -n1 \
     -H "Authorization: Bearer $TOKEN" \
     $WS_BASE_URL$PATH
   ```

5. **Interpret the result.**
   - **Exit 0 + output** — connection and auth succeeded, server responded. Pass.
   - **Exit 0 + no output** — handshake OK but server doesn't push first. Depends on protocol; may still be correct.
   - **HTTP 401** — token rejected. Check `iss`, `aud`, `exp` via JWT decode.
   - **HTTP 403** — authenticated but not authorized for this path. Check policy/roles.
   - **Connection refused** — backend not running on the expected port.
   - **Subprotocol mismatch** — if the server expects `graphql-transport-ws` etc., retry with `--protocol <name>`.

## Guardrails

- Don't dump the token in stdout. Use `-v` only when debugging — it prints headers.
- If the endpoint requires a specific Origin, the server may reject without it; pass `-H "Origin: http://localhost:5173"` if needed.

## When to defer to the skill

For deeper debugging (reconnect, mTLS, subprotocols, ping/pong, or interactive sessions), invoke the `backend-validation` skill and consult `references/websocat-patterns.md`.
