#!/bin/bash
# Reference implementation for OIDC token acquisition + refresh caching.
# Copy into a project as scripts/get-backend-token.sh, chmod +x, and customize.
#
# Expects two env vars (via .env.<something>.local or shell):
#   ISSUER=https://<auth-server>/application/o/<app-slug>/
#   CLIENT_ID=<public-pkce-client-id>
#   PROJECT_NAME=<short-name>    # used for keychain service name, optional

set -euo pipefail

ISSUER="${ISSUER:-}"
CLIENT_ID="${CLIENT_ID:-}"
SCOPES="openid profile email offline_access"
KEYCHAIN_SERVICE="${PROJECT_NAME:-app}-refresh-token"

if [ -z "$ISSUER" ] || [ -z "$CLIENT_ID" ]; then
  echo "error: ISSUER and CLIENT_ID must be set" >&2
  exit 1
fi

# Try cached refresh_token first
RT=$(security find-generic-password -s "$KEYCHAIN_SERVICE" -a "$CLIENT_ID" -w 2>/dev/null || true)
if [ -n "$RT" ]; then
  RESP=$(curl -sS -X POST "${ISSUER%/}/token/" \
    --data-urlencode "grant_type=refresh_token" \
    --data-urlencode "refresh_token=$RT" \
    --data-urlencode "client_id=$CLIENT_ID" \
    --data-urlencode "scope=$SCOPES")
  AT=$(echo "$RESP" | jq -r '.access_token // empty')
  if [ -n "$AT" ]; then
    # IdPs rotate refresh tokens — update keychain if we got a new one
    NEW=$(echo "$RESP" | jq -r '.refresh_token // empty')
    if [ -n "$NEW" ] && [ "$NEW" != "$RT" ]; then
      security add-generic-password -U -s "$KEYCHAIN_SERVICE" -a "$CLIENT_ID" -w "$NEW"
    fi
    echo "$AT"
    exit 0
  fi
  echo "warning: cached refresh_token rejected, falling back to interactive auth" >&2
fi

# Interactive auth-code + PKCE via oauth2c
command -v oauth2c >/dev/null || {
  echo "error: oauth2c not installed. Run: brew install oauth2c" >&2
  exit 1
}

# Optional: login hint from a separate keychain entry so oauth2c pre-fills username
HINT=$(security find-generic-password -s "${PROJECT_NAME:-app}-test-user" 2>/dev/null \
  | awk -F\" '/"acct"/ {print $4}')
HINT_ARGS=()
[ -n "$HINT" ] && HINT_ARGS=(--login-hint "$HINT")

TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

oauth2c "$ISSUER" \
  --client-id "$CLIENT_ID" \
  --auth-method none \
  --grant-type authorization_code \
  --response-types code \
  --response-mode query \
  --pkce \
  --prompt login \
  "${HINT_ARGS[@]}" \
  --scopes "openid,profile,email,offline_access" \
  --silent >"$TMP" 2>&1 || {
    echo "error: oauth2c failed" >&2
    cat "$TMP" >&2
    exit 1
  }

AT=$(jq -r '.access_token // empty' "$TMP")
RT=$(jq -r '.refresh_token // empty' "$TMP")

if [ -z "$AT" ]; then
  echo "error: no access_token in oauth2c response" >&2
  cat "$TMP" >&2
  exit 1
fi

[ -n "$RT" ] && security add-generic-password -U -s "$KEYCHAIN_SERVICE" -a "$CLIENT_ID" -w "$RT"
echo "$AT"
