#!/bin/bash
# SessionStart hook: detect dev-browser installation, report availability,
# and cache the help output so the skill always has version-accurate patterns.

set -euo pipefail

ENV_FILE="${CLAUDE_ENV_FILE:-}"
HELP_CACHE="$HOME/.dev-browser/help-cache.txt"

cache_help() {
  local bin="$1"
  "$bin" --help > "$HELP_CACHE" 2>/dev/null || true
}

# Check if dev-browser is available globally
if command -v dev-browser &>/dev/null; then
  VERSION=$(dev-browser --version 2>/dev/null || echo "unknown")
  cache_help "dev-browser"
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      echo "export DEV_BROWSER_VERSION=$VERSION"
      echo "export DEV_BROWSER_HELP=$HELP_CACHE"
    } >> "$ENV_FILE"
  fi
  echo "dev-browser $VERSION ready. Help cached at $HELP_CACHE — read it before your first dev-browser command."
  exit 0
fi

# Check if installed in plugin data directory
PLUGIN_DATA="${CLAUDE_PLUGIN_DATA:-}"
if [ -n "$PLUGIN_DATA" ] && [ -x "$PLUGIN_DATA/node_modules/.bin/dev-browser" ]; then
  VERSION=$("$PLUGIN_DATA/node_modules/.bin/dev-browser" --version 2>/dev/null || echo "unknown")
  cache_help "$PLUGIN_DATA/node_modules/.bin/dev-browser"
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      echo "export DEV_BROWSER_VERSION=$VERSION"
      echo "export DEV_BROWSER_BIN=$PLUGIN_DATA/node_modules/.bin/dev-browser"
      echo "export DEV_BROWSER_HELP=$HELP_CACHE"
    } >> "$ENV_FILE"
  fi
  echo "dev-browser $VERSION ready. Help cached at $HELP_CACHE — read it before your first dev-browser command."
  exit 0
fi

# Not installed — report unavailability
if [ -n "$ENV_FILE" ]; then
  echo "export DEV_BROWSER_AVAILABLE=false" >> "$ENV_FILE"
fi

echo "dev-browser is not installed. To enable browser automation, install with: npm install -g dev-browser && dev-browser install"
