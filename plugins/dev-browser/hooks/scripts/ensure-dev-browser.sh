#!/bin/bash
# SessionStart hook: detect dev-browser installation, report availability,
# and cache the help output so the skill always has version-accurate patterns.
#
# Search order:
# 1. Local patched repo at ~/Repos/dev-browser/dev-browser (preferred)
# 2. Global install (command -v dev-browser)
# 3. Plugin data directory

set -euo pipefail

ENV_FILE="${CLAUDE_ENV_FILE:-}"
HELP_CACHE="$HOME/.dev-browser/help-cache.txt"
LOCAL_REPO="$HOME/Repos/dev-browser"

found() {
  local bin="$1"
  local version="$2"
  local source="$3"
  mkdir -p "$(dirname "$HELP_CACHE")"
  "$bin" --help > "$HELP_CACHE" 2>/dev/null || true
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      echo "export DEV_BROWSER_VERSION=$version"
      echo "export DEV_BROWSER_BIN=$bin"
      echo "export DEV_BROWSER_HELP=$HELP_CACHE"
    } >> "$ENV_FILE"
  fi
  echo "dev-browser $version ready ($source). Help cached at $HELP_CACHE"
  exit 0
}

# 1. Check local patched repo
if [ -x "$LOCAL_REPO/dev-browser" ]; then
  found "$LOCAL_REPO/dev-browser" "patched" "local repo"
fi

# 2. Check global install
if command -v dev-browser &>/dev/null; then
  VERSION=$(dev-browser --version 2>/dev/null || echo "unknown")
  found "dev-browser" "$VERSION" "global"
fi

# 3. Check plugin data directory
PLUGIN_DATA="${CLAUDE_PLUGIN_DATA:-}"
if [ -n "$PLUGIN_DATA" ] && [ -x "$PLUGIN_DATA/node_modules/.bin/dev-browser" ]; then
  VERSION=$("$PLUGIN_DATA/node_modules/.bin/dev-browser" --version 2>/dev/null || echo "unknown")
  found "$PLUGIN_DATA/node_modules/.bin/dev-browser" "$VERSION" "plugin data"
fi

# Not found
if [ -n "$ENV_FILE" ]; then
  echo "export DEV_BROWSER_AVAILABLE=false" >> "$ENV_FILE"
fi

echo "dev-browser is not installed. Run: cd ~/Repos/dev-browser && ./setup.sh"
