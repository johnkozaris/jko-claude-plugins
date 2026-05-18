# Workspace Organization

> When and how to split a Rust project into sub-crates. Companion to [modules-cargo.md](modules-cargo.md) (which covers single-crate organization) and [architecture.md](architecture.md) (which covers code-level patterns).

The single principle: **split when there's a consumer served by the boundary**. Not for tidiness. Not for "future flexibility." Tokio split into many sub-crates around 2019 and merged them back ([issue #1318](https://github.com/tokio-rs/tokio/issues/1318)) — the maintenance overhead and user confusion exceeded the modularity benefit. The lesson is canonical: don't pre-split.

---

## Why people split

A Rust crate is the unit of compilation. When you change a file, rustc recompiles the entire crate it belongs to. For a 1k-line crate that's seconds. For a 50k-line crate it's minutes.

After a split, only the changed crate plus its reverse dependencies rebuild. Reported gains: **~10% on clean builds, ~30% on incremental** ([benw.is benchmark](https://benw.is/posts/how-i-improved-my-rust-compile-times-part2)), up to **70% on large workspaces** with clean separation. The crate DAG also lets Cargo parallelize compilation across crates.

A workspace gives you **enforceable boundaries**. Module-level `pub(crate)` is one mechanism; crate boundaries are stronger. If `domain` is its own crate and `infra` depends on it, the compiler refuses to let `domain` import from `infra` — it would be a circular dep.

And: workspaces let you **publish selectively**. Most internal crates have `publish = false` and `version = "0.0.0"`. The one or two that are libraries with public APIs get real versions and go to crates.io.

---

## The signals you should split

Any one of these is enough.

1. **Compile time pain.** Saving a file triggers a 30s+ rebuild. The single best fix is to push code into a library crate that's already compiled; only your thin binary recompiles.

2. **Two binaries share non-trivial code.** A `server` and a `cli` both call into the same business logic. Three crates: `core` (library), `server` (binary), `cli` (binary). Each binary stays thin.

3. **You're publishing a library AND keeping a private CLI/daemon around it.** The library is the contract with crates.io users; the CLI is the project's internal user. Two crates: published `mylib`, private `mylib-cli`.

4. **You have a plugin SDK that third parties depend on.** Zellij's `zellij-tile` is the canonical example — it's the *only* published crate in the workspace; `zellij-server`, `zellij-client`, and `zellij-utils` are all internal.

5. **FFI: pure Rust logic + C-ABI shim.** Convention: `mylib-core` (rlib, all the logic, fully testable in Rust) + `mylib-ffi` (cdylib + staticlib, only `extern "C"` shim functions, never panics across the boundary). UniFFI and similar generators consume the `-core` and produce bindings from the `-ffi`.

6. **Multiple teams own different subsystems.** PRs collide because everyone's editing the same monorepo crate. Splitting the codebase into team-owned crates moves the merge boundary to the crate level.

7. **A specific build dependency is heavy and crate-isolated.** A proc-macro crate, a build-script crate, or a codegen crate that you don't want to recompile when business code changes.

---

## The signals you should NOT split

1. **"It will be cleaner."** [Tokio merged sub-crates back in 2019](https://github.com/tokio-rs/tokio/issues/1318). The aesthetic of "many small crates" is not worth the cost.

2. **"For the future."** YAGNI. Split when the seam hurts. Splitting too early locks you into a public interface across a boundary you don't yet understand.

3. **"To enforce clean interfaces."** Modules with `pub(crate)` do this within a single crate. You don't need crate boundaries to enforce visibility.

4. **"To copy a layered architecture from another language."** Java enterprise apps split `domain`, `infra`, `api` into separate modules because their build systems can't enforce dependency direction otherwise. Rust's `pub(crate)` already does this within one crate.

5. **"Because we use a workspace anyway."** A workspace can have one crate. That's fine. The workspace gives you `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]` even with one member.

---

## Zellij: a real worked example

Zellij is the canonical mid-size Rust workspace. Its layout shows why each split exists.

```
zellij/
├── Cargo.toml              # virtual manifest, [workspace]
├── zellij/                 # the BINARY (small, fast relink)
├── zellij-client/          # client-side library (terminal UI rendering)
├── zellij-server/          # server-side library (PTY, tiling, plugin host)
├── zellij-utils/           # SHARED code (used by both client and server)
├── zellij-tile/            # PUBLISHED plugin SDK (the project's contract with plugin authors)
├── default-plugins/        # built-in plugins
│   ├── status-bar/
│   ├── tab-bar/
│   └── session-manager/
└── xtask/                  # build automation in Rust
```

Why each:
- **`zellij`** is the binary. Kept thin so re-linking is fast.
- **`zellij-client` and `zellij-server`** are two independently-shippable runtime components. Splitting lets each evolve at its own pace.
- **`zellij-utils`** is the shared code. Without it, every change to a utility would force both client and server to recompile.
- **`zellij-tile`** is the **only published crate**. Plugin authors take a dep on `zellij-tile`; everything else is implementation detail.
- **Built-in plugins** are workspace members because they each compile to a separate WASM artifact. Their boundaries are real.
- **`xtask`** is build automation (matklad's pattern). Not shipped; just runs locally.

Plugin communication uses Protobuf via `.proto` files compiled at build time — the boundary between host and plugins is across the WASM ABI, not Rust types.

This is **purposeful**, not ornamental. Every crate has a *consumer* that requires the boundary.

---

## matklad's flat layout

From [Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html), based on rust-analyzer (~200k LOC, ~50 crates):

```
my-project/
├── Cargo.toml              # virtual manifest, [workspace]
├── crates/                 # internal crates
│   ├── my-project/         # the user-facing main crate
│   ├── my-project-core/
│   ├── my-project-utils/
│   ├── my-project-plugin-foo/
│   └── my-project-plugin-bar/
├── libs/                   # OPTIONAL: separated publishable libraries
│   └── shared-protocol/
└── xtask/                  # build automation
```

Rules:

1. **Flat, not hierarchical.** `crates/my-project-foo/`, never `crates/foo/bar/baz/`. Cargo's crate namespace is flat — nested directories invent a second hierarchy that doesn't match.
2. **Folder name == crate name.** No abbreviations. `crates/my-project-core/` contains `my-project-core`, not `core`.
3. **Internal crates**: `version = "0.0.0"`, `publish = false`. This makes accidental `cargo publish` impossible.
4. **Optional `libs/` separation** for crates intended for crates.io publishing. Enforces a one-way dependency: internal `crates/` can depend on `libs/`, never reverse.
5. **`xtask` crate** for all automation. Cross-platform, shares dependencies with the workspace, written in Rust.

---

## Shared workspace configuration

`[workspace.package]`, `[workspace.dependencies]`, and `[workspace.lints]` centralize what would otherwise drift.

```toml
# Root Cargo.toml — virtual manifest
[workspace]
resolver = "3"  # opt-in on any edition since Rust 1.84; the default for Edition 2024
members = ["crates/*", "libs/*", "xtask"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.85"
license = "MIT OR Apache-2.0"
repository = "https://github.com/acme/my-project"

[workspace.dependencies]
tokio = { version = "1.5", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
thiserror = "2"

[workspace.lints.rust]
unsafe_code = "forbid"   # relax per-crate where actually needed

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
correctness = "deny"
```

Member crate uses inheritance:

```toml
# crates/my-project-core/Cargo.toml
[package]
name = "my-project-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
tokio.workspace = true
serde = { workspace = true, features = ["rc"] }   # additive features OK
anyhow.workspace = true

[lints]
workspace = true
```

This eliminates version drift and gives you one place to bump lints, MSRV, and license metadata.

---

## The feature-unification trap

When you have a workspace, Cargo **unifies features across members**. If crate A wants `serde = { features = ["x"] }` and crate B wants `serde = { features = ["y"] }`, both build with `["x", "y"]` enabled.

Usually harmless. Occasionally breaks platform-specific or mutually-exclusive features. The fix is one of:

1. **Use `[workspace.dependencies]`** to declare each dep once with a baseline feature set; per-crate adds features additively.
2. **`dep:foo` syntax** (resolver v2/v3) prevents implicit feature aliasing from optional deps.
3. **`foo?/bar` weak features** for optional cascading.
4. **`resolver = "3"`** (Edition 2024 default) — better isolates build-deps, dev-deps, and platform-specific features.
5. **For pathological cases**: [`cargo-hakari`](https://docs.rs/cargo-hakari) manages a "workspace-hack" crate that pins the unified feature set and prevents redundant rebuilds. Reported up to 50% savings on consecutive builds in large monorepos.

---

## When publishing matters

If even one workspace crate goes to crates.io:

- Run **`cargo-semver-checks`** before publish to catch accidental API breaks (Predrag Gruevski's empirical data: 1 in 31 releases violates semver).
- Run **`cargo publish --workspace`** (stable 1.90) to publish in topological order with one command.
- Pin **MSRV** via `rust-version` in `Cargo.toml`. The MSRV-aware resolver (1.84) prevents accidental MSRV bumps from transitive deps.
- Decide on `Cargo.lock` policy. The 2023 Cargo guidance is **commit it for everything, including libraries** — `cargo package` strips it from published output, so downstream is unaffected.

---

## Counter-examples: when one crate is right

Not every project needs a workspace. Some real Rust projects that lived (or live) happily single-crate at scale:

- **fd** — modern `find` replacement, single crate, multi-platform binary.
- **bat** — `cat` with syntax highlighting, single crate, single binary.
- **ripgrep** (originally) — single crate for several years. Later split into a workspace as plugin-like features (matcher, searcher, printer, ignore, globset) emerged.
- **Most internal services for their first 6-12 months.**
- **Most CLI tools** ever written.

If you're under ~10k LOC, single binary, single team, single deployment — stay single-crate. Workspace overhead (more `Cargo.toml`s to maintain, more cross-crate version coordination, feature unification surprises) isn't worth paying without a concrete win.

---

## Summary

Split a Rust project into a workspace when there's a **consumer served by the boundary**:

- Compile-time pain (~30s+ small-edit rebuilds)
- Two binaries sharing non-trivial code
- A published library + private CLI around it
- A published plugin SDK (Zellij's `zellij-tile`)
- An FFI shim (`-core` + `-ffi` pattern)
- Team ownership boundaries

Do not split for tidiness, for "future flexibility," or because the architecture looks more serious with multiple crates. Tokio's merge-back ([issue #1318](https://github.com/tokio-rs/tokio/issues/1318)) is the canonical warning.

When you do split: flat layout (`crates/<name>/`), internal crates `publish = false`, shared config via `[workspace.*]` inheritance, automation in an `xtask` crate, `cargo-semver-checks` before publishing the public surface.

Sources:
- [matklad — Large Rust Workspaces](https://matklad.github.io/2021/08/22/large-rust-workspaces.html)
- [Tokio issue #1318 — collapse sub-crates](https://github.com/tokio-rs/tokio/issues/1318)
- [matklad/cargo-xtask](https://github.com/matklad/cargo-xtask) — the xtask pattern
- [benw.is — Compile times case study](https://benw.is/posts/how-i-improved-my-rust-compile-times-part2)
- [cargo-hakari](https://docs.rs/cargo-hakari) — for pathological feature-unification cases
- [The Cargo Book — Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- Real codebases to study: [Zellij](https://github.com/zellij-org/zellij), [Helix](https://github.com/helix-editor/helix), [ripgrep](https://github.com/BurntSushi/ripgrep), [Bevy](https://github.com/bevyengine/bevy)
