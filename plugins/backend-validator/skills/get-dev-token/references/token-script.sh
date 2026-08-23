#!/bin/bash
# macOS reference implementation for a public OIDC client using auth code + PKCE.
# Required exported variables: ISSUER, CLIENT_ID
# Optional: PROJECT_NAME, SCOPES, REFRESH_SCOPE,
#           ALLOW_CROSS_ORIGIN_TOKEN_ENDPOINT

set -euo pipefail

ISSUER="${ISSUER:-}"
CLIENT_ID="${CLIENT_ID:-}"
SCOPES="${SCOPES:-openid profile email offline_access}"
REFRESH_SCOPE="${REFRESH_SCOPE:-}"
KEYCHAIN_SERVICE="${PROJECT_NAME:-app}-refresh-token"

if [ -z "$ISSUER" ] || [ -z "$CLIENT_ID" ]; then
  echo "error: ISSUER and CLIENT_ID must be exported" >&2
  exit 1
fi

case "$ISSUER" in
  https://*[\?#]*)
    echo "error: ISSUER must not contain a query or fragment" >&2
    exit 1
    ;;
  https://*) ;;
  *)
    echo "error: ISSUER must use https" >&2
    exit 1
    ;;
esac

if [ "$(uname -s)" != "Darwin" ] || ! command -v security >/dev/null 2>&1; then
  echo "error: this reference script requires macOS Keychain" >&2
  exit 1
fi

for tool in curl jq; do
  command -v "$tool" >/dev/null 2>&1 || {
    echo "error: required tool is missing: $tool" >&2
    exit 1
  }
done

DISCOVERY_URL="${ISSUER%/}/.well-known/openid-configuration"
if ! DISCOVERY=$(curl -fsS "$DISCOVERY_URL"); then
  echo "error: cannot fetch OIDC discovery document: $DISCOVERY_URL" >&2
  exit 1
fi

DISCOVERY_ISSUER=$(printf '%s' "$DISCOVERY" | jq -er '.issuer') || {
  echo "error: discovery response is not valid OIDC metadata" >&2
  exit 1
}
if [ "$DISCOVERY_ISSUER" != "$ISSUER" ]; then
  echo "error: discovery issuer does not exactly match ISSUER" >&2
  exit 1
fi

TOKEN_ENDPOINT=$(printf '%s' "$DISCOVERY" | jq -er '.token_endpoint') || {
  echo "error: discovery document has no token_endpoint" >&2
  exit 1
}
case "$TOKEN_ENDPOINT" in
  https://*) ;;
  *)
    echo "error: discovered token_endpoint must use https" >&2
    exit 1
    ;;
esac

ISSUER_ORIGIN=$(printf '%s' "$ISSUER" | sed -E 's#^(https://[^/]+).*#\1#')
TOKEN_ORIGIN=$(printf '%s' "$TOKEN_ENDPOINT" | sed -E 's#^(https://[^/]+).*#\1#')
if [ "$TOKEN_ORIGIN" != "$ISSUER_ORIGIN" ] &&
   [ "${ALLOW_CROSS_ORIGIN_TOKEN_ENDPOINT:-0}" != "1" ]; then
  echo "error: token_endpoint origin differs from issuer" >&2
  echo "Set ALLOW_CROSS_ORIGIN_TOKEN_ENDPOINT=1 only after verifying provider metadata." >&2
  exit 1
fi

kc_get() {
  security find-generic-password -s "$KEYCHAIN_SERVICE" -a "$CLIENT_ID" -w 2>/dev/null
}

kc_set() {
  # macOS security has no stdin form for -w; avoid running this on a shared host.
  security add-generic-password -U -s "$KEYCHAIN_SERVICE" -a "$CLIENT_ID" -w "$1" >/dev/null
}

check_bearer() {
  token_type=$(printf '%s' "$1" | jq -r '.token_type // empty' | tr '[:upper:]' '[:lower:]')
  if [ "$token_type" != "bearer" ]; then
    echo "error: unsupported token_type: ${token_type:-missing}" >&2
    return 1
  fi
}

RT=$(kc_get || true)
if [ -n "$RT" ]; then
  RTF=$(mktemp)
  chmod 600 "$RTF"
  printf '%s' "$RT" >"$RTF"
  trap 'rm -f "$RTF"' EXIT

  if [ -n "$REFRESH_SCOPE" ]; then
    RESP=$(curl -fsS -X POST "$TOKEN_ENDPOINT" \
      --data-urlencode "grant_type=refresh_token" \
      --data-urlencode "refresh_token@$RTF" \
      --data-urlencode "client_id=$CLIENT_ID" \
      --data-urlencode "scope=$REFRESH_SCOPE") || RESP=""
  else
    RESP=$(curl -fsS -X POST "$TOKEN_ENDPOINT" \
      --data-urlencode "grant_type=refresh_token" \
      --data-urlencode "refresh_token@$RTF" \
      --data-urlencode "client_id=$CLIENT_ID") || RESP=""
  fi

  AT=$(printf '%s' "$RESP" | jq -r '.access_token // empty' 2>/dev/null || true)
  if [ -n "$AT" ] && check_bearer "$RESP"; then
    NEW=$(printf '%s' "$RESP" | jq -r '.refresh_token // empty')
    [ -n "$NEW" ] && [ "$NEW" != "$RT" ] && kc_set "$NEW"
    printf '%s\n' "$AT"
    exit 0
  fi
  echo "warning: cached refresh token rejected; starting interactive auth" >&2
fi

command -v oauth2c >/dev/null 2>&1 || {
  echo "error: oauth2c is missing; install it from its current upstream instructions" >&2
  exit 1
}

HINT=$(security find-generic-password -s "${PROJECT_NAME:-app}-test-user" 2>/dev/null \
  | awk -F\" '/"acct"/ {print $4}') || HINT=""

TMP=$(mktemp)
TMP_ERR=$(mktemp)
trap 'rm -f "${RTF:-}" "$TMP" "$TMP_ERR"' EXIT

set -- oauth2c "$ISSUER" \
  --client-id "$CLIENT_ID" \
  --auth-method none \
  --grant-type authorization_code \
  --response-types code \
  --response-mode query \
  --pkce \
  --prompt login \
  --scopes "$(printf '%s' "$SCOPES" | tr ' ' ',')" \
  --silent
[ -n "$HINT" ] && set -- "$@" --login-hint "$HINT"

"$@" >"$TMP" 2>"$TMP_ERR" || {
  echo "error: oauth2c failed" >&2
  sed -E \
    -e 's/eyJ[A-Za-z0-9._-]+/[REDACTED_TOKEN]/g' \
    -e 's/((code|token)=)[^&[:space:]]+/\1[REDACTED]/g' \
    "$TMP_ERR" | tail -20 >&2
  exit 1
}

AT=$(jq -r '.access_token // empty' "$TMP" 2>/dev/null || true)
RT=$(jq -r '.refresh_token // empty' "$TMP" 2>/dev/null || true)
if [ -z "$AT" ] || ! check_bearer "$(cat "$TMP")"; then
  echo "error: oauth2c returned no supported access token" >&2
  jq -r '"oauth error: \(.error // "unknown") - \(.error_description // "no description")"' \
    "$TMP" >&2 2>/dev/null || true
  exit 1
fi

[ -n "$RT" ] && kc_set "$RT"
printf '%s\n' "$AT"
