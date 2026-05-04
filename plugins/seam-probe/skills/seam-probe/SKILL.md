---
name: seam-probe
description: >
  This skill should be used when the user wants to drive, validate, or
  fuzz a process boundary ("seam") that exposes itself as an FFI dynamic
  library or a Unix-domain socket — for example: probing a Rust runtime
  hosted in a Swift or Electron app via its C ABI, or sending crafted
  length-prefixed frames to a daemon's UDS endpoint. Use when the user
  asks to "probe", "fuzz", "exercise", "test the FFI surface", "test the
  socket protocol", "send raw frames", or "discover what a dylib
  exports".
allowed-tools: Bash, Read, Grep, Write
---

# seam-probe

`seam-probe` is a **generic** NDJSON probe for embedded-runtime seams. It
ships **zero** app-specific knowledge. Everything app-specific is
expressed in a small JSON **manifest** that the user (or you) writes and
hands to the binary.

There are two real seams in this skill's world:

| Seam            | What it is                                            | Probe mode      |
| --------------- | ----------------------------------------------------- | --------------- |
| FFI dylib       | dlopen + extern "C" entry points + callback struct    | `seam-probe ffi` |
| UDS endpoint    | Unix-domain socket carrying length-prefixed frames    | `seam-probe socket` |

Plus two utility modes that need no manifest:

| Utility          | What it does                                          |
| ---------------- | ----------------------------------------------------- |
| `inspect`        | Dump exported symbols of a dylib (Mach-O / ELF / PE)  |
| `vocab`          | Print the stdin/stdout NDJSON contract verbatim       |

## Preflight

```bash
seam-probe vocab >/dev/null
```

If that errors with `seam-probe is not built`, the user has not run the
one-time build yet. Tell them to run `/seam-probe-setup` (or
`cd ${CLAUDE_PLUGIN_ROOT}/crate && cargo build --release`), then resume.
Build instructions live in `references/setup.md`.

If the command runs but a flag in this skill is missing or behaviour
disagrees with the docs, stop and ask the user to **update** the
plugin. Do not guess around missing features.

## When to reach for which mode

1. **You don't know what the dylib exports** → start with `seam-probe inspect`.
2. **You have an unfamiliar dylib but know its callback contract** → write
   a manifest, then `seam-probe ffi`.
3. **You have a UDS endpoint but don't know the framing** → try
   `--framing be32` first (most common); fall back to `--framing none`
   and inspect raw bytes if framing is wrong.
4. **You forget the NDJSON contract** → `seam-probe vocab`.

## Discovery workflow (do this for every new app)

Apps differ. Don't assume any of the file shapes below. Always verify
against the target app's source or generated header before writing a
manifest.

### Discover an FFI surface

1. `seam-probe inspect --lib path/to/libfoo.dylib` → list exported symbols.
2. Read the app's generated C header (look for `*.h` produced by cbindgen
   or hand-maintained alongside the dylib). The header is the source of
   truth for callback struct field order and per-field signatures.
3. For each callback field, classify the signature into one of the
   probe's three kinds (see `references/manifest-schema.md`).
4. Write the manifest. Field order in `callback_struct[]` MUST match the
   C struct's declaration order — the manifest **is** the struct
   definition for the probe's purposes.

### Discover a UDS protocol

1. Find where the server binds the socket (search for `bind`,
   `UnixListener`, `SOCK_STREAM`, framing constants).
2. Identify framing by reading the server's read loop. Look for
   `length_field_type::<u32>` (be32), explicit byte reads of 4 or 8
   bytes, or no length prefix at all.
3. Probe with `--framing be32` first. If the server hangs forever on the
   first byte, framing is wrong — try `be64` or `varint`, or fall back to
   `none` and reverse-engineer.

See `references/discover-ffi-symbols.md` and
`references/discover-socket-protocol.md` for concrete steps.

## Toy examples

These are deliberately **synthetic** — no real app embedded in the skill.
They exist to show the shape; real apps need real manifests.

### FFI: probe a dylib that exports `lib_start`/`lib_stop`/`lib_send_event`

```bash
cat > /tmp/foo.manifest.json <<'EOF'
{
  "schema_version": 1,
  "lifecycle": {
    "start_symbol": "lib_start",
    "stop_symbol":  "lib_stop"
  },
  "callback_struct": [
    { "name": "on_event", "kind": "json" }
  ],
  "lanes": [
    { "name": "events", "symbol": "lib_send_event" }
  ]
}
EOF

# Drive the runtime
{
  echo '{"op":"send","lane":"events","payload":{"hello":"world"}}'
  echo '{"op":"sleep_ms","ms":250}'
  echo '{"op":"stop"}'
} | seam-probe ffi --lib ./libfoo.dylib --manifest /tmp/foo.manifest.json
```

### UDS: send a JSON frame to a be32-framed Unix socket

```bash
echo '{"op":"send","payload":{"ping":"pong"}}' | \
  seam-probe socket --path /tmp/foo.sock --framing be32
```

### UDS: send raw hex bytes to a no-framing socket

```bash
echo '{"op":"raw","hex":"deadbeef"}' | \
  seam-probe socket --path /tmp/foo.sock --framing none
```

## Output: NDJSON, one event per line

Every line of stdout is a single JSON object terminated by `\n`. Every
line of stdin must be a single JSON object terminated by `\n`. The full
vocabulary is in `references/ndjson-vocab.md`. When in doubt run
`seam-probe vocab`.

## Safety: things you must internalise

The probe loads arbitrary native code into its own address space. Three
rules guard against UB:

1. **The probe never calls `dlclose`.** Loaded libraries leak
   intentionally — runtime-spawned worker threads may still be in
   flight when stdin closes, and unloading their `.text` segment would
   segfault. See `references/safety-notes.md`.
2. **Callback fields not declared in the manifest are bound to a
   sentinel that aborts.** If the runtime's actual callback struct has
   more fields than the manifest declares and the runtime calls into
   that slot, the probe aborts loudly rather than invoking
   uninitialised memory. The fix: add the missing field to the
   manifest.
3. **Signatures are restricted to three shapes** (see
   `references/manifest-schema.md`). Anything else (variadic, struct-
   by-value, stdcall, fastcall) requires a code change to the probe.

## Reference docs

- `references/setup.md` — one-time `cargo build --release` instructions
  for the bundled crate.
- `references/manifest-schema.md` — full manifest grammar and the three
  callback signature kinds.
- `references/discover-ffi-symbols.md` — how to learn an unfamiliar
  dylib's surface.
- `references/discover-socket-protocol.md` — how to learn an unfamiliar
  UDS endpoint's protocol.
- `references/framing-modes.md` — be32 / be64 / varint / none.
- `references/ndjson-vocab.md` — every input op and every output kind.
- `references/safety-notes.md` — never-unload, ABI over-allocation,
  shutdown grace, sentinel slots.
