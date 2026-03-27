#!/bin/bash
# SessionStart hook: detect the dev-browser CLI, report availability,
# and cache the help output so the skill always has version-accurate patterns.

set -euo pipefail

ENV_FILE="${CLAUDE_ENV_FILE:-}"
PLUGIN_DATA="${CLAUDE_PLUGIN_DATA:-}"
BIN_OVERRIDE="${DEV_BROWSER_BIN:-}"

if [ -n "$PLUGIN_DATA" ]; then
  HELP_CACHE="$PLUGIN_DATA/help-cache.txt"
else
  HELP_CACHE="$HOME/.cache/dev-browser-plugin/help-cache.txt"
fi

found() {
  local bin="$1"
  local version="$2"
  mkdir -p "$(dirname "$HELP_CACHE")"
  "$bin" --help > "$HELP_CACHE" 2>/dev/null || true
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      printf 'export DEV_BROWSER_VERSION=%q\n' "$version"
      printf 'export DEV_BROWSER_BIN=%q\n' "$bin"
      printf 'export DEV_BROWSER_HELP=%q\n' "$HELP_CACHE"
      printf "export PATH=%q:\$PATH\n" "$(dirname "$bin")"
    } >>"$ENV_FILE"
  fi
  echo "dev-browser $version ready (CLI detected). Help cached at $HELP_CACHE"
  exit 0
}

# Check explicit binary override first.
if [ -n "$BIN_OVERRIDE" ] && [ -x "$BIN_OVERRIDE" ]; then
  VERSION=$("$BIN_OVERRIDE" --version 2>/dev/null || echo "unknown")
  found "$BIN_OVERRIDE" "$VERSION"
fi

# Then fall back to PATH discovery.
if command -v dev-browser >/dev/null 2>&1; then
  RESOLVED_BIN="$(command -v dev-browser)"
  VERSION=$("$RESOLVED_BIN" --version 2>/dev/null || echo "unknown")
  found "$RESOLVED_BIN" "$VERSION"
fi

if [ -n "$ENV_FILE" ]; then
  {
    echo "export DEV_BROWSER_AVAILABLE=false"
    printf 'export DEV_BROWSER_HELP=%q\n' "$HELP_CACHE"
  } >>"$ENV_FILE"
fi

echo "dev-browser CLI not found. Install or build it, make sure 'dev-browser' is on PATH (or set DEV_BROWSER_BIN), then run: dev-browser install"
