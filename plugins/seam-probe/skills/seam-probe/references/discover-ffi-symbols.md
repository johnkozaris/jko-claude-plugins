# Discovering an unfamiliar FFI surface

The probe makes you write a manifest. The manifest describes **shape**:
which symbols start/stop the runtime, the callback struct's field
order, and the signatures of each lane and op. You discover this from
the target app's source — the probe will not.

## 1. List exported symbols

```bash
seam-probe inspect --lib path/to/libfoo.dylib
```

This emits one NDJSON line per global function (and data) symbol, plus
a summary. Filter to function symbols:

```bash
seam-probe inspect --lib libfoo.dylib | \
  jq 'select(.kind=="symbol" and .sym_kind=="function") | .name'
```

You are looking for:
- a single "start" symbol returning an opaque handle;
- a single "stop" symbol consuming that handle;
- one symbol per JSON command lane;
- ad-hoc ops that take a session id or a flag.

Most projects follow a `<prefix>_<verb>_<noun>` convention. If the
prefix isn't obvious, ask the user.

## 2. Read the C header

If the app uses cbindgen, look for a generated header in
`target/include/`, `build/include/`, or alongside the dylib. The
header is the **single source of truth** for the callback struct's
field order and per-field signature.

If there is no generated header, look for hand-written `*.h` files in
the repo. Worst case: read `extern "C"` declarations in the source.

## 3. Classify each callback field

For every field of the callback struct, classify its signature into
one of the three probe kinds (see `manifest-schema.md`). The most
common patterns:

| Header signature                                                                                | Manifest `kind`     |
| ----------------------------------------------------------------------------------------------- | ------------------- |
| `void(*)(const uint8_t*, size_t, void*)`                                                        | `json`              |
| `void(*)(const char*, const uint8_t*, size_t, void*)`                                           | `json_with_sid`     |
| `void(*)(const char*, uint64_t, const uint8_t*, size_t, void*)`                                 | `raw_with_seq`      |

If the runtime exposes a callback shape outside these three, **stop**.
The probe cannot bind a trampoline of the wrong arity safely. Add a
new kind in `crate/src/ffi/trampolines.rs` and rebuild.

## 4. Cross-check

Build a manifest. Run `seam-probe ffi` against the dylib. If the first
control line emits `"runtime started"` you've cleared the start
boundary. If you see an `error` line about a missing symbol, check
spelling and exported visibility (`#[no_mangle]`, `#[unsafe(no_mangle)]`,
custom `*_API`-style export macros some projects use).

If the runtime invokes a callback you didn't declare, the probe
**aborts** with a loud message — that means your `callback_struct[]`
under-declares the runtime's actual struct. Add the missing field.

## 5. Common gotchas

- **Mangled symbol names** — Rust without `#[no_mangle]` will mangle. The
  app's symbols should be unmangled if they're meant to be called from
  C/Swift/JS.
- **Static vs dynamic** — `seam-probe ffi` only takes dynamic libraries
  (`.dylib`, `.so`, `.dll`). For staticlibs, link them into a small
  cdylib first.
- **Calling convention** — the probe uses the platform default (cdecl on
  x86-64, AAPCS on ARM64). stdcall/fastcall require code changes.
- **Variadics & struct-by-value args** — explicitly out of scope. Reject
  manifests that need them.
