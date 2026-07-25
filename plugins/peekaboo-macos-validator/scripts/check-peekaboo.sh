#!/usr/bin/env bash
# Silent SessionStart prerequisite check. Use the doctor command for details.

set -u

[[ "$(uname -s)" == "Darwin" ]] || exit 0

if ! command -v peekaboo >/dev/null 2>&1; then
  cat >&2 <<'EOF'
peekaboo-macos-validator: `peekaboo` is missing.
Install: brew install steipete/tap/peekaboo
Run `/peekaboo-macos-validator:peekaboo-doctor` to identify the runtime that
needs Screen Recording and Accessibility; it may not be the terminal.
EOF
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "peekaboo-macos-validator: jq is required for structured output." >&2
fi

exit 0
