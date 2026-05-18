# FFI and Foreign-Language Interop

> What to use when Rust talks to (or is called by) C, C++, Swift/Kotlin, Python, Node.js, Ruby, Lua, V8, or WebAssembly. Plus the Edition 2024 unsafe-attribute syntax mandatory for FFI declarations.

The general shape: Rust at the boundary speaks `extern "C"`. Above that, language-specific bridge crates generate idiomatic bindings. Below it, `cdylib` or `staticlib` produces the artifact other languages can consume.

---

## Edition 2024 mandatory syntax

If your code targets Rust 1.85+ on Edition 2024, FFI declarations have new required spelling. (The bare forms compile on Edition 2021 but are deny-by-default in 2024 — `cargo fix --edition` will migrate them.)

| Before | After (Edition 2024) |
|---|---|
| `#[no_mangle]` | `#[unsafe(no_mangle)]` |
| `#[export_name = "..."]` | `#[unsafe(export_name = "...")]` |
| `#[link_section = "..."]` | `#[unsafe(link_section = "...")]` |
| `extern "C" { fn foo(); }` | `unsafe extern "C" { fn foo(); }` |

Inside an `unsafe extern "C"` block, individual items can be marked `pub safe fn` if they're genuinely safe to call from safe Rust:

```rust
unsafe extern "C" {
    pub safe fn sqrt(x: f64) -> f64;             // safe — callable from safe code
    pub fn read(fd: i32, buf: *mut u8, count: usize) -> isize;  // unsafe by default
}
```

Strict provenance is also stable since 1.84 — use `addr`, `with_addr`, `expose_provenance` instead of `as usize` round-trips for pointer math. Miri and CHERI hardware track provenance and reject the casts.

---

## Crate types

When you build a Rust library that's consumed by another language, you pick a `crate-type` in `Cargo.toml`:

| Crate type | Output | Use when |
|---|---|---|
| `rlib` (default for libraries) | Rust-internal archive | Default. Rust→Rust consumption only. |
| `staticlib` | `.a` / `.lib` (C-style static archive) | Embedding Rust into a non-Rust app, statically. Bundles all Rust deps; no dynamic Rust ABI. |
| `cdylib` | `.so` / `.dylib` / `.dll` / `.wasm` | C-callable shared library. Only `extern "C"` items are exported; everything else is dead-code-stripped. **Required** for WebAssembly, PyO3, napi-rs, JNI, and most plugin systems. |
| `dylib` | `.so` / `.dylib` / `.dll` (Rust ABI) | Rust-to-Rust dynamic linking with the **same compiler version**. Niche. |
| `bin` | Executable | Binaries. |
| `proc-macro` | Compiler plugin | Procedural macros. |

You can list multiple types in one crate:

```toml
[lib]
crate-type = ["staticlib", "cdylib"]
```

This builds both flavors from one source. Common for libraries that ship in both static (for binaries) and dynamic (for late-loading plugins) modes.

Practical gotcha: a `cdylib` minimum size is ~2.2 MB even if empty — the Rust panic/alloc machinery comes along. Strip with `panic = "abort"` and `lto = "fat"` in the release profile, plus `strip = "symbols"`.

---

## C interop

**Two crates do the heavy lifting**:

- **`bindgen`** — reads C headers, generates Rust `unsafe extern` declarations. MSRV 1.70+. Use `--allowlist-function` / `--allowlist-type` to keep generated code small. For C++ headers, allowlist aggressively and mark `std::*` opaque. Run in CI and commit the output so drift is detected.
- **`cbindgen`** — reads Rust `extern "C"` items, generates a C header. Pair with `#[unsafe(no_mangle)]` on every exported function. Stable types only.

Canonical layering:

- `foo-sys` — raw bindgen output, all `unsafe`. Mostly mechanical.
- `foo` — safe wrapper with `Drop`, RAII handles, idiomatic error types.

**Safety rules** for code that crosses the boundary:

- `extern "C"` + `#[repr(C)]` on every type/function that crosses.
- `Option<&T>`, `Option<&mut T>`, `Option<NonNull<T>>`, `Option<extern "C" fn()>` are FFI-safe and same-layout as the inner pointer (null-pointer optimization). Use these instead of raw `*mut T` for nullable parameters.
- Wrap every exported function body in `std::panic::catch_unwind` — letting a Rust panic unwind into C is UB. Convert panics to error codes.
- Use `extern "C-unwind"` only if panics may legitimately cross and the caller is prepared.
- **Never** expose `dyn Trait` or generics across FFI — neither has a stable ABI. Use `#[repr(C)]` vtable structs with function pointers, or opaque handles (`*mut MyOpaque`) with C-ABI functions operating on them.
- `#[repr(transparent)]` for newtype wrappers around an FFI-compatible inner type.
- `#[repr(C, packed)]` for wire/file formats (mind the alignment hazards — taking references to fields is UB).

---

## C++

`cxx` (dtolnay) is the modern Rust↔C++ bridge. ~60M downloads, used in Chromium, Android, AWS code.

It's schema-driven: you declare a `#[cxx::bridge]` module describing shared types and functions; `cxx` generates the safe wrappers and validates type compatibility. Supports first-class translations:
- Rust `String` ↔ C++ `std::string`
- Rust `Box<T>` ↔ C++ `std::unique_ptr<T>`
- Rust `Vec<T>` ↔ C++ `std::vector<T>`
- Rust `&str` ↔ C++ `rust::Str`
- Rust `Result<T, E>` ↔ C++ exception

**Prefer `cxx` over `bindgen` for C++.** Bindgen works for C++ but lacks the type-safety guarantees `cxx` provides.

**`autocxx`** combines `cxx` with bindgen-style auto-generation. Driven mainly by Google for Chrome.

---

## Swift / iOS / Apple

**UniFFI** (Mozilla, `uniffi-rs`) is the de facto choice. Used in production by Firefox, 1Password, Signal-style apps.

Build artifact path: Rust → `staticlib` + `cdylib` → XCFramework (for multi-arch iOS device + simulator + macOS).

Tooling:
- `cargo swift` — cargo plugin that builds a Swift Package directly
- `uniffi-starter` — template for a Rust core + Swift package + Gradle module
- `cargo-ndk`-style helpers for cross-compilation

UniFFI generates a Swift module that imports a C ABI module map. Rust `enum` becomes Swift `enum`, Rust trait becomes Swift `protocol` + concrete class, Rust `dictionary` becomes Swift `struct`. `snake_case` auto-converts to `camelCase`. Result types map to Swift `throws`.

Status caveat: UniFFI is pre-1.0 ("ready for production but not API-stable"). Swift 6 strict concurrency support is partial; expect to add `@Sendable` annotations manually in some cases.

Alternative: **`swift-bridge`** (chinedufn/swift-bridge) — declarative bridge module syntax similar to `cxx`. Smaller ecosystem; better for tight coupling. Lags UniFFI in adoption.

Apple's official Swift/C++ interop (Swift 5.9+) doesn't help with Rust directly — you still need a C ABI seam.

---

## Kotlin / Android

**UniFFI for Kotlin** is the high-level path. Same `.udl` or proc-macro definition feeds Kotlin (Android) and Swift (iOS) from one Rust source.

Build chain: Cargo + `cargo-ndk` Gradle plugin + AGP (Android Gradle Plugin).

Historically UniFFI used **JNA** for the JVM side. The 2025-2026 migration to direct JNI is in progress (issue #2672) — JNA's per-call reflection is painful in hot paths.

Limitations to know:
- UniFFI cannot pass `jobject` handles through its bindings layer. For passing Android/iOS native objects (an `Activity`, a `Surface`), drop to JNI or Objective-C-Interop with an opaque handle on each side.

**Direct JNI via `jni-rs` crate** (~v0.21):
- For zero abstraction or interacting with platform objects UniFFI can't pass
- Function symbol naming: `Java_com_pkg_Class_method`; in Edition 2024 that's `#[unsafe(export_name = "Java_...")]`
- Threading: call `AttachCurrentThread` once when spawning Rust threads that call back. Otherwise every call attaches/detaches — expensive.
- Library is `cdylib`, linked from the AAR.

**Kotlin Multiplatform**: `Gobley` (or "KMP UniFFI") generates KMP bindings — same Rust core targets both JVM (Android) and Kotlin/Native (iOS via Kotlin). Relevant when the team is already invested in KMP.

---

## Python

**PyO3 + maturin** is the standard.

Current: **PyO3 0.28+**, **maturin 1.13+**.

Build with `maturin develop`, `maturin build`, or `uv publish`. Cargo: `[lib] crate-type = ["cdylib"]`. Then `#[pyclass]` / `#[pyfunction]` / `#[pymodule]` macros, `pyo3::prelude::*`.

**Stable-ABI wheels (abi3)**: feature flags `abi3-py38`, `abi3-py39`, … `abi3-py313`. Pick the lowest Python version you support; the wheel works on that and every newer 3.x. One wheel covers many Python versions.

`manylinux_2_17` (aka `manylinux2014`) is the minimum since rustc 1.64 requires glibc 2.17.

**2026 PyO3 changes**:
- The `extension-module` feature flag is retired in favor of `PYO3_BUILD_EXTENSION_MODULE` env var (set automatically by maturin ≥1.9.4 and setuptools-rust ≥1.12). This fixed linking for `cargo test`/`cargo bench`.
- `PYO3_NO_PYTHON` builds abi3 wheels without a host Python (useful in cross-builds and CI).
- `generate-import-lib` (experimental) creates `python3.lib` on Windows when the import library isn't available; needs LLVM's `llvm-dlltool` in PATH.

---

## Node.js

**`napi-rs`** is the dominant 2026 choice. SWC, Next.js, Prisma, Rspack all use it. SWC migrated *away from* Neon — see [Issue #852](https://github.com/swc-project/swc/issues/852).

Why napi-rs won:
- `#[napi]` proc macro hides the Node-API juggling. Define an `async fn` in Rust → it appears as an async function in JS.
- Auto-generates TypeScript `.d.ts` and a small JS loader (no more `@node-rs/helper`).
- Cross-compilation matrix is mature: ships prebuilt binaries to npm via `napi-rs/cli`.
- Built on the **Node-API (N-API) stable ABI** — one binary works across Node major versions without recompiling.

**Neon** is still maintained, more "type-system pure" Rust API. Use it for simpler use cases or projects already invested.

For Electron: both work; napi-rs's prebuilt distribution pattern is friendlier to electron-builder.

---

## Ruby, Lua, V8/Deno

| Language | Crate | Notes |
|---|---|---|
| Ruby | `magnus` (matsadler/magnus) | Bidirectional Ruby↔Rust. `#[magnus::wrap]` wraps Rust types as Ruby objects. Mutability constraint: Ruby's GC owns storage, so wrapped types use `RefCell` newtypes for interior mutability. |
| Lua | `mlua` (mlua-rs) | Supports Lua 5.5/5.4/5.3/5.2/5.1, LuaJIT, Luau. Async support via Lua coroutines (`feature = "async"`). `serde` feature serializes any `Serialize` type into `mlua::Value`. The old `rlua` crate is a deprecated thin wrapper around `mlua`. A Luau-focused fork **mluau** exists for Roblox-style use. |
| V8 / Deno | `rusty_v8` (denoland/rusty_v8) | Stable. Versioned with Chrome. Zero-overhead C++→Rust binding to V8. `deno_core` builds on it: event loop, ops macros (JS Promises ↔ Rust Futures), V8 Fast API path. Used for sandboxed JS, edge runtimes, custom runtimes. Note: raw `rusty_v8` has *no Web APIs* — no `fetch`, no `crypto`, no `TextEncoder` — those come from `deno_runtime`'s extension crates. |

---

## WebAssembly

Target landscape in 2026:

| Target | Use |
|---|---|
| `wasm32-unknown-unknown` | Browser/JS via `wasm-bindgen`. Still the dominant in-browser target. |
| `wasm32-wasip1` (formerly `wasm32-wasi`) | Legacy WASI Preview 1. |
| `wasm32-wasip2` | **WASI Preview 2 + Component Model. STABLE since Rust 1.82.** Native Cargo produces components without `cargo-component` for WASI-only workloads. |
| `wasm32-wasip3` | EXPERIMENTAL. Preview 3 brings native async (a WASI TCP read can `await` without blocking the instance). |

### Browser

**`wasm-bindgen`** is the canonical Rust↔JS interop crate. Version pin: the crate version must match the CLI version exactly. Use `wasm-pack` for build orchestration.

The `rustwasm` GitHub org was sunset in July 2025; tools moved to independent orgs but are still maintained.

### Server / Components (Bytecode Alliance ecosystem)

**Different layer from `wasm-bindgen`**:

- **`wit-bindgen`** — generates guest bindings from `.wit` interface files. Languages: Rust, C, C++, C#.
- **`cargo-component`** — Cargo plugin for components with dependencies. Generates `src/bindings.rs` from resolved WIT deps in `Cargo.toml`.
- **`wasmtime::component::bindgen!`** — host-side macro that emits trait-based bindings for a runtime to implement.
- **`wasm-tools`** — inspect/compose components (`wasm-tools component wit`, `wasm-tools compose`).

Production users in 2026: Figma, 1Password, Shopify ship Rust WASM components. Zed extensions and Zellij plugins both use WASM Components for sandboxed third-party extension.

Quick decision:
- WASI-only modern Rust → `cargo build --target wasm32-wasip2`
- Custom WIT or component dependencies → `cargo-component`
- Browser → `wasm-bindgen` + `wasm-pack`

---

## Unsafe Rust patterns at FFI boundaries

A few patterns come up repeatedly in FFI code:

- **`UnsafeCell<T>`** — the only legal way to mutate behind `&T`. All interior mutability primitives wrap it. Note: it disables niche optimizations.
- **`MaybeUninit<T>`** — sound replacement for the deprecated `mem::uninitialized()`. Holds potentially-uninitialized data without auto-dropping. Patterns: out-pointers (`fn fill(out: &mut MaybeUninit<T>)`), partially-initialized arrays. Reading or referencing the inside before initialization is UB.
- **`NonNull<T>`** — non-null raw pointer carrying the niche. `NonNull::new(ptr)` to construct from maybe-null `*mut T`; `NonNull::dangling()` for ZST/placeholder. `Option<NonNull<T>>` is the same size as `*mut T` and FFI-safe.
- **Raw pointer access** — `&raw const place` / `&raw mut place` (stable since 1.82) avoid creating an intermediate reference, sidestepping the strong aliasing assertions a reference would impose. Prefer these in new code over `ptr::addr_of!` / `addr_of_mut!` macros.

**Strict provenance** (stable since 1.84) — methods on pointers and `std::ptr`:
- `<*T>::addr() -> usize` — extract the integer address *without* exposing provenance.
- `<*T>::with_addr(self, usize) -> Self` — clone provenance, give it a new address. The cornerstone API.
- `<*T>::map_addr(self, FnOnce(usize) -> usize) -> Self` — wrapper.
- `<*T>::expose_provenance() -> usize` — like `addr` but also adds the provenance to a global "exposed" pool.
- `std::ptr::{dangling, dangling_mut, without_provenance, with_exposed_provenance}`.

Use strict provenance for tagged pointers, aligned allocators, and any code that previously did `as usize` round-trips. Tooling: Miri tracks provenance; CHERI hardware maps directly to it.

---

## Type punning between byte representations

Use `bytemuck` or `zerocopy` instead of `mem::transmute`. Both rely on safe-marker traits the user attests to; both ship derive macros that verify the invariants at compile time.

- **`bytemuck`** — minimal, no_std-friendly, "plain old data" model. Traits: `Pod`, `Zeroable`, `NoUninit`, `AnyBitPattern`. Functions: `cast`, `cast_slice`, `pod_read_unaligned`, `bytes_of`. Fast to compile.
- **`zerocopy`** (Google, formally verified with Kani) — richer trait taxonomy: `IntoBytes`, `FromBytes`, `FromZeros`, `TryFromBytes`, `KnownLayout`, `Immutable`, `Unaligned`. Macros: `transmute!`, `transmute_ref!`, `transmute_mut!`, plus `try_*` variants. Compile-time size/alignment checks make unconditional casts zero-cost at runtime. Foundation of Fuchsia's Rust networking stack.

When to pick which:
- `bytemuck` for tight no_std budgets and POD-only needs.
- `zerocopy` when you need `TryFromBytes` (runtime-validated structural conformance) or `Unaligned`, for network protocols, or for formal-verification-grade rigor.

---

## Self-referential structs

First rule: prefer redesigning the data layout. Self-referential structs are a code smell unless you genuinely need them (FFI handles that own buffers, single-allocation parse trees, intrusive lists).

If you must:

- **`self_cell`** — minimal (<300 lines), no proc-macro, `no_std`, Miri-tested. **Modern default**.
- **`ouroboros`** — proc-macro driven, richer API. Slower compiles.
- **`yoke`** (ICU4X / Unicode) — pairs a "cart" (owner) with a "yoke" (zero-copy view). Idiomatic for serialized-data-backed views, e.g., mmap-backed parse trees.

All three internally use `unsafe` correctly so you don't have to. None replaces `Pin` projection inside futures — that's a different problem (see [async reference](async.md)).

---

## Pinning (when implementing FFI types that contain futures)

`Pin<P>` exists for *address-stable* values: self-referential structs and most async futures.

For FFI types that need pinned fields, use macro-driven projection:

- **`pin-project-lite`** (declarative macro, no proc-macro deps) — preferred in `no_std`, kernel, embedded.
- **`pin-project`** (proc-macro) — full features: `#[pinned_drop]`, `#[project = Foo]` for enums, `UnsafeUnpin`. Heavier compile.

Usage: `#[pin]` on structurally-pinned fields, then `self.project()` inside `Pin<&mut Self>` methods to get `Pin<&mut T>` for pinned fields and `&mut T` for unpinned ones.

Drop runs as `&mut Self`, not `Pin<&mut Self>` — the macros generate a `Drop` impl that delegates to `#[pinned_drop]` so the invariant is preserved.

---

## Inline assembly

- **`asm!`** macro — stable since Rust 1.59. Function-scope inline asm with input/output operands.
- **`global_asm!`** — stable. Module-scope assembly.
- **`#[unsafe(naked)]` + `naked_asm!`** — stable since Rust 1.88 (June 2025; the stabilization blog post followed in July). The function body must be a single `naked_asm!` invocation. Use case: ABI shims, syscall trampolines, kernel/firmware entry points.

Syntax: LLVM's internal assembler dialect. x86 defaults to `.intel_syntax noprefix`; ARM uses `.syntax unified`.

`feature(cfg_asm)` (annotating individual asm lines with `#[cfg(...)]`) — still nightly in 2026.

---

## Cheat sheet

| You're connecting Rust to... | Use |
|---|---|
| C | `bindgen` (consume) / `cbindgen` (expose) |
| C++ | `cxx` (modern) or `autocxx` (auto-gen for large headers) |
| Swift / Kotlin (mobile) | `UniFFI` |
| Python | `PyO3 0.28+` + `maturin 1.13+`, abi3 wheels |
| Node.js | `napi-rs` |
| Ruby | `magnus` |
| Lua | `mlua` |
| V8 / sandboxed JS | `rusty_v8` + `deno_core` |
| Browser WebAssembly | `wasm-bindgen` + `wasm-pack` |
| Server WebAssembly / Components | `wasm32-wasip2` + `wit-bindgen` + `cargo-component` |

---

## Sources

- [Edition 2024 unsafe attributes](https://doc.rust-lang.org/edition-guide/rust-2024/unsafe-attributes.html)
- [The Rustonomicon — FFI](https://doc.rust-lang.org/nomicon/ffi.html)
- [bindgen User Guide](https://rust-lang.github.io/rust-bindgen/)
- [cxx.rs](https://cxx.rs/)
- [UniFFI book](https://mozilla.github.io/uniffi-rs/)
- [PyO3 user guide](https://pyo3.rs)
- [napi.rs docs](https://napi.rs/)
- [WASI Preview 2 / Component Model](https://component-model.bytecodealliance.org/)
- [wit-bindgen](https://github.com/bytecodealliance/wit-bindgen)
- [Effective Rust Item 34 — FFI](https://www.effective-rust.com/ffi.html)
- [Effective Rust Item 35 — Prefer bindgen](https://effective-rust.com/bindgen.html)
- [Strict provenance APIs (PR #130350)](https://github.com/rust-lang/rust/pull/130350)
