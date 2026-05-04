# seam-probe (plugin)

Generic NDJSON probe for embedded-runtime seams: dlopen FFI dylibs and
length-prefixed Unix-domain sockets. Carries no app-specific
knowledge — apps are described externally via small JSON manifests.

Modeled after `curl`, `websocat`, and `grpcurl`: protocol-generic,
API-agnostic, stdin/stdout NDJSON in and out.

## Modes

| Mode      | What it does                                       |
| --------- | -------------------------------------------------- |
| `ffi`     | dlopen a dylib + drive its surface via a manifest  |
| `socket`  | connect to a UDS + ferry framed/raw bytes          |
| `inspect` | dump exported symbols (Mach-O / ELF / PE)          |
| `vocab`   | print the NDJSON I/O contract                      |

## Quick start

```bash
# What does this dylib export?
seam-probe inspect --lib /path/to/libfoo.dylib

# Drive it (requires a manifest — see skills/seam-probe/references/manifest-schema.md)
echo '{"op":"send","lane":"events","payload":{"hello":"world"}}' | \
  seam-probe ffi --lib /path/to/libfoo.dylib --manifest /path/to/foo.manifest.json

# Send a be32-framed JSON message to a UDS endpoint
echo '{"op":"send","payload":{"ping":"pong"}}' | \
  seam-probe socket --path /tmp/foo.sock --framing be32

# Print the I/O contract
seam-probe vocab
```

## Skill

The plugin ships one skill (`seam-probe`) that teaches Claude how to:

- inventory a dylib's exports;
- read the app's C header to classify callback signatures;
- build a manifest;
- choose the right framing mode for an unknown UDS endpoint;
- interpret NDJSON output.

It does **not** ship any app-specific manifests. Apps differ; build
your own.

## Build the binary

The plugin ships a pre-built **darwin-arm64** binary at
`bin/seam-probe-darwin-arm64`. On Apple Silicon Macs, first run is
instant — no toolchain required.

For other platforms, the wrapper (`bin/seam-probe`) falls back to
building the bundled Rust source under `crate/` into
`${CLAUDE_PLUGIN_DATA}/target/release/` (the persistent plugin data
dir, survives plugin updates). Cargo rebuilds only when `Cargo.toml`,
`Cargo.lock`, or `src/` change.

| Host                    | First-run path                                     |
| ----------------------- | -------------------------------------------------- |
| Apple Silicon (darwin-arm64) | exec `bin/seam-probe-darwin-arm64` directly  |
| Anything else           | `cargo build --release` from `crate/`, ~30s        |

The fallback requires **`cargo`** on `PATH`
(<https://www.rust-lang.org/tools/install>).

### Add a pre-built binary for your platform

Build the crate and drop the binary alongside the wrapper:

```bash
cd ~/Repos/myclaudeplugins/seam-probe/crate
cargo build --release
strip target/release/seam-probe
cp target/release/seam-probe "../bin/seam-probe-$(uname -s | tr 'A-Z' 'a-z')-$(uname -m | sed 's/aarch64/arm64/;s/amd64/x86_64/')"
chmod +x ../bin/seam-probe-*
```

### Develop standalone

```bash
cd ~/Repos/myclaudeplugins/seam-probe/crate
cargo build --release
# binary at: crate/target/release/seam-probe
```

## Safety

- Loaded libraries leak intentionally — never `dlclose`. See
  `skills/seam-probe/references/safety-notes.md` for the rationale.
- Callback slots beyond the manifest's declared fields abort the
  process loudly if the runtime ever calls them. Fix the manifest.
- Frame sizes capped at 8 MiB (FFI commands and UDS frames).
- Restricted to `extern "C"` callbacks. Anything else needs a code
  change.

## License

MIT.
