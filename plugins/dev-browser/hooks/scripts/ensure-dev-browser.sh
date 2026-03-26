#!/bin/bash
# SessionStart hook: detect dev-browser installation and report availability.
# Sets environment variables via CLAUDE_ENV_FILE so Claude knows whether
# browser automation is available.

set -euo pipefail

ENV_FILE="${CLAUDE_ENV_FILE:-}"

# Check if dev-browser is available globally
if command -v dev-browser &>/dev/null; then
  VERSION=$(dev-browser --version 2>/dev/null || echo "unknown")
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      echo "export DEV_BROWSER_VERSION=$VERSION"
    } >> "$ENV_FILE"
  fi
  exit 0
fi

# Check if installed in plugin data directory
PLUGIN_DATA="${CLAUDE_PLUGIN_DATA:-}"
if [ -n "$PLUGIN_DATA" ] && [ -x "$PLUGIN_DATA/node_modules/.bin/dev-browser" ]; then
  VERSION=$("$PLUGIN_DATA/node_modules/.bin/dev-browser" --version 2>/dev/null || echo "unknown")
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      echo "export DEV_BROWSER_VERSION=$VERSION"
      echo "export DEV_BROWSER_BIN=$PLUGIN_DATA/node_modules/.bin/dev-browser"
    } >> "$ENV_FILE"
  fi
  exit 0
fi

# Not installed — report unavailability
if [ -n "$ENV_FILE" ]; then
  echo "export DEV_BROWSER_AVAILABLE=false" >> "$ENV_FILE"
fi

echo "dev-browser is not installed. To enable browser automation, install with: npm install -g dev-browser && dev-browser install"
