#!/usr/bin/env bash
# peekaboo-macos-validator — SessionStart sanity check.
#
# Silent on success. Prints a short, actionable hint to stderr if peekaboo
# is missing or the OS is not macOS, then exits 0 so the session continues.
#
# Why stderr-only-on-failure: every Claude Code session would otherwise emit
# noise. Shipping a doctor command (/peekaboo:peekaboo-doctor) is the right
# place for verbose output.

set -u

# Only macOS is supported.
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "⚠️  peekaboo-macos-validator: requires macOS (uname=$(uname -s)). Skipping." >&2
  exit 0
fi

if ! command -v peekaboo >/dev/null 2>&1; then
  cat >&2 <<'EOF'
⚠️  peekaboo-macos-validator: `peekaboo` binary not found.

Install it once with Homebrew:

    brew install steipete/tap/peekaboo

Then grant System Settings → Privacy & Security → Screen Recording AND
Accessibility to your terminal (or to whichever process invokes peekaboo).

Run `/peekaboo-macos-validator:peekaboo-doctor` for a full health check.
EOF
fi

exit 0
