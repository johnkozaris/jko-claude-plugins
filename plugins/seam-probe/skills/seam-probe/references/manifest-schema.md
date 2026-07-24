# Manifest schema (v2)

The manifest is a JSON document that tells `seam-probe` how to drive an
unfamiliar shared library. It describes the **shape** of exported
symbols, not their semantics.

## Top-level shape

```json
{
  "schema_version": 2,
  "label": "optional human-readable label echoed in control output",
  "lifecycle": {
    "start_symbol": "<name of (callbacks, user) -> handle>",
    "stop_symbol":  "<name of (handle) -> void>"
  },
  "callback_struct": [ … ],
  "lanes":           [ … ],
  "ops":             [ … ]
}
```

- `schema_version` MUST be `2`.
- `label` is cosmetic.
- `callback_struct[]`, `lanes[]`, `ops[]` may all be empty if the
  surface doesn't need them.

## `lifecycle`

Two symbols describing the runtime's start/stop pair.

The probe assumes the **start** signature is, in C terms:

```c
void* start(const struct callbacks_t* cb, void* user);
```

…and **stop** is:

```c
void  stop(void* handle);
```

If your app's surface differs, the probe cannot drive it without code
changes.

Schema v1's by-value callback ABI was removed because a runtime-sized
struct cannot be called soundly through one fixed function signature.
The loader rejects v1 manifests before opening the library. Migrate by
changing the exported start function to take `const callbacks_t*` and
set `schema_version` to `2`.

## `callback_struct[]`

An ordered list of fields in the runtime's callback struct. **Field
order MUST match the C struct's declaration order byte-for-byte** —
the probe passes a process-lifetime pointer to an over-allocated
64-pointer table, and the runtime reads from index 0 forward according
to its own declaration. A
mismatched order means the runtime invokes the wrong trampoline, with
the wrong signature: undefined behaviour.

Each field has a `name` (cosmetic; used to label emitted events) and a
`kind` (signature shape). Three kinds are supported:

| `kind`            | C signature                                                                | Used for                                                |
| ----------------- | -------------------------------------------------------------------------- | ------------------------------------------------------- |
| `json`            | `void cb(const uint8_t* json, size_t len, void* user)`                     | Most lanes; JSON-encoded events                         |
| `json_with_sid`   | `void cb(const char* sid, const uint8_t* json, size_t len, void* user)`    | Control frames scoped to a session id                   |
| `raw_with_seq`    | `void cb(const char* sid, uint64_t seq, const uint8_t* bytes, size_t len, void* user)` | Hot byte-stream lanes (terminal output, audio, …)       |

If the runtime uses a fourth signature, the probe needs a code change.

## `lanes[]`

Symbols shaped `(handle, *const uint8_t json, size_t len) -> int32_t`.
This is the dominant "send a JSON command into the runtime" idiom.

```json
{ "name": "<lane name>", "symbol": "<exported symbol>" }
```

Lane name is the value you put in `{"op":"send","lane":"…"}` on stdin.

## `ops[]`

Catch-all for symbols that don't fit the lane shape. Two `kind`s:

| `kind`        | C signature                            | Used for                       |
| ------------- | -------------------------------------- | ------------------------------ |
| `handle_cstr` | `int32_t op(void* handle, const char* arg)` | session-id-scoped operations  |
| `handle_only` | `int32_t op(void* handle)`             | parameterless toggles          |

Op name is the value you put in `{"op":"call","name":"…","arg":"…"}` on stdin.

## Limits

- `MAX_CALLBACK_FIELDS = 64` — raise the constant in
  `crate/src/manifest.rs` and rebuild if you need more.
- Command JSON is bounded at 8 MiB on the way in.
