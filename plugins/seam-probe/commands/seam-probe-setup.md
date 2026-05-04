---
description: Build the seam-probe Rust binary one-time (~30s). Re-run after plugin updates or crate edits.
argument-hint: ""
allowed-tools:
  - Bash
user-invocable: true
---

# seam-probe Setup

Build the bundled Rust crate into `crate/target/release/seam-probe`. The
wrapper at `bin/seam-probe` execs that binary on every invocation.

## Steps

### 1. Verify Rust is installed

```bash
if ! command -v cargo >/dev/null 2>&1; then
  echo "❌ cargo not on PATH."
  echo "   Install Rust: https://www.rust-lang.org/tools/install"
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

The build output lives in `${CLAUDE_PLUGIN_DATA}` — that directory
survives plugin updates per the Claude Code spec, so subsequent
updates only rebuild when the bundled crate sources actually changed.
First build takes ~30s; rebuilds are incremental and near-instant.

### 3. Smoke test

```bash
seam-probe vocab | head -5
```

This should print the NDJSON I/O contract. If it does, setup is
complete — the skill `seam-probe` can now be used to probe FFI
dylibs and UDS endpoints.

### 4. Report

Tell the user:

- ✅ seam-probe vX.Y.Z built and on PATH
- Hand-off to the `seam-probe` skill for actual probing work.

## When to re-run

- After `claude plugin update` if the bundled crate changed.
- After hand-editing files in `${CLAUDE_PLUGIN_ROOT}/crate/`.
- If `seam-probe vocab` errors with "seam-probe is not built".
