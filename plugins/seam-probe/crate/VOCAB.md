# seam-probe — NDJSON I/O contract

Every line on stdout is one JSON object terminated by `\n`. Every line on
stdin is one JSON object terminated by `\n`. Empty lines are ignored.
Timestamps are RFC 3339 UTC with nanosecond precision (`ts`).

## Output kinds

### `control`
Operational messages. Useful for greppable progress.
```json
{"ts":"…","kind":"control","msg":"runtime started; ready for stdin"}
```

### `error`
Recoverable error (the probe keeps running) or fatal error before exit.
The `detail` object varies by error.
```json
{"ts":"…","kind":"error","msg":"unknown lane","detail":{"lane":"foo"}}
```

### `rc`
Acknowledgement of a stdin command. `rc` is the integer return value of
the underlying call.
- FFI mode: `rc` is whatever the manifest-declared symbol returned
  (typical convention is `0`=ok, non-zero=error code, but the probe
  doesn't interpret it — pass it through).
- Socket mode: `rc=0` on successful write, `rc=-1` on transport error.
```json
{"ts":"…","kind":"rc","op":"send","name":"events","rc":0}
{"ts":"…","kind":"rc","op":"send","rc":0,"bytes_sent":214}
```

### `event` (FFI, json kind)
Emitted when the runtime calls a manifest-declared `kind:"json"`
callback. `callback` is the manifest field name.
```json
{"ts":"…","kind":"event","callback":"on_event","payload":{…}}
```

### `json_with_sid` (FFI, json_with_sid kind)
Emitted for callbacks shaped `(sid, json, len, user)` — control-frame
lanes scoped to a session id (e.g. a per-tab control channel).
```json
{"ts":"…","kind":"json_with_sid","callback":"on_session_control","session_id":"sess-1","payload":{…}}
```

### `raw_with_seq` (FFI, raw_with_seq kind)
Emitted when the runtime calls a `kind:"raw_with_seq"` callback. Bytes
are hex-encoded (Claude can grep known sequences).
```json
{"ts":"…","kind":"raw_with_seq","callback":"on_data","session_id":"sess-1","seq":42,"len":4,"hex":"1b5b314d"}
```

### `frame` (socket)
Inbound frame from the socket. `payload` is the JSON-decoded frame when
parsing succeeds; `hex` is always populated.
```json
{"ts":"…","kind":"frame","bytes":312,"payload":{…},"hex":"7b2274797..."}
```

### `symbol` / `summary` (inspect)
```json
{"ts":"…","kind":"symbol","name":"lib_start","sym_kind":"function","address":12345,"size":256}
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

`send` lanes match a `lanes[].name` in the manifest. `call` ops match a
`ops[].name` in the manifest (note the `name` field — the outer `op`
discriminator already names the operation kind). Missing `arg` defaults
to an empty C string for `kind:"handle_cstr"` and is ignored for
`kind:"handle_only"`.

### Socket mode

```json
{"op":"send","payload":{…}}
{"op":"send","payload_hex":"deadbeef"}
{"op":"raw","hex":"deadbeef"}
{"op":"sleep_ms","ms":250}
{"op":"stop"}
```

`payload` is JSON-serialised and sent as one frame. `payload_hex` lets
you send arbitrary bytes (binary fuzzing).

## Lifecycle

- FFI `stop` / EOF / SIGINT → invoke the manifest lifecycle stop on a
  dedicated thread, enforce the total `--shutdown-grace-ms` deadline,
  then exit.
- Socket `stop` / EOF / SIGINT → shut down the socket write half,
  cancel its reader, and exit. Socket mode has no lifecycle symbol or
  shutdown-grace flag.
- SIGINT also emits the control line `cancelled (SIGINT)`.

## Safety promises

- The probe never calls `dlclose`. Loaded libraries leak intentionally;
  see `skills/seam-probe/references/safety-notes.md` in the plugin for rationale.
- Callback slots not declared in the manifest are bound to a sentinel
  that aborts the process if invoked, rather than returning to
  uninitialised memory.
- All command JSON is bounded at 8 MiB.
