---
description: Manual rebuild of the seam-probe Rust binary with verbose output. Only needed if the auto-build SessionStart hook failed or Rust was unavailable at session start.
argument-hint: ""
allowed-tools:
  - Bash
user-invocable: true
---

# seam-probe Setup

The plugin auto-builds the probe on session start. This command is the
verbose recovery path — use it when the auto-build failed (e.g. `cargo`
wasn't on PATH, source/lock mismatch, network issue fetching crates).

## Steps

### 1. Verify Rust is installed

```bash
if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ cargo not on PATH. Install Rust: https://www.rust-lang.org/tools/install"
  exit 1
fi
cargo --version
```

If cargo is missing, stop and tell the user to install Rust.

### 2. Build the crate (release profile)

```bash
cargo build --release --locked \
  --manifest-path "${CLAUDE_PLUGIN_ROOT}/crate/Cargo.toml" \
  --target-dir "${CLAUDE_PLUGIN_DATA}/target"
```

Output goes to `${CLAUDE_PLUGIN_DATA}/target/release/seam-probe`. That
dir survives plugin updates per the Claude Code spec.

### 3. Smoke test

```bash
seam-probe vocab | head -5
```

Should print the NDJSON I/O contract.

### 4. Report

- ✅ `seam-probe vX.Y.Z` built and on PATH
- Hand off to the `seam-probe` skill for probing work.
