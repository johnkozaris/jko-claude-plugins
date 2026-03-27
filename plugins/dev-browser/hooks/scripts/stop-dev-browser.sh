#!/bin/bash
set -euo pipefail

if [ -n "${DEV_BROWSER_BIN:-}" ] && [ -x "${DEV_BROWSER_BIN}" ]; then
  "${DEV_BROWSER_BIN}" stop >/dev/null 2>&1 || true
  exit 0
fi

if command -v dev-browser >/dev/null 2>&1; then
  "$(command -v dev-browser)" stop >/dev/null 2>&1 || true
fi
