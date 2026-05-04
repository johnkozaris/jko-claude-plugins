# Safety notes

The probe loads arbitrary native code into its own process and calls
into it. Three rules matter.

## 1. The probe never calls `dlclose`

The `Library` returned by `libloading::Library::new` is leaked into a
`&'static Library`. We do not unload it.

Why: most embedded runtimes spawn worker threads (Tokio, libuv, GCD,
…). Those threads may be holding pointers into the library's `.text`
segment (function pointers from callbacks they registered, return
addresses on their stack). If we `dlclose` while any of those threads
is still mid-call, the unmap silently invalidates pages they're
executing from — segfault.

The fix is "don't unload". Process exit reclaims everything cleanly.

## 2. Over-allocated callback struct + sentinel

The probe always passes a 64-pointer array to the runtime's `start`
function, regardless of how many fields the manifest declares.

The C ABI rules for structs > 16 bytes (System V x86-64, Microsoft
x64, AAPCS64) require the caller to allocate the struct and pass a
hidden pointer to the callee. The callee reads only the bytes it
expects. Passing a larger struct than the callee declared is harmless
**provided the callee doesn't read past its own declaration** — which
no correctly-written runtime ever does.

But: if the runtime declared more fields than your manifest, the
runtime will read past your declared fields into the over-allocated
slots. Those slots are bound to a single sentinel function:

```rust
unsafe extern "C" fn unused_callback_slot() {
    eprintln!("seam-probe: runtime called callback slot beyond manifest …");
    std::process::abort();
}
```

If the runtime ever calls into one of those slots, we abort with a
loud message. **Never silently corrupt memory.** The fix is to add the
missing field to the manifest's `callback_struct[]` (in the correct
position).

## 3. Shutdown grace period

`stop` (or stdin EOF) triggers:

1. Call the lifecycle stop symbol.
2. Sleep `--shutdown-grace-ms` (default 2000 ms).
3. `process::exit(0)`.

The grace period exists to give in-flight callbacks time to drain.
Real-world runtimes can have shutdown bugs (e.g. a stop symbol that
hangs forever) — exiting after the grace window sidesteps them.
Bumping `--shutdown-grace-ms` to 5000 ms or more is fine; setting it to
0 makes shutdown instant but risks racing pending callbacks.

## 4. Restricted to `extern "C"` (cdecl on Unix, cdecl on Windows)

The probe's trampolines are declared `unsafe extern "C" fn`. Anything
else (variadic, `extern "stdcall"`, `extern "fastcall"`,
struct-by-value arguments) requires changes to
`crates/seam-probe/src/ffi/trampolines.rs` and a recompile.

## 5. Bounded frame sizes

Both the FFI command JSON and the UDS framed reads are bounded at
**8 MiB**. The probe rejects larger payloads to keep accidental fuzz
runs from OOM-ing the host.
