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
(
  cd "${CLAUDE_PLUGIN_ROOT}" || exit 1
  rustc --version
  cargo --version
)
```

If cargo is missing, stop and tell the user to install Rust. The bundled
`rust-toolchain.toml` selects Rust 1.97.1; rustup installs that pinned toolchain
on first use when necessary.

### 2. Build the crate (release profile)

```bash
(
  cd "${CLAUDE_PLUGIN_ROOT}" || exit 1
  cargo build --release --locked \
    --manifest-path crate/Cargo.toml \
    --target-dir "${CLAUDE_PLUGIN_DATA}/target"
)
```

Output goes to `${CLAUDE_PLUGIN_DATA}/target/release/seam-probe`. That
dir survives plugin updates per the Claude Code spec.

### 3. Smoke test

```bash
seam-probe vocab | head -5
```

Should print the NDJSON I/O contract.

### 4. Report

- ✅ `seam-probe v0.2.0` built with Rust 1.97.1 and on PATH
- Hand off to the `seam-probe` skill for probing work.
