#!/bin/sh
# hooks/scripts/rebuild-if-needed.sh
#
# Runs from the SessionStart hook in async mode. Fingerprints the bundled
# toolchain, Cargo manifests, vocabulary, and Rust source; if they differ from
# the successful build stamp, rebuilds the probe from source. Exit codes:
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

STAMP=$DATA/source.stamp
BIN=$DATA/target/release/seam-probe

SOURCE_STAMP=$(
  cd "$ROOT" || exit 2
  find .cargo/config.toml rust-toolchain.toml crate/Cargo.toml crate/Cargo.lock crate/VOCAB.md crate/src -type f -print |
    LC_ALL=C sort |
    while IFS= read -r file; do
      cksum "$file" | awk -v file="$file" '{ print file ":" $1 ":" $2 }'
    done |
    cksum |
    awk '{ print $1 ":" $2 }'
) || {
  echo "seam-probe: cannot fingerprint bundled Rust source" >&2
  exit 2
}

# Fast path: binary exists and its successful-build stamp matches all sources.
if [ -x "$BIN" ] && [ -f "$STAMP" ] &&
   [ "$(cat "$STAMP")" = "$SOURCE_STAMP" ]; then
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

BUILD_LOG=$DATA/build.$$.log
if (
  cd "$ROOT" || exit 2
  cargo build --release --locked \
    --manifest-path crate/Cargo.toml \
    --target-dir "$DATA/target"
) >"$BUILD_LOG" 2>&1; then
  rm -f "$BUILD_LOG"
  STAMP_TMP=$STAMP.$$
  if printf '%s\n' "$SOURCE_STAMP" >"$STAMP_TMP" &&
     mv "$STAMP_TMP" "$STAMP"; then
    exit 0
  fi
  rm -f "$STAMP_TMP"
  echo "seam-probe: build succeeded but source stamp could not be saved" >&2
  exit 2
else
  rm -f "$STAMP"
  cat "$BUILD_LOG" >&2
  rm -f "$BUILD_LOG"
  echo "seam-probe: cargo build failed. Follow the setup fallback in the seam-probe skill." >&2
  exit 2
fi
