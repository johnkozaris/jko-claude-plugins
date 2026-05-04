#!/bin/sh
# hooks/scripts/rebuild-if-needed.sh
#
# Runs from the SessionStart hook in async mode. Compares the bundled
# Cargo.lock against the copy stashed in the persistent data dir; if
# they differ (first install or plugin update with crate changes),
# rebuilds the probe from source. Exit codes:
#   0  no work needed, or build succeeded
#   2  build failed or environment misconfigured (asyncRewake surfaces
#      stderr to Claude as a system reminder)

set -u

ROOT=${CLAUDE_PLUGIN_ROOT:-}
DATA=${CLAUDE_PLUGIN_DATA:-}

if [ -z "$ROOT" ] || [ -z "$DATA" ]; then
  echo "seam-probe: CLAUDE_PLUGIN_ROOT/CLAUDE_PLUGIN_DATA not set; hook misconfigured" >&2
  exit 2
fi

SRC_LOCK=$ROOT/crate/Cargo.lock
STAMP=$DATA/Cargo.lock
BIN=$DATA/target/release/seam-probe

# Fast path: binary exists AND build stamp matches the bundled lock.
if [ -x "$BIN" ] && diff -q "$SRC_LOCK" "$STAMP" >/dev/null 2>&1; then
  exit 0
fi

mkdir -p "$DATA" || {
  echo "seam-probe: cannot create plugin data dir at $DATA" >&2
  exit 2
}

if ! command -v cargo >/dev/null 2>&1; then
  echo "seam-probe: cargo not on PATH. Install Rust (https://www.rust-lang.org/tools/install), then reload the plugin." >&2
  exit 2
fi

# Copy the stamp first; remove on failure so next session retries.
cp "$SRC_LOCK" "$STAMP"

if cargo build --release --locked \
     --manifest-path "$ROOT/crate/Cargo.toml" \
     --target-dir "$DATA/target" >&2; then
  exit 0
fi

rm -f "$STAMP"
echo "seam-probe: cargo build failed. Run /seam-probe-setup for verbose output." >&2
exit 2
