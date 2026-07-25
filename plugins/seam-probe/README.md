# seam-probe (plugin)

Generic NDJSON probe for embedded-runtime seams: dlopen FFI dylibs and
length-prefixed Unix-domain sockets. Carries no app-specific
knowledge — apps are described externally via small JSON manifests.
Manifest schema v2 uses a pointer-based callback-table ABI.

Modeled after `curl`, `websocat`, and `grpcurl`: protocol-generic and
API-agnostic. `ffi`, `socket`, and `inspect` emit NDJSON; `vocab` and help emit
text.

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

# Drive it, capturing probe NDJSON and SUT stderr separately
RUST_LOG=debug RUST_BACKTRACE=1 \
  seam-probe ffi --lib /path/to/libfoo.dylib --manifest /path/to/foo.manifest.json \
  > probe.ndjson 2> sut.stderr <<'EOF'
{"op":"send","lane":"events","payload":{"hello":"world"}}
{"op":"sleep_ms","ms":250}
{"op":"stop"}
EOF

# Send a be32-framed JSON message to a UDS endpoint
echo '{"op":"send","payload":{"ping":"pong"}}' | \
  seam-probe socket --path /tmp/foo.sock --framing be32 <<'EOF'
{"op":"send","payload":{"ping":"pong"}}
{"op":"sleep_ms","ms":250}
{"op":"stop"}
EOF

# Print the I/O contract
seam-probe vocab
```

## Why two streams matter

`stdout` is probe NDJSON. In FFI mode stderr is shared by probe diagnostics,
sentinel failures, panics, and the loaded library. In socket mode the SUT is a
separate process; capture its log independently.

Bugs and runtime issues are usually only diagnosable by reading
both. The skill teaches Claude how to capture, filter, and time-
correlate them.

## Skill

The plugin ships one skill (`seam-probe`) that teaches Claude how to:

- inventory a dylib's exports;
- read the app's C header to classify callback signatures;
- build a manifest;
- choose the right framing mode for an unknown UDS endpoint;
- drive deterministic input sequences from heredocs;
- capture probe NDJSON and SUT logs side-by-side;
- triage failures by correlating the two streams (panics, ABI
  mismatches, dropped events, framing errors, hangs).

It does **not** ship any app-specific manifests. Apps differ; build
your own.

## Scope

In scope:

- Native `.dylib` (Mach-O) and `.so` (ELF) libraries exposing an
  `extern "C"` lifecycle + callback struct.
- `inspect` can also parse PE/COFF `.dll` files supplied on a POSIX host.
- Unix-domain stream sockets with `be32`/`be64` framing or raw bytes.
  The reserved `varint` option reports how to use raw mode with a manual
  LEB128 prefix.

The packaged launcher and SessionStart hook require a POSIX host.
Native Windows execution is not currently shipped.

Out of scope today (use a different tool, or extend the probe):

- TCP sockets, subprocess+stdio, wasm modules, gRPC, D-Bus, HTTP,
  WebSocket, kernel modules, JNI, Python C API.

## Setup

Claude Code auto-builds through a SessionStart hook. Hosts that do not run
plugin hooks need one-time `/seam-probe:seam-probe-setup` or the Cargo command
printed by `bin/seam-probe`. A direct skill-only install does not include the
launcher or crate; install the binary independently.

Requires **`cargo`** on `PATH`
(<https://www.rust-lang.org/tools/install>). The bundled toolchain file pins
Rust **1.97.1**, and Cargo rejects older compilers through `rust-version`.

## Safety

- Loaded libraries leak intentionally — never `dlclose`. See
  `skills/seam-probe/references/safety-notes.md` for the rationale.
- Callback structs up to 64 pointer fields receive aborting sentinel slots
  beyond the manifest declaration. Larger structs are out of scope.
- Frame sizes capped at 8 MiB (FFI commands and UDS frames).
- Restricted to `extern "C"` callbacks. Anything else needs a code
  change.

## License

MIT.
