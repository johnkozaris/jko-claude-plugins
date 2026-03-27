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
  local runtime_status="unknown"
  local runtime_ready="false"
  local install_needed="false"
  local status_output=""
  local display_name="dev-browser"

  if [ -n "$version" ] && [ "$version" != "unknown" ]; then
    display_name="dev-browser $version"
  fi

  if status_output=$("$bin" status 2>&1); then
    runtime_status="ready"
    runtime_ready="true"
  elif [[ "$status_output" == *"Embedded daemon dependencies are missing. Run \`dev-browser install\` first."* ]]; then
    runtime_status="install-required"
    install_needed="true"
  fi

  mkdir -p "$(dirname "$HELP_CACHE")"
  "$bin" --help > "$HELP_CACHE" 2>/dev/null || true
  if [ -n "$ENV_FILE" ]; then
    {
      echo "export DEV_BROWSER_AVAILABLE=true"
      printf 'export DEV_BROWSER_RUNTIME_STATUS=%q\n' "$runtime_status"
      printf 'export DEV_BROWSER_RUNTIME_READY=%q\n' "$runtime_ready"
      printf 'export DEV_BROWSER_INSTALL_NEEDED=%q\n' "$install_needed"
      printf 'export DEV_BROWSER_VERSION=%q\n' "$version"
      printf 'export DEV_BROWSER_BIN=%q\n' "$bin"
      printf 'export DEV_BROWSER_HELP=%q\n' "$HELP_CACHE"
      printf "export PATH=%q:\$PATH\n" "$(dirname "$bin")"
    } >>"$ENV_FILE"
  fi
  case "$runtime_status" in
    ready)
      echo "$display_name ready. Help cached at $HELP_CACHE"
      ;;
    install-required)
      echo "$display_name detected, but the embedded runtime is not installed yet. Run: dev-browser install"
      ;;
    *)
      echo "$display_name detected, but runtime readiness could not be confirmed. Help cached at $HELP_CACHE"
      ;;
  esac
  exit 0
}

detect_version() {
  local bin="$1"
  "$bin" --version 2>/dev/null || "$bin" -V 2>/dev/null || echo "unknown"
}

# Check explicit binary override first.
if [ -n "$BIN_OVERRIDE" ] && [ -x "$BIN_OVERRIDE" ]; then
  VERSION=$(detect_version "$BIN_OVERRIDE")
  found "$BIN_OVERRIDE" "$VERSION"
fi

# Then fall back to PATH discovery.
if command -v dev-browser >/dev/null 2>&1; then
  RESOLVED_BIN="$(command -v dev-browser)"
  VERSION=$(detect_version "$RESOLVED_BIN")
  found "$RESOLVED_BIN" "$VERSION"
fi

if [ -n "$ENV_FILE" ]; then
  {
    echo "export DEV_BROWSER_AVAILABLE=false"
    echo "export DEV_BROWSER_RUNTIME_STATUS=missing"
    echo "export DEV_BROWSER_RUNTIME_READY=false"
    echo "export DEV_BROWSER_INSTALL_NEEDED=false"
    printf 'export DEV_BROWSER_HELP=%q\n' "$HELP_CACHE"
  } >>"$ENV_FILE"
fi

echo "dev-browser CLI not found. Install or build it, make sure 'dev-browser' is on PATH (or set DEV_BROWSER_BIN), then run: dev-browser install"
