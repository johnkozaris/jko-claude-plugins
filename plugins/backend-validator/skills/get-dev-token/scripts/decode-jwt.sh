#!/bin/bash
# Inspect JWT claims from TOKEN or stdin. This does not verify the signature.

set -euo pipefail

TOKEN_VALUE="${TOKEN:-}"
if [ -z "$TOKEN_VALUE" ]; then
  IFS= read -r TOKEN_VALUE
fi

case "$TOKEN_VALUE" in
  *.*.*) ;;
  *)
    echo "error: access token is not JWT-shaped" >&2
    exit 1
    ;;
esac

PAYLOAD=$(printf '%s' "$TOKEN_VALUE" | awk -F. '{print $2}' | tr '_-' '/+')
case $((${#PAYLOAD} % 4)) in
  2) PAYLOAD="${PAYLOAD}==" ;;
  3) PAYLOAD="${PAYLOAD}=" ;;
  1)
    echo "error: invalid base64url payload length" >&2
    exit 1
    ;;
esac

if DECODED=$(printf '%s' "$PAYLOAD" | base64 --decode 2>/dev/null); then
  :
elif DECODED=$(printf '%s' "$PAYLOAD" | base64 -D 2>/dev/null); then
  :
else
  echo "error: cannot decode JWT payload" >&2
  exit 1
fi

printf '%s\n' "$DECODED" | jq .
