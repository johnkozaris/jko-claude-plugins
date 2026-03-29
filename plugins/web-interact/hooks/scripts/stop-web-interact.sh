#!/bin/bash
set -euo pipefail

if [ -n "${WEB_INTERACT_BIN:-}" ] && [ -x "${WEB_INTERACT_BIN}" ]; then
  "${WEB_INTERACT_BIN}" stop >/dev/null 2>&1 || true
  exit 0
fi

if command -v web-interact >/dev/null 2>&1; then
  "$(command -v web-interact)" stop >/dev/null 2>&1 || true
fi
