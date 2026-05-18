# 2026 Currency Anchor

> Single file with version anchors for rustc, Tokio, Axum, Edition, and the deprecation list. When new content is added elsewhere referencing versions, point here. Update this file when Rust releases or LTS lines roll.

Last updated: May 2026 (Rust 1.95 stable, 1.96 beta).

---

## Rust toolchain

- **Stable**: 1.95.0 (April 16, 2026). [Release notes](https://blog.rust-lang.org/2026/04/16/Rust-1.95.0/).
- **Beta**: 1.96.0 (stable May 28, 2026).
- **Nightly**: 1.97.x.
- **Current Edition**: 2024 (stable since Rust 1.85, Feb 2025). No 2025 or 2026 edition planned.

## Tokio LTS

- **Current**: 1.5x (latest 1.52.3, released May 8, 2026). The 1.51.0 release introduced LIFO slot stealing (#7431); 1.52.2 reverted it after a measured perf regression. (1.52.1 had reverted a different change — the `spawn_blocking` sharded queue work, which was causing hangs.)
- **LTS 1.47.x**: support until **September 2026**. MSRV 1.70.
- **LTS 1.51.x**: support until **March 2027**. MSRV 1.71.
- **No Tokio 2.0 planned** — the project maintains 1.x backward compat aggressively.

## Axum

- **Current**: 0.8+ (released Jan 2025). Breaking changes from 0.7:
  - Path syntax: `/:id` → `/{id}` (matches OpenAPI)
  - `Option<T>` extractor no longer swallows all rejections — types must implement `OptionalFromRequestParts`/`OptionalFromRequest`
  - WebSocket messages use `Bytes`/`Utf8Bytes` instead of `Vec<u8>`/`String`
  - `#[async_trait]` removed — uses native async-fn-in-trait
- **0.9** in development as of May 2026.

## cargo-dist

- **Current**: v0.31+ (released Feb 2026)
- **History**: axo.dev (the company) wound down — domain `axo.dev` is for sale. Tool survives under `github.com/axodotdev/cargo-dist`; Astral's fork patches were merged back in v0.29.0.

---

## Per-release feature index (1.75 → 1.95)

Every stabilization that matters for code recommendations. When suggesting an API or syntax, verify it's stable as of the project's `rust-version`.

### Language

| Feature | Since | Replaces |
|---|---|---|
| `async fn` in traits (AFIT) and RPITIT | 1.75 (Dec 2023) | `#[async_trait]` for static dispatch |
| C-string literals `c"..."` | 1.77 | `CString::new("...").unwrap()` |
| `offset_of!` macro | 1.77 | Unsafe macros that materialized dangling pointers |
| `#[diagnostic::on_unimplemented]` | 1.78 | Hand-written trait error messages |
| Inline `const { ... }` expressions | 1.79 | Spinning up named `const ITEM:` for one-off compile-time values |
| Associated-type bounds `T: Trait<Assoc: Bounds>` | 1.79 | Intermediate generic parameters |
| Exclusive range patterns `0..10` | 1.80 | `0..=9` workarounds |
| Checked `cfg` (warns on typos) | 1.80 | Silently-typo'd feature names |
| `LazyLock` / `LazyCell` | 1.80 | `lazy_static!` macro, `once_cell::sync::Lazy` |
| `OnceLock` / `OnceCell` | 1.70 | `once_cell::sync::OnceCell` |
| `#[expect(lint, reason = "...")]` | 1.81 | `#[allow(lint)]` (stale suppression detection) |
| `core::error::Error` | 1.81 | std-only `Error` trait (no_std now supported) |
| Precise impl-Trait capture `+ use<...>` | 1.82 | Implicit capture workarounds |
| `&raw const` / `&raw mut` | 1.82 | `addr_of!` / `addr_of_mut!` macros |
| `unsafe extern "C" { pub safe fn sqrt(...); }` | 1.82 | Bare `extern "C"` blocks (mandatory in Edition 2024) |
| const refs to statics, `&mut` in const eval | 1.83 | Unsound workarounds |
| MSRV-aware resolver | 1.84 | Dep version selection that ignored `rust-version` |
| Strict provenance APIs (`addr`, `with_addr`, `expose_provenance`, `without_provenance`) | 1.84 | `as usize` / `as *const T` round-trips |
| **Edition 2024 ships** | 1.85 (Feb 2025) | Edition 2021 for new code |
| `async \|\| { ... }` closures, `AsyncFn`/`AsyncFnMut`/`AsyncFnOnce` traits | 1.85 | `\|\| async { ... }` workaround |
| Trait upcasting `&dyn Sub` → `&dyn Super` | 1.86 | Manual `as_supertrait()` methods |
| `target_feature_11` (safe `#[target_feature]` fns) | 1.86 | `unsafe` wrapping of safe SIMD code |
| `Vec::pop_if` | 1.86 | Manual `if last_matches { pop() }` two-step |
| `Vec::extract_if`, `LinkedList::extract_if` | 1.87 | Manual filter-and-drain dance |
| `std::io::pipe` (cross-platform anonymous pipes) | 1.87 | `os_pipe` / unix-specific pipe code |
| **`let` chains** in `if`/`while` (Edition 2024 only) | 1.88 | `if_chain!` crate, nested `if let` |
| `#[unsafe(naked)]` + `naked_asm!` | 1.88 | Hand-rolled syscall trampolines |
| `Cell::update` | 1.88 | Load/store/replace dance |
| `Result::flatten()` | 1.89 | `.and_then(\|x\| x)` for `Result<Result<T, E>, E>` |
| `File::lock`, `lock_shared`, `try_lock`, `unlock` | 1.89 | `fs2` / `fd-lock` crates |
| Const-generic `_` inference | 1.89 | Verbose const param specification |
| `mismatched_lifetime_syntaxes` lint (warn) | 1.89 | Inconsistent elision tripping up readers |
| `#[repr(u128)]` / `#[repr(i128)]` enums | 1.89 | Workarounds for large discriminants |
| **`rust-lld` default linker on x86_64-linux** | 1.90 | Manually configured `mold`/`lld`. 40% faster incremental link |
| `cargo publish --workspace` | 1.90 | Manual topological publish scripts |
| `aarch64-pc-windows-msvc` Tier 1 | 1.91 | The GNU target on ARM Windows |
| `strict_add`, `strict_sub`, `strict_mul` | 1.91 | `checked_add().unwrap()` (always panics on overflow, even in release) |
| `Duration::from_mins()`, `from_hours()` | 1.91 | `Duration::from_secs(n * 60)` |
| Never-type fallback deny-by-default | 1.92 | Code depending on `!` → `()` fallback |
| **panic=abort emits unwind tables on Linux** | 1.92 | Losing backtraces with `panic = "abort"` |
| `RwLockWriteGuard::downgrade()` | 1.92 | Drop + reacquire (race-prone) |
| `String::into_raw_parts`, `Vec::into_raw_parts` | 1.93 | `mem::forget` + manual tuple |
| `std::fmt::from_fn` | 1.93 | `struct Wrapper(T)` + manual `Display` impl |
| `cfg` on individual `asm!` statements | 1.93 | Duplicating asm blocks per platform |
| `<[T]>::array_windows::<N>()` | 1.94 | `.windows(n).map(\|w\| <&[T; N]>::try_from(w).unwrap())` |
| `Peekable::next_if_map` | 1.94 | `peek().filter().map()` two-step |
| `LazyLock::get`, `force_mut` | 1.94 | Forced init when you only wanted to check |
| TOML 1.1 in `Cargo.toml` (multi-line inline tables, trailing commas) | 1.94 | Crowded inline tables |
| **`cfg_select!` macro** | 1.95 | `cfg-if` crate |
| **`if let` guards in `match` arms** | 1.95 | Nested `match` in arm bodies |
| `AtomicPtr/AtomicBool/AtomicIsize/AtomicUsize::update` / `try_update` | 1.95 | Hand-rolled `compare_exchange` CAS loops (other Atomic widths remain unstable) |
| `Vec::push_mut`, `VecDeque::push_*_mut`, `LinkedList::push_*_mut` | 1.95 | `vec.push(x); vec.last_mut().unwrap()` |
| `bool: TryFrom<{integer}>` | 1.95 | Manual `n == 0 \|\| n == 1` checks |
| `core::range::{RangeInclusive, RangeInclusiveIter}` module | 1.95 | Old `Range` types — only `RangeInclusive` and its iterator are stabilized in `core::range` so far; full migration is staged |
| `core::hint::cold_path()` | 1.95 | `#[cold]` on the whole function when only one branch is cold |
| `MaybeUninit<[T;N]>` ↔ `[MaybeUninit<T>;N]` via `From`/`AsRef`/`AsMut` | 1.95 | `transmute` between the two |

### Build / tooling

- **LLVM 22 bundled with Rust 1.95** — newer auto-vectorization.
- **`rust-lld` default on x86_64-linux since 1.90.** Don't override unless measured reason.
- **Apple ARM platforms Tier 2 since 1.95** (`aarch64-apple-tvos`, `-watchos`, `-visionos` + sims).
- **`x86_64-apple-darwin` demoted to Tier 2 since 1.90** — Intel Mac no longer guaranteed (1.89 was last Tier 1 release).
- **`aarch64-pc-windows-msvc` Tier 1 since 1.91.**
- **JSON target specs destabilized since 1.95** — require `-Z unstable-options`.

### New default lints

| Lint | Level | Since | Catches |
|---|---|---|---|
| `mismatched_lifetime_syntaxes` | warn | 1.89 | Inconsistent explicit/elided lifetime syntax |
| `dangerous_implicit_autorefs` | deny | 1.89 | Implicit autoref of raw pointer dereferences |
| `dangling_pointers_from_locals` | warn | 1.91 | Returning raw pointers to local variables |
| `never_type_fallback_flowing_into_unsafe` | deny | 1.92 | Code affected by upcoming `!` type changes |
| `dependency_on_unit_never_type_fallback` | deny | 1.92 | Code depending on `!` → `()` fallback |
| `function_casts_as_integer` | warn | 1.93 | `fn` item cast directly to int |
| `const_item_interior_mutations` | warn | 1.93 | Mutating interior-mutable `const` items |
| `unused_visibilities` | warn | 1.94 | Visibility on `const _` |
| `ambiguous_glob_imported_traits` | future-incompat | 1.95 | Glob-imported traits with ambiguous resolution |

---

## Still nightly in May 2026

Don't recommend these for production code; describe workarounds instead.

| Feature | Status | 2026 workaround |
|---|---|---|
| `AsyncDrop` | Nightly (tracking #126482) | Explicit `async fn close(self)` + `DropBomb` |
| `AsyncIterator` / `Stream` in std | Nightly | `futures::Stream` + `async-stream::stream!` macro |
| `gen` blocks / coroutines | Nightly | Iterator combinators; `async-stream` for async streams |
| `Allocator` trait (per-collection) | Nightly 6+ years | `allocator_api2` polyfill |
| Pin language support | Library-only | `pin-project-lite` |
| Polonius borrow checker | Nightly (2026 goal) | Restructure or use `entry()`-style APIs |
| Parallel rustc frontend `-Z threads=N` | Nightly | Use on nightly; 15-50% wallclock improvement |
| `std::simd` (portable_simd) | Nightly | `pulp`, `macerator`, or `wide` for stable |
| Full specialization | Nightly (`min_specialization` is std-only) | Trait dispatch via marker types |
| `-Zscript` cargo single-file scripts | Nightly | `rust-script` crate (works on stable) |
| `trim-paths` profile setting | Nightly | Hand-craft `--remap-path-prefix` |
| Return-type notation (RTN) for async traits | Nightly | `trait_variant::make` macro |
| Cranelift codegen backend (`rustc_codegen_cranelift`) | Nightly-only as `rustc-codegen-cranelift-preview`. Active 2025H2 project goal to ship stable. | Use on nightly for ~20% dev-build speedup; not yet available on stable. |

---

## 2026 Deprecation watchlist

Existing code mostly still works — migrate when natural, not urgently.

### Unmaintained crates

| Crate | Advisory | Migrate to |
|---|---|---|
| `async-std` | [RUSTSEC-2025-0052](https://rustsec.org/advisories/RUSTSEC-2025-0052.html) (Aug 2025, discontinued Mar 1, 2025) | `smol` or Tokio |
| `bincode` | [RUSTSEC-2025-0141](https://rustsec.org/advisories/RUSTSEC-2025-0141) (Dec 2025; team cites doxxing/harassment, v1.3.3 considered complete) | `postcard` (size-optimized) / `rkyv` + `bytecheck` (zero-copy) / `wincode` (community fork) |
| `flume` | Casual maintenance mode (announced 2024) | New code: `crossbeam-channel` (sync), `tokio::sync::mpsc` (async), `crossfire` (high-throughput) |

### Superseded by std

| Old | New (std stable since) |
|---|---|
| `once_cell::sync::Lazy`, `lazy_static!` macro | `LazyLock` (1.80) |
| `once_cell::sync::OnceCell` (for new code) | `OnceLock` (1.70) |
| `addr_of!`, `addr_of_mut!` macros | `&raw const`, `&raw mut` (1.82) |
| `fs2`, `fd-lock`, `file-lock` crates (for the simple cases) | `File::lock`, `lock_shared`, `try_lock` (1.89) |
| `mem::uninitialized`, `mem::zeroed` (deprecated) | `MaybeUninit<T>` (long stable) |

### Superseded by language

| Old | New |
|---|---|
| `if_chain!` crate | native let chains (1.88, Edition 2024) |
| `cfg-if` crate (for new code) | `cfg_select!` macro (1.95) |
| `as usize` round-trips for pointer math | Strict provenance APIs (1.84) |
| Bare `extern "C"` blocks | `unsafe extern "C"` (Edition 2024) |
| `#[no_mangle]`, `#[link_section]`, `#[export_name]` | `#[unsafe(no_mangle)]`, etc. (Edition 2024) |
| `#[allow(lint)]` (when transient) | `#[expect(lint, reason = "...")]` (1.81) |
| `#[bench]` macro | `criterion` or `divan` (hard error on stable since 1.88) |
| `async-trait` for static dispatch | Native AFIT (1.75). Keep `async-trait` only for `dyn Trait` |
| `std::collections::LinkedList` | `Vec` or `VecDeque` (almost always — even LinkedList's own docs say so) |
| `packed_simd2` | `pulp`, `wide`, `macerator` (or wait for `std::simd`) |
| `actions-rs/*` GitHub Actions | `dtolnay/rust-toolchain` + `Swatinem/rust-cache` + `taiki-e/install-action` |
| `actions-rs/audit-check` | `rustsec/audit-check` |

### Library status concerns

| Crate | Concern | Migration target |
|---|---|---|
| `sled` (for new projects) | Beta forever — ~6 years pre-1.0, known memory issues | `redb` 4.x (B-tree) or `fjall` 3.x (LSM) |
| GPUI (third-party use) | Zed-tied, pre-1.0, breaking changes, sparse docs | Pick a different Rust UI framework (Tauri/egui/Iced/Slint) |
| Floem (for i18n apps) | IME bugs | Iced for serious app architecture |
| `chrono` (for new code touching tz) | Still receiving releases (0.4.44, Feb 2026), but the maintainer signaled intent to wind down chrono and chrono-tz in a Jan 2026 year-in-review and now recommends jiff | `jiff` (still pre-1.0 in May 2026 — see Rule 5 in decision-rules.md) |

---

## Default crate set (the 2026 toolbox)

Reach for these by default; alternatives need justification.

| Job | Default | Notes |
|---|---|---|
| Serialization | `serde` | Universal |
| Async runtime | `tokio` 1.5x | LTS lines: 1.47 (Sep 2026), 1.51 (Mar 2027). MSRV 1.71 |
| CLI args | `clap` v4 (derive) | Universal |
| Structured logging | `tracing` + `tracing-subscriber` + `EnvFilter` | Never `Span::enter()` across `.await` |
| Library errors | `thiserror` v2 | |
| App errors | `anyhow` v2 | `.context()` at layer boundaries |
| HTTP client (async) | `reqwest` | Build one `Client` and clone; set `.timeout()` AND `.connect_timeout()` |
| HTTP client (sync, minimal) | `ureq` | No async, smaller dep tree |
| Web framework | `axum` 0.8+ | Path syntax `/{id}` |
| Date/time | See Rule 5 in decision-rules.md | jiff for new code w/ tz; chrono for stability |
| Builder | `bon` (new) / `derive_builder` (validation hooks) | See Rule 4 |
| UUID | `uuid` | v7 for ordered IDs, v4 for tokens |
| Regex | `regex` with `OnceLock<Regex>` | Recompiling is #1 perf foot-gun |
| Local SQLite | `rusqlite` with `bundled` | |
| Embedded DB (pure Rust) | `redb` 4.x or `fjall` 3.x | Not `sled` for new projects |
| Test runner | `cargo-nextest` | Run `cargo test --doc` separately |
| Snapshot tests | `insta` | `cargo insta review`, never bulk-accept |
| Property tests | `arbtest` or `proptest` | `arbtest` shares `Arbitrary` with `cargo-fuzz` |
| Async test time | `#[tokio::test(start_paused = true)]` | `tokio::time::advance(Duration)` for control |
| Concurrency permutation | `loom` | For unsafe primitives, not app code |
| Fuzz | `cargo-fuzz` + `libfuzzer-sys` | Or `afl.rs` on stable |
| Benchmarks | `criterion` or `divan` | NOT `#[bench]` |
| Profiler | `samply` | Cross-platform Firefox-UI default |
| Coverage | `cargo-llvm-cov` | Signal not target |
| HTTP mock | `wiremock` | Real local server tests serialization |
| Trait mock (last resort) | `mockall` | Only at I/O boundaries |
| File watcher | `notify` v8 + `notify-debouncer-full` | Mandatory debouncing |
| Cross-platform paths | `directories::ProjectDirs` | |
| UTF-8 paths (tools) | `camino` | |
| Clipboard | `arboard` + `wayland-data-control` feature | |
| Keychain | `keyring-core` v1 (the embeddable library); `keyring` v4 is now a demo/CLI crate | |
| Concurrent map | `dashmap` | Don't hold `Ref`/`RefMut` while calling other methods |
| Hasher (trusted keys) | `foldhash` or `rapidhash` | `rustc-hash` for short int keys |
| Hasher (DoS-resistant) | `ahash` | |
| Allocator (Tokio servers, musl) | `mimalloc` or `tikv-jemallocator` | musl + default = 7-20× slowdown |
| JSON (perf) | `sonic-rs` | 1.5-2× simd-json, 3-4× serde_json |
| Zero-copy bytes | `bytes` + `zerocopy` derives | |
| Pin projection | `pin-project-lite` | |
| Self-referential | `self_cell` | Don't unless you really must |
| Color output | `owo-colors` or `anstyle` | |
| Progress bars | `indicatif` | Wrap async I/O, don't manual `inc()` |

---

## Source verification

- Cloudflare Nov 18, 2025 outage — [official postmortem](https://blog.cloudflare.com/18-november-2025-outage/)
- RUSTSEC advisories — [rustsec.org/advisories](https://rustsec.org/advisories/)
- Per-release stabilizations — [blog.rust-lang.org/releases](https://blog.rust-lang.org/releases/)
- Tokio LTS schedule — [tokio.rs releases](https://github.com/tokio-rs/tokio/releases)
- Tokio merge-back history — [Tokio issue #1318](https://github.com/tokio-rs/tokio/issues/1318)
- Workspace guidance — [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)
- Unwrap as invariant assertion — [Using unwrap() in Rust is Okay](https://burntsushi.net/unwrap/)
- Actor pattern — [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/)
