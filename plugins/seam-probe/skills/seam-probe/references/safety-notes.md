# Safety notes

The probe loads arbitrary native code into its own process and calls
into it. Five rules matter.

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

The supported `start` ABI takes `const callbacks_t*`, never a callback
struct by value. The probe allocates a 64-pointer table, passes its
address, and keeps the allocation alive until process exit. The runtime
reads only the prefix described by its own callback struct declaration.

If the runtime declared more fields than the manifest, it reads past
the declared fields into over-allocated slots. Those slots are bound to
a single sentinel function:

```rust
unsafe extern "C" fn unused_callback_slot() {
    eprintln!("seam-probe: runtime called callback slot beyond manifest …");
    std::process::abort();
}
```

This protection applies only when the runtime callback struct contains at most
64 pointer fields. A larger struct can read beyond the allocation and is out of
scope; verify field count before loading.

If the runtime calls one of the protected undeclared slots, the probe aborts
with a loud message. The fix is to add the
missing field to the manifest's `callback_struct[]` (in the correct
position).

## 3. Shutdown grace period

`stop` (or stdin EOF) triggers:

1. Call the lifecycle stop symbol on a dedicated OS thread.
2. Allow at most `--shutdown-grace-ms` total (default 2000 ms) for stop
   to return and callbacks to drain.
3. Exit even if stop is still blocked.

The deadline prevents a buggy stop symbol from hanging the probe while
still giving in-flight callbacks time to drain. Bumping
`--shutdown-grace-ms` to 5000 ms or more is fine; setting it to 0 makes
shutdown immediate but can race pending callbacks.

The grace deadline does not cover lifecycle `start`, lane calls, or manifest
ops. Those calls execute synchronously and may block indefinitely. Run
hang-prone or untrusted inputs under an external process supervisor/timeout.

## 4. Restricted to `extern "C"`

The probe uses the platform's default C calling convention. Anything
else (variadic, `extern "stdcall"`, `extern "fastcall"`,
struct-by-value arguments) requires changes to
`crate/src/ffi/trampolines.rs` and a recompile.

## 5. Bounded frame sizes

FFI and socket stdin lines, outbound payloads, and UDS framed reads are
bounded at **8 MiB** before JSON parsing or network writes. Raw inbound
reads use 8 KiB chunks. The probe rejects larger inputs to keep
accidental fuzz runs from OOM-ing the host.
