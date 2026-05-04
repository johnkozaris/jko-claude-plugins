---
name: seam-probe
description: >
  Use this skill to test, validate, fuzz, or debug a process boundary
  ("seam") that exposes itself as an FFI dynamic library or a Unix-domain
  socket — for example probing a Rust runtime hosted in a Swift or
  Electron app via its C ABI, or sending crafted length-prefixed frames
  to a daemon's UDS endpoint. The skill teaches how to drive the probe
  from the CLI, capture both the probe's NDJSON output AND the system-
  under-test's stderr/logs, and triage failures by correlating the two
  streams. Reach for it when the user asks to "probe", "fuzz",
  "exercise", "test the FFI surface", "test the socket protocol",
  "send raw frames", "discover what a dylib exports", "find a bug in
  the runtime", "reproduce a crash", or "see why the seam panics or
  hangs".
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

### Scope: what counts as a "seam"

The probe handles **dynamically-loaded native code** (FFI mode) and
**Unix-domain sockets carrying byte frames** (socket mode). Concretely
in scope:

- `.dylib` (Mach-O), `.so` (ELF), `.dll` (PE) loaded via `dlopen` /
  `LoadLibrary`. The library must export an `extern "C"` lifecycle
  function and accept a callback struct.
- Unix-domain stream sockets with `be32` / `be64` / `varint` framing,
  or no framing at all.

Out of scope today (would need probe extensions, not just a manifest):

- **TCP sockets** — same framing logic would apply, but the binary
  has no `--tcp host:port` mode yet. For loopback testing of a TCP
  service, fall back to `nc`/`socat`/`websocat`.
- **Subprocess + stdio** — driving a CLI that reads NDJSON on stdin
  and emits responses on stdout. Common for "headless mode" Rust
  runtimes. For this, just use shell pipes + `tee`.
- **Wasm modules** — `wasmtime`/`wasmer` calling exports with import
  shims. Different ABI from C; not implemented.
- **Statically-linked binaries** — no dynamic surface to probe; you'd
  drive them via their stdio or socket interface instead.
- **gRPC, D-Bus, HTTP, WebSocket** — use `grpcurl`, `busctl`, `curl`,
  `websocat` respectively.
- **Kernel modules, JNI, Python C API** — out of scope.

If the user wants to probe one of the out-of-scope surfaces, say so
and suggest the right tool rather than forcing a fit.

## Preflight

```bash
seam-probe vocab >/dev/null
```

If this errors with `seam-probe is not built`, the SessionStart hook
either hasn't finished yet (fresh install or plugin update — wait a
moment and retry) or it failed. Tell the user to run
`/seam-probe-setup` for verbose recovery output.

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
5. **You're hunting a bug, panic, or hang** → run the probe with stderr
   captured (FFI) or with the SUT log tailed (socket) and correlate the
   two streams. See "Read both streams" below.

## How to invoke the CLI

The probe is one binary with four subcommands. **Always separate stdout
(NDJSON) from stderr (probe diagnostics + anything the loaded library
prints).** The two streams answer different questions and you usually
need both.

### `inspect` — list a dylib's exports

```bash
seam-probe inspect --lib /path/to/libfoo.dylib
```

No manifest, no stdin. Emits one `symbol` line per export plus a
`summary` line. Use it before writing a manifest, or to confirm a
build actually exposes what you expect.

### `ffi` — drive a dylib via a manifest

```bash
seam-probe ffi \
    --lib /path/to/libfoo.dylib \
    --manifest /path/to/foo.manifest.json \
    [--no-events] \
    [--shutdown-grace-ms 2000]
```

| Flag | When to use it |
| --- | --- |
| `--lib` | Required. Shared library to dlopen. |
| `--manifest` | Required. JSON describing the FFI surface (`references/manifest-schema.md`). |
| `--no-events` | Mute callback `event`/`json_with_sid`/`raw_with_seq` lines. Use when you only care about send/call return codes. |
| `--shutdown-grace-ms` | Time to wait between `stop` and process exit. Bump it (e.g. `5000`) if the runtime has worker threads that need longer to drain. Symptom of "too short": probe hangs at exit, or the runtime later complains about lost messages. |

Read input from stdin (one JSON op per line). Heredoc is the cleanest
way to drive a deterministic sequence:

```bash
seam-probe ffi --lib ./libfoo.dylib --manifest /tmp/foo.manifest.json \
    > /tmp/probe.ndjson 2> /tmp/sut.stderr <<'EOF'
{"op":"call","name":"start_session","arg":"sess-1"}
{"op":"send","lane":"events","payload":{"hello":"world"}}
{"op":"sleep_ms","ms":500}
{"op":"stop"}
EOF
echo "exit=$?"
```

### `socket` — drive a UDS endpoint

```bash
seam-probe socket \
    --path /tmp/foo.sock \
    [--framing be32|be64|varint|none] \
    [--no-events]
```

| Flag | When to use it |
| --- | --- |
| `--path` | Required. The Unix socket the SUT bound. |
| `--framing` | Default `be32`. Switch when the SUT's read loop expects something else (`references/framing-modes.md`). |
| `--no-events` | Suppress inbound `frame` events; emit only `rc`/`control`/`error`. |

Socket mode talks to a separate process — start it (or have the user
start it) and tail its log file in parallel. See "Read both streams".

### `vocab` — print the I/O contract

```bash
seam-probe vocab
```

Run this when you forget input ops or output kinds. Cheaper than
reading `references/ndjson-vocab.md`.

### Useful env vars (FFI mode only)

The loaded library runs in the probe's process. Anything it prints to
stderr lands on the probe's stderr. Two Rust-runtime knobs commonly
matter:

| Env var | Effect |
| --- | --- |
| `RUST_LOG=debug` (or `trace`) | Most Rust runtimes use `tracing` or `env_logger`; this dumps internal events to stderr. Invaluable for understanding why a callback never fired. |
| `RUST_BACKTRACE=1` | Print a backtrace on panic. The whole probe process aborts on a Rust panic, but you still get the stack. |

Apply at invocation:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 \
  seam-probe ffi --lib ./libfoo.dylib --manifest /tmp/foo.manifest.json \
  > /tmp/probe.ndjson 2> /tmp/sut.stderr < /tmp/input.ndjson
```

For non-Rust runtimes use whatever logging-level/destination knob the
runtime exposes.

## Read both streams (probe NDJSON + SUT logs)

The probe tells you **what the seam returned**. The SUT's logs tell
you **why**. Bugs and runtime errors are usually only visible by
correlating the two — never reason from probe output alone unless the
issue is a probe-side parse error or transport error.

### FFI: same process

The loaded library shares the probe's address space, so:

- The library's stderr → the probe's stderr.
- A panic in the library aborts the probe.
- A SIGSEGV in the library kills the probe.

Capture and inspect:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 \
  seam-probe ffi --lib ./libfoo.dylib --manifest M.json \
  > /tmp/probe.ndjson 2> /tmp/sut.stderr < /tmp/input.ndjson
echo "exit=$?"

echo "--- probe NDJSON (head) ---"; head -50 /tmp/probe.ndjson
echo "--- SUT stderr (head) ---";   head -50 /tmp/sut.stderr
echo "--- probe errors only ---";   grep '"kind":"error"' /tmp/probe.ndjson
echo "--- SUT panics/errors ---";   grep -iE 'panic|error|warn|fatal' /tmp/sut.stderr
```

### Socket: separate processes

The SUT runs independently. Either the user already has it running
(ask where logs go), or you start it yourself:

```bash
SOCK=/tmp/foo.sock
LOG=/tmp/sut.log

rm -f $SOCK
RUST_LOG=debug ./target/debug/sut > $LOG 2>&1 &
SUT_PID=$!

# Wait up to 2.5s for the socket to appear.
for _ in $(seq 1 50); do [ -S $SOCK ] && break; sleep 0.05; done
[ -S $SOCK ] || { echo "SUT failed to bind"; cat $LOG; kill $SUT_PID 2>/dev/null; exit 1; }

seam-probe socket --path $SOCK --framing be32 \
  > /tmp/probe.ndjson < /tmp/input.ndjson
PROBE_RC=$?

kill -TERM $SUT_PID 2>/dev/null; wait $SUT_PID 2>/dev/null

echo "probe=$PROBE_RC"
echo "--- probe ---"; cat /tmp/probe.ndjson
echo "--- sut  ---"; cat $LOG
```

If the user already has the SUT running, ask:
1. The socket path.
2. Where logs go (file? stderr to a pane? `journalctl -u …`? docker
   log stream?). Read from there.

### Triage table

| Probe NDJSON shows | SUT log likely shows | Most likely cause |
| --- | --- | --- |
| `kind:"rc"` `rc!=0` after `call` | `error!()` line near that ts (or nothing) | Manifest-declared symbol returned non-zero. Check runtime's error semantics. |
| no `event` lines despite `send` | `WARN dropped event` or silence | Lane not wired in runtime, or a callback field is missing/wrong in the manifest. |
| probe process aborts mid-run | `thread '…' panicked at …` + backtrace | Bug in the runtime. **This is what the probe is for** — capture stdin as a repro. |
| probe process aborts, no panic | `Segmentation fault (core dumped)` | ABI mismatch. Manifest's `callback_struct[]` is wrong (field order or signature kind). Re-run `inspect`, recheck the C header. |
| probe hangs after `stop` | `worker N still running` or nothing | Runtime didn't honour shutdown. Try `--shutdown-grace-ms 5000`; if still hangs → runtime bug. |
| `kind:"error"` `msg:"unknown lane"` | nothing | Typo in `lane` op or in `manifest.lanes[].name`. |
| `kind:"error"` `msg:"connect refused"` (socket) | `bind: Address already in use` or nothing | SUT not listening, wrong path, or permission. `ls -l $SOCK`. |
| probe hangs after first `send` (socket) | nothing | Wrong framing. Try `be64`, `varint`, `none`. |
| `kind:"error"` `msg:"frame too large"` (socket) | `wrote oversized frame` | Framing mismatch the other way. |
| `kind:"error"` `msg:"connection reset"` (socket) | panic / `signal: aborted` | SUT crashed handling your message. Capture as repro. |

For the long-form playbook with recovery steps for each pattern, see
`references/observability.md`.

### Filter NDJSON cheaply

```bash
jq -c 'select(.kind=="error")' /tmp/probe.ndjson           # probe errors
jq -c 'select(.kind=="rc" and .rc!=0)' /tmp/probe.ndjson   # failed ops
jq -c 'select(.kind=="event")' /tmp/probe.ndjson           # all events
jq -c 'select(.kind=="event" and .callback=="on_x")' /tmp/probe.ndjson
```

If `jq` is missing, `grep '"kind":"error"'` covers most needs.

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

- `references/manifest-schema.md` — full manifest grammar and the three
  callback signature kinds.
- `references/observability.md` — long-form triage and log-correlation
  playbook for finding bugs in the SUT.
- `references/discover-ffi-symbols.md` — how to learn an unfamiliar
  dylib's surface.
- `references/discover-socket-protocol.md` — how to learn an unfamiliar
  UDS endpoint's protocol.
- `references/framing-modes.md` — be32 / be64 / varint / none.
- `references/ndjson-vocab.md` — every input op and every output kind.
- `references/safety-notes.md` — never-unload, ABI over-allocation,
  shutdown grace, sentinel slots.
