# Probe and system-under-test evidence

The probe reports what crossed the seam. The system-under-test logs explain why.
Capture them separately and correlate by timestamp, operation, session, and
sequence.

## Capture discipline

In FFI mode, probe NDJSON and the loaded library share a process. Probe output
uses stdout; diagnostics, sentinel failures, panics, and normal library stderr
share fd 2:

```bash
"$SEAM_PROBE" ffi --lib ./libfoo.dylib --manifest manifest.json \
  >probe.ndjson 2>sut.stderr <input.ndjson
```

If the loaded library writes to stdout, those lines corrupt the NDJSON stream.
Filter valid JSON with `jq -Rc 'fromjson? // empty'` and preserve rejected lines
as system-under-test output.

In socket mode the server is another process. Preserve its own logs and do not
restart it without permission; a restart can destroy the state under
investigation.

## Chronological correlation

When the server log already begins with RFC3339 timestamps:

```bash
{
  jq -r '"\(.ts) [probe] \(. | tostring)"' probe.ndjson
  sed -E 's/^([0-9]{4}-[0-9]{2}-[0-9]{2}T[^ ]*)/\1 [sut]  /' sut.stderr
} | sort -s -k1,1
```

Normalize another log format before merging rather than sorting stream labels.

## Interpret evidence narrowly

- An `rc` records the target-defined FFI return value or local socket write
  result. It does not prove application acceptance.
- A successful write followed by silence may indicate a one-way protocol,
  missing setup, wrong callback manifest, or a target-side failure.
- A panic or signal suggests a runtime bug or ABI mismatch; use stderr, headers,
  source, and a minimized input before choosing one.
- Socket silence or reset may indicate framing, application protocol, target
  shutdown, or transport failure. Inspect the server read loop and logs.
- Shutdown-grace expiry means only that `stop` did not finish inside the grace
  window. Other FFI calls are not covered by that deadline.

Exit status alone never proves success. `seam-probe vocab` prints the current
exit-code and output-kind contract. Inspect `kind:"error"`, stderr, and the
system-under-test postcondition.
