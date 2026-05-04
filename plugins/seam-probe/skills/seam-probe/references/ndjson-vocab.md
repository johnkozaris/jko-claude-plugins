# NDJSON I/O contract

Every line on stdout is one JSON object terminated by `\n`. Every line
on stdin must be one JSON object terminated by `\n`. Empty lines are
ignored. Timestamps are RFC 3339 UTC with nanosecond precision.

## Output kinds

### `control`
Operational messages.
```json
{"ts":"…","kind":"control","msg":"runtime started; ready for stdin"}
```

### `error`
Recoverable error (probe keeps running) or fatal error before exit.
```json
{"ts":"…","kind":"error","msg":"unknown lane","detail":{"lane":"foo"}}
```

### `rc`
Acknowledgement of a stdin command.
- FFI mode: `rc` is whatever the manifest-declared symbol returned (by
  convention `0`=ok, others=error codes).
- Socket mode: `rc=0` on successful write, `rc=-1` on transport error.
```json
{"ts":"…","kind":"rc","op":"send","name":"events","rc":0}
{"ts":"…","kind":"rc","op":"send","rc":0,"bytes_sent":214}
```

### `event` (FFI, `kind:"json"`)
```json
{"ts":"…","kind":"event","callback":"on_event","payload":{…}}
```

### `json_with_sid` (FFI, `kind:"json_with_sid"`)
```json
{"ts":"…","kind":"json_with_sid","callback":"on_control","session_id":"sess-1","payload":{…}}
```

### `raw_with_seq` (FFI, `kind:"raw_with_seq"`)
Bytes are hex-encoded so they're greppable.
```json
{"ts":"…","kind":"raw_with_seq","callback":"on_data","session_id":"sess-1","seq":42,"len":4,"hex":"1b5b314d"}
```

### `frame` (socket)
Inbound frame from the socket. `payload` is JSON-decoded when parsing
succeeds; `hex` is always populated.
```json
{"ts":"…","kind":"frame","bytes":312,"payload":{…},"hex":"7b22…"}
```

### `symbol` / `summary` (inspect)
```json
{"ts":"…","kind":"symbol","name":"foo_start","sym_kind":"function","address":12345,"size":256}
{"ts":"…","kind":"summary","format":"mach-o","total":42,"functions":40,"data":2}
```

## Input ops (one per stdin line)

### FFI mode

```json
{"op":"send","lane":"<manifest lane name>","payload":{…}}
{"op":"call","name":"<manifest op name>","arg":"optional cstring"}
{"op":"sleep_ms","ms":250}
{"op":"stop"}
```

`send` lanes match a `lanes[].name` in the manifest. `call` ops match
an `ops[].name` (the field is `name`, not `op` — the outer `op`
discriminator already names the operation kind). `arg` is required for
`kind:"handle_cstr"` ops, ignored for `kind:"handle_only"`.

### Socket mode

```json
{"op":"send","payload":{…}}
{"op":"send","payload_hex":"deadbeef"}
{"op":"raw","hex":"deadbeef"}
{"op":"sleep_ms","ms":250}
{"op":"stop"}
```

`payload` is JSON-serialised and sent as one frame. `payload_hex` lets
you send arbitrary bytes for binary fuzzing.

## Lifecycle

- `stop` on stdin → call lifecycle stop, wait `--shutdown-grace-ms`, exit 0.
- EOF on stdin → same as `stop`.
- SIGINT → same as `stop`, with a `cancelled (SIGINT)` control line.
