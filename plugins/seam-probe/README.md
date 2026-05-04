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

## Setup

The plugin auto-builds on session start. First session takes a few
seconds to compile in the background; subsequent sessions are
instant. If `seam-probe` is invoked before the first build finishes,
retry in a moment. Run `/seam-probe-setup` if anything looks off.

Requires **`cargo`** on `PATH`
(<https://www.rust-lang.org/tools/install>).

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
