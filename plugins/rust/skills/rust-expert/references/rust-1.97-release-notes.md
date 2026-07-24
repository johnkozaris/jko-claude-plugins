# Rust 1.96.1 through 1.97.1 Release Notes

> Complete migration-relevant digest of the stable releases after this skill's
> Rust 1.96.0 anchor. Verified against the Rust, Cargo, and Clippy upstream
> release records on July 24, 2026.

## Release timeline

| Release | Date | Role |
|---|---|---|
| 1.96.1 | June 30, 2026 | Security and compiler-correctness point release |
| 1.97.0 | July 9, 2026 | Feature release |
| 1.97.1 | July 16, 2026 | LLVM miscompilation fix; current stable |

Upgrade directly to **1.97.1**. The 1.97.1 compiler fix addresses a latent LLVM
miscompilation present since at least Rust 1.87, not merely a cosmetic 1.97.0
regression.

## Rust 1.96.1

The point release contains three classes of fixes:

- rustc fixes a miscompilation in a MIR optimization
  ([rust#158214](https://github.com/rust-lang/rust/pull/158214)).
- Cargo fixes automatic retries for spurious HTTP failures
  ([cargo#17131](https://github.com/rust-lang/cargo/pull/17131),
  [cargo#17134](https://github.com/rust-lang/cargo/pull/17134)).
- Cargo patches its vendored libssh2 for CVE-2025-15661 (critical),
  CVE-2026-55199 (high), and CVE-2026-55200 (high)
  ([cargo#17140](https://github.com/rust-lang/cargo/pull/17140)).

There are no language, library API, rustdoc, rustfmt, Miri, or Clippy feature
changes in 1.96.1.

## Rust 1.97.0

### Compiler and language

- v0 symbol mangling is now the stable default. Generic instantiations remain
  represented in symbol names instead of being hidden behind hashes, producing
  more consistent demangling. Update old debuggers, profilers, and symbol tools.
  The legacy scheme is nightly-only and scheduled for removal.
- `Result<T, U>` and `ControlFlow<U, T>` receive the same `must_use` treatment
  as `T` when `U` is uninhabited.
- The allow-by-default `dead_code_pub_in_binary` lint detects unused `pub` items
  in binary crates. Applications should normally opt it into `warn`.
- The target features `div32`, `lam-bh`, `lamcas`, `ld-seq-sa`, and `scq` are
  stable.
- `cfg(target_has_atomic_primitive_alignment)` is stable.
- More import forms may end with `self`.

### Standard library

The following APIs or implementations are stable in 1.97:

- `{integer}::bit_width(self) -> u32`.
- `{integer}::highest_one(self) -> Option<u32>` and
  `{integer}::lowest_one(self) -> Option<u32>` return set-bit indices.
- `{integer}::isolate_highest_one(self) -> Self` and
  `{integer}::isolate_lowest_one(self) -> Self` retain one set bit.
- The same five operations on `NonZero<{integer}>`; the non-zero type removes
  unnecessary zero handling.
- `Default for core::iter::RepeatN<A>`.
- `Copy for core::ffi::FromBytesUntilNulError`.
- `Send for std::fs::File` on UEFI targets.
- `char::is_control` is now callable in const contexts.

### Cargo

Stable additions:

- `build.warnings = "allow" | "warn" | "deny"` in Cargo configuration, with
  the `CARGO_BUILD_WARNINGS` environment equivalent. Prefer this over
  `RUSTFLAGS="-D warnings"` because changing it does not invalidate build
  artifacts.
- `resolver.lockfile-path`, allowing the lockfile to live outside a read-only
  source tree.
- `-m` as shorthand for `--manifest-path`.

Stable behavior and diagnostics:

- `cargo rustfmt` suggests the correct `cargo fmt` command.
- `.cargo-checksum.json` gains a `$comment` explaining that it is not a
  security mechanism.
- Mistyped `-p` package names receive similar-workspace-member suggestions.
- `cargo clean` refuses a `--target-dir` that does not resemble a Cargo target
  directory, but accepts an explicitly named directory that does not yet exist.
- `cargo clean` respects `build.target`.
- Relative `[env]` paths from included configs resolve against the correct
  config file, and included config paths normalize `..`.
- `cargo help` gives temporary man pages a `.1` extension for NetBSD.
- Workspace publishing no longer reports a false deadlock while packages wait
  for registry confirmation, fixing a 1.96 regression.
- Cargo's crates.io implementation no longer depends on `curl`; internal
  platform dependencies were also reduced.

Nightly-only Cargo work in this release remains unstable: rustdoc JSON output
rebuilds, `-Zscript` hints, `-Zcargo-lints`, `-Zjson-target-spec`, and
`-Zpublic-dependency` changes are not production recommendations.

### rustdoc

- `rustdoc --emit` is stable.
- `rustdoc --remap-path-prefix` is stable.

There are no user-facing rustfmt, Miri, bootstrap, or installer changes in
1.97.0.

### Clippy

New lints:

| Lint | Group |
|---|---|
| `manual_assert_eq` | `pedantic` |
| `manual_clear` | `perf` |
| `useless_borrows_in_formatting` | `perf` |
| `inline_trait_bounds` | `restriction` |
| `inline_modules` | `restriction` |

`nonminimal_bool` and `overly_complex_bool_expr` move to `pedantic`. Clippy also
stops treating `(a..b).into_iter()` as a useless conversion, preserving the
forward-compatible spelling for the future range-type transition.

### Compatibility and behavior changes

- `std::pin::pin!` no longer deref-coerces its input. `pin!(x)` for
  `x: &mut T` now correctly yields `Pin<&mut &mut T>`. The old behavior,
  introduced in 1.88, was unsound.
- Free items under `std::char` are deprecated; use inherent `char::` and
  `u32::` APIs.
- Linker stdout/stderr is no longer hidden. The new warn-by-default
  `linker_messages` lint is intentionally outside the `warnings` lint group.
- Reliance on `f32: From<{float}>` to constrain an inferred float now receives
  a future-compatibility warning.
- Hidden `f64` methods deprecated since Rust 1.0 are removed.
- `varargs_without_pattern` is now reported through dependencies.
- Generic arguments on module path segments are rejected even when the module
  re-exports a generic enum variant.
- Invalid Mach-O `link_section` values now error.
- Empty `#[export_name = ""]` now errors.
- `#[link_name]` and `#[link(name = ...)]` values receive stricter validation.
- Tuple-index shorthand is rejected in struct patterns.
- Some enums without a representation guarantee receive different internal
  encodings. Code must never depend on an unspecified enum layout.
- Windows socket writes after write-side shutdown now report
  `io::ErrorKind::BrokenPipe` instead of `Other`.
- `nvptx64-nvidia-cuda` drops old GPU architectures and instruction sets.

## Rust 1.97.1

1.97.1 backports an LLVM fix for a miscompilation and reverts a rustc IR change
that made the latent bug easier to trigger. The underlying LLVM issue has been
present since at least Rust 1.87; the rustc revert is an additional precaution.
There are no language, library, Cargo, rustdoc, rustfmt, Miri, or Clippy feature
changes in this point release.

## Migration checklist

1. Pin 1.97.1, not 1.97.0, then rebuild release artifacts.
2. Verify debugger, profiler, crash-symbolication, and backtrace tooling handles
   v0 symbols.
3. Replace CI `RUSTFLAGS="-D warnings"` with
   `CARGO_BUILD_WARNINGS=deny`; configure `linker_messages` separately.
4. Run `cargo check --all-targets --all-features` for the pin and FFI attribute
   tightening, then run Clippy with the new lint set.
5. Replace deprecated `std::char::*` calls with inherent APIs.
6. On Windows, update error matching that expected `Other` after socket
   shutdown.
7. Re-run release tests for code built by Rust 1.87 through 1.97.0 because the
   fixed LLVM optimization bug predates 1.97.

## Primary sources

- [Rust 1.96.1 upstream notes](https://doc.rust-lang.org/stable/releases.html#version-1961-2026-06-30)
- [Rust 1.97.0 announcement](https://blog.rust-lang.org/2026/07/09/Rust-1.97.0/)
- [Rust 1.97.0 upstream notes](https://doc.rust-lang.org/stable/releases.html#version-1970-2026-07-09)
- [Rust 1.97.1 announcement](https://blog.rust-lang.org/2026/07/16/Rust-1.97.1/)
- [Rust 1.97.1 root-cause issue](https://github.com/rust-lang/rust/issues/159035)
- [Cargo 1.97 changelog](https://doc.rust-lang.org/nightly/cargo/CHANGELOG.html#cargo-197-2026-07-09)
- [Clippy 1.97 changelog](https://github.com/rust-lang/rust-clippy/blob/master/CHANGELOG.md#rust-197)
