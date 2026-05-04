# Setup — build the seam-probe binary

The plugin ships the Rust **source** for the probe under
`${CLAUDE_PLUGIN_ROOT}/crate/`. You build it once, the wrapper at
`bin/seam-probe` execs the resulting release binary on every invocation.

## Why source-only

The probe loads arbitrary native code (FFI dylibs) into its own address
space. Distributing prebuilt binaries that do that is a foot-gun for
trust and verification. People who probe FFI surfaces already have a
Rust toolchain set up — building locally takes ~30s and you read the
code you ran.

## Build

In Claude Code:

```
/seam-probe-setup
```

Or directly in a shell:

```bash
cargo build --release --locked \
  --manifest-path "${CLAUDE_PLUGIN_ROOT}/crate/Cargo.toml" \
  --target-dir "${CLAUDE_PLUGIN_DATA}/target"
```

The wrapper looks for `${CLAUDE_PLUGIN_DATA}/target/release/seam-probe`
and execs it. `${CLAUDE_PLUGIN_DATA}` is the persistent per-plugin data
directory the Claude Code spec guarantees survives plugin updates.

If you build the crate **outside** of Claude Code (no plugin env vars
set), the wrapper falls back to `${XDG_CACHE_HOME:-$HOME/.cache}/seam-probe/target`
— set `--target-dir` to that path when invoking cargo by hand.

## Rebuild triggers

- Plugin updated (`/plugin marketplace update` then `/plugin update`).
- Hand-edited the crate source.
- `seam-probe vocab` errors with `seam-probe is not built`.

## Toolchain

- **Required:** Rust stable (any recent version; check
  `crate/Cargo.toml` `rust-version` if pinned).
- **Install:** <https://www.rust-lang.org/tools/install>.

If `cargo` isn't on PATH, `/seam-probe-setup` aborts with a pointer to
the install URL.
