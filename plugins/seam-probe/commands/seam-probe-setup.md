---
description: Manual rebuild of the seam-probe Rust binary with verbose output. Only needed if the auto-build SessionStart hook failed or Rust was unavailable at session start.
argument-hint: ""
allowed-tools:
  - Bash
user-invocable: true
---

# seam-probe Setup

Claude Code runs the plugin build hook on session start. Use this command for
recovery there and as one-time setup on hosts that do not run plugin hooks.

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
DATA_DIR="${CLAUDE_PLUGIN_DATA:-${XDG_CACHE_HOME:-$HOME/.cache}/seam-probe}"
(
  cd "${CLAUDE_PLUGIN_ROOT}" || exit 1
  cargo build --release --locked \
    --manifest-path crate/Cargo.toml \
    --target-dir "$DATA_DIR/target"
)
```

Output goes to `$DATA_DIR/target/release/seam-probe`.

### 3. Smoke test

```bash
"${CLAUDE_PLUGIN_ROOT}/bin/seam-probe" vocab | head -5
```

Should print the NDJSON I/O contract.

### 4. Report

- Report the actual output of
  `"${CLAUDE_PLUGIN_ROOT}/bin/seam-probe" --version`.
- Hand off to the `seam-probe` skill for probing work.
