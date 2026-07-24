# Observability: probe NDJSON + SUT logs

The probe answers "what came back across the seam". The SUT's stderr
or log file answers "why". Most bugs need both. This doc collects the
patterns that worked in practice.

## Capturing both streams

### FFI: the SUT runs in the probe's process

Anything `eprintln!`/`tracing::error!`/`log::error!` etc. inside the
loaded library lands on the probe's stderr (fd 2). Send NDJSON to one
file and stderr to another:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 \
  seam-probe ffi --lib ./libfoo.dylib --manifest M.json \
  > probe.ndjson 2> sut.stderr < input.ndjson
echo "exit=$?"
```

Notes:

- `RUST_LOG` works for any runtime using `log`/`tracing`. For other
  languages, use whatever logging level/destination knob the runtime
  exposes.
- `RUST_BACKTRACE=1` (or `full`) prints a stack on panic. The probe
  process aborts on panic, but the backtrace still flushes to fd 2.
- **Always check the exit code.** Non-zero on panic, abort, or signal.

### Socket: the SUT is a separate process

Three things needed:

1. The SUT started with logs going somewhere readable.
2. The socket path it binds.
3. A way to stop it cleanly when you're done.

If the user lets you run the SUT yourself:

```bash
SUT_BIN=./target/debug/sut
SOCK=/tmp/foo.sock
LOG=/tmp/sut.log

rm -f $SOCK
RUST_LOG=debug $SUT_BIN > $LOG 2>&1 &
SUT_PID=$!

# Wait up to 2.5s for the socket to appear.
for _ in $(seq 1 50); do [ -S $SOCK ] && break; sleep 0.05; done
[ -S $SOCK ] || { echo "SUT failed to bind $SOCK"; cat $LOG; kill $SUT_PID 2>/dev/null; exit 1; }

seam-probe socket --path $SOCK --framing be32 \
  > probe.ndjson < input.ndjson
PROBE_RC=$?

kill -TERM $SUT_PID 2>/dev/null
wait $SUT_PID 2>/dev/null

echo "probe=$PROBE_RC"
echo "--- probe ---"; cat probe.ndjson
echo "--- sut  ---"; cat $LOG
```

If the user already has the SUT running, ask:

1. The socket path.
2. Where logs go (file? stderr to a tmux pane? `journalctl -u
   foo.service`? `docker logs <id>`? a Sentry/Datadog stream?).

Then read the logs wherever they live. Don't restart the SUT without
permission — restarts can destroy ephemeral state the user is trying
to reproduce a bug from.

## Filtering NDJSON

Every output line is one JSON object, so `jq` is the natural tool. If
`jq` is missing, `grep '"kind":"…"'` works for the common cases.

```bash
jq -c 'select(.kind=="error")' probe.ndjson                  # probe errors
jq -c 'select(.kind=="rc" and .rc!=0)' probe.ndjson          # failed ops
jq -c 'select(.kind=="event")' probe.ndjson                  # all callbacks
jq -c 'select(.kind=="event" and .callback=="on_x")' probe.ndjson
jq -r '.ts + " " + .kind + " " + (.msg // "")' probe.ndjson  # timeline view
jq -c 'select(.kind=="frame") | .payload' probe.ndjson       # socket payloads
```

## Time-correlating the two streams

The probe timestamps every line with RFC 3339 UTC. Most runtimes
default to ISO 8601 / RFC 3339 too, so a chronological merge works:

```bash
{ sed 's/^/[probe] /' probe.ndjson;
  sed 's/^/[sut]   /' sut.stderr; } \
| sort -s -k1,1 \
| head -200
```

If the SUT log uses a different timestamp format, prefix every line
with a wall-clock stamp at capture time:

```bash
$SUT_BIN 2>&1 | ts '%Y-%m-%dT%H:%M:%.S' > sut.log    # ts from moreutils
```

Then merge as above.

## Triage by symptom

### A `call` returns non-zero

- **NDJSON**: `{"kind":"rc","op":"call","name":"X","rc":-2}`
- **Means**: the manifest-declared symbol returned non-zero.
- **Look in SUT** for an error log near that timestamp.
- **Fix**: check the runtime's documented return codes. The probe is
  agnostic — it just hands you the integer.

### A `send` produces no event

- **NDJSON**: `{"kind":"rc","op":"send","name":"events","rc":0}` then
  silence.
- **Means**: the runtime accepted the input but never invoked any
  callback the probe is listening on.
- **Look in SUT** for `dropped`, `no handler`, `lane not bound`, or
  silence.
- **Fix candidates**:
  1. Lane name typo. Check `manifest.lanes[].name`.
  2. The callback field is missing from the manifest's
     `callback_struct`, so the runtime is calling into the sentinel
     slot. Usually the runtime crashes before logging — look for
     `Segmentation fault`.
  3. The runtime requires lifecycle setup before lanes accept input.
     Add a `call` op to start a session before the `send`.

### Probe aborts with a Rust panic backtrace

- **NDJSON**: ends abruptly. No `control` line about exit.
- **stderr**: `thread '…' panicked at …` followed by a backtrace
  (with `RUST_BACKTRACE=1`).
- **Means**: bug in the runtime triggered by your input.
- **Fix**: this **is** the find. Capture stdin verbatim as a repro and
  hand both files to the runtime author.

### Probe aborts with no panic

- **stderr**: `Segmentation fault (core dumped)` or `signal: SIGABRT`
  with no panic frame.
- **Means**: ABI mismatch. The probe wrote into a callback slot the
  runtime overwrote, or a `kind:"json"` slot the runtime treats as
  `kind:"raw_with_seq"`.
- **Fix**:
  1. `seam-probe inspect --lib …` and re-read the symbol list.
  2. Compare the manifest's `callback_struct[]` against the C header
     produced by `cbindgen` or hand-maintained alongside the dylib.
  3. Match field order **and** signature kind exactly.

### Runtime stop exceeds the grace deadline

- **NDJSON**: `{"kind":"error","msg":"runtime stop exceeded shutdown grace; forcing process exit",...}`.
- **Means**: the lifecycle stop call did not return before the total
  stop-and-drain deadline.
- **Fix**:
  1. Bump `--shutdown-grace-ms 5000` only to collect more diagnostics.
  2. Fix the runtime shutdown path. SUT stderr will usually show
     per-thread state when the deadline expires.

### Socket: probe hangs forever after the first `send`

- **NDJSON**: one `rc` for the send, then nothing.
- **Means**: framing mismatch.
- **Fix**: try `be32`, `be64`, then `none`. For varint, use raw mode with
  a manual LEB128 prefix. Confirm by reading the SUT's read loop (look
  for `length_field_type`, `read_u32`, `read_u64`, or no length at all).

### Socket: connection fails

- **NDJSON**: `{"kind":"error","msg":"connect failed: …"}`
- **Fix**: `ls -l $SOCK` and check the SUT log for a successful bind
  line. SUT may not be running, the path may be wrong, or you lack
  permission.

### Socket: `connection reset` mid-stream

- **NDJSON**: events flowing, then suddenly
  `{"kind":"error","msg":"frame read error: …"}` in framed mode or
  `{"kind":"error","msg":"read error: …"}` in raw mode.
- **Means**: SUT crashed or closed the socket.
- **Look in SUT**: a panic, signal, or deliberate close near the same
  timestamp.
- **Fix**: minimise the input until the failure stops, then you have
  a repro.

### Socket: framed read is too large

- **NDJSON**: `{"kind":"error","msg":"frame read error: …"}` with a
  codec message indicating the frame exceeded the configured maximum.
- **Means**: framing mismatch the other way — SUT wrote a frame
  whose length prefix decodes to > 8 MiB. Probe caps at 8 MiB to
  avoid OOM on misframed input.
- **Fix**: try a different `--framing` mode. If the SUT genuinely
  writes >8MiB frames, the probe needs a code change to lift the cap.

## Live debugging (interactive)

For interactive sessions it's often nice to see both streams live.

### Two terminals

```bash
# T1: probe with stderr captured to a file
seam-probe ffi --lib X --manifest M.json \
  > probe.ndjson 2> sut.stderr < input.ndjson

# T2 (concurrent): tail the SUT stderr and grep for trouble
tail -F sut.stderr | grep -iE 'panic|error|warn'
```

### Single terminal with annotated lines

```bash
seam-probe ffi --lib X --manifest M.json \
  > >(sed 's/^/[probe] /') \
  2> >(sed 's/^/[sut]   /' >&2) \
  < input.ndjson
```

Always also capture to files so you can grep after.

## When the probe is the right tool, but you can't capture SUT logs

If the runtime is closed-source or logs to somewhere you can't reach:

1. Run with `RUST_BACKTRACE=full` anyway — even closed-source Rust
   binaries usually emit panic backtraces with `panic = "abort"` or
   the default unwinder.
2. Use `dtruss` (macOS), `strace -f` (Linux), or `lldb`/`gdb` to
   attach to the probe process and observe syscalls / signals.
3. If the runtime has a verbose flag (`--verbose`, `--debug`,
   `LOG_LEVEL=trace`), enable it.
4. If still nothing, you can only reason from probe output. Note this
   limitation in any bug report.
