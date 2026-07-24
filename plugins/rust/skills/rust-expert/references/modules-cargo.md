# Modules, Workspaces & Cargo

## Project Structure

Put all business logic in `lib.rs`. Keep `main.rs` as a thin orchestration wrapper. This makes logic testable without running the binary.

### Privacy Model
- Everything private by default
- `pub` — visible to all (semver commitment)
- `pub(crate)` — visible within crate only (use heavily for internal sharing)
- `pub(super)` — visible to parent module

## When to Split Crates

**Stay in one crate:** <10k LOC, modules have mutual dependencies, small team.

**Split when:** compilation parallelism matters, components are independently publishable, team/domain boundaries, optional heavy dependencies.

**Start with:** one library crate + one binary crate. Add more only on genuine pain.

Circular dependency fix: extract shared code into a third crate.

## Workspace Setup

```toml
# Root Cargo.toml — virtual manifest
[workspace]
resolver = "3"            # Latest resolver (Edition 2024)
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.XX"     # Set to project's MSRV
license = "MIT OR Apache-2.0"

[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
```

Member crates inherit:
```toml
[package]
name = "my-service"
version.workspace = true
edition.workspace = true

[dependencies]
tokio.workspace = true
serde = { workspace = true, features = ["derive"] }
```

## Feature Flag Rules

**Features MUST be additive.** Cargo unions all features across the dependency graph. A `disable-foo` feature is meaningless — it cannot be honored.

```toml
# CORRECT: opt IN to std
[features]
default = ["std"]
std = ["some-dep/std"]

# WRONG: opt OUT of std
[features]
no_std = []
```

- Use `dep:` prefix for optional deps (Rust 1.60+): `serde = ["dep:serde"]`
- Adding to `default` is semver-compatible but breaks `default-features = false` users
- Test in CI: no features, all features, each significant feature alone

## Cargo.lock Policy

| Project type | Commit Cargo.lock? |
|---|---|
| Binary / application | Yes — reproducible builds |
| Library | No — consumers resolve their own |

## Dependency Version Strategy

- Use `"^1.2"` (default caret) for most deps — maximum flexibility
- Never use `"*"` — too permissive
- Pin git deps to `rev = "sha"`, not just a branch
- Set up Dependabot or Renovate for automated upgrade PRs

### Git + registry on the same dependency (Rust 1.96+)

Cargo 1.96 allows a single dependency to specify both `git` and `registry` simultaneously. Locally, the git source is used; on `cargo publish`, the registry version is recorded. Use this when you need to test an unreleased fix from a fork or branch but the consuming crate must still publish to an alternate registry with a real version pin.

```toml
[dependencies]
my-internal-crate = { version = "0.4", git = "https://github.com/acme/my-internal-crate", branch = "fix/leak", registry = "acme-private" }
```

Keep `version` set — it's what publish uses. Prefer a `rev = "<sha>"` over `branch` for reproducible local builds. Don't use this combo for normal third-party deps — it confuses the resolver story for downstream consumers.

### Cargo 1.97 warning policy and lockfile placement

Cargo 1.97 stabilizes a cache-safe warning policy. Prefer the environment form
in CI so local development remains flexible:

```bash
CARGO_BUILD_WARNINGS=deny cargo clippy --all-targets --all-features
```

For a repository-wide policy, put this in `.cargo/config.toml` (not
`Cargo.toml`):

```toml
[build]
warnings = "deny"
```

Unlike `RUSTFLAGS="-D warnings"`, changing this setting does not invalidate
compiled artifacts. The new `linker_messages` lint is outside the `warnings`
group; configure it explicitly under `[lints.rust]` if linker output must fail
CI.

Cargo 1.97 also stabilizes `resolver.lockfile-path` for read-only source trees
and adds `-m` as shorthand for `--manifest-path` after a subcommand:
`cargo check -m path/to/Cargo.toml`.

## Build Profiles

```toml
[profile.release]
opt-level = 3
lto = "fat"           # 10-20%+ runtime gains
codegen-units = 1     # more optimization, slower build
panic = "abort"       # removes unwinding machinery
```

Profiles are workspace-root-only. Library authors do not control them.

## Workspace Lint Configuration (Rust 1.74+)

Centralize lint config in the workspace root instead of per-crate `lib.rs` attributes:

```toml
# Root Cargo.toml
[workspace.lints.rust]
unsafe_code = "deny"
unreachable_pub = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
unwrap_used = "warn"
expect_used = "warn"
todo = "warn"
```

Member crates inherit:
```toml
# crates/my-service/Cargo.toml
[lints]
workspace = true
```

**DO**: Centralize lints in workspace root — one source of truth.
**DO**: Use `priority = -1` on group lints so individual overrides take effect.
**DO**: Comment every `allow` with a reason.
**DON'T**: Use `#![deny(warnings)]` in source — breaks on new rustc versions. On Rust 1.97+, use `CARGO_BUILD_WARNINGS=deny` in CI.
**DON'T**: Use `#[allow(...)]` — use `#[expect(...)]` (Rust 1.81+) which warns when stale.

## Cargo.toml Dos and Don'ts

**DO**:
- Always specify `edition = "2024"` — omitting defaults to 2015
- Always set `rust-version` (MSRV) — prevents cryptic errors for users on older toolchains
- Use `[workspace.dependencies]` to pin shared dependency versions once
- Use `publish = false` for internal crates not meant for crates.io
- Pin git deps to a `rev` (commit SHA), not just a branch name
- Use `[profile.release] overflow-checks = true` — prevents silent integer wrapping

**DON'T**:
- Use wildcard `*` versions — crates.io rejects them
- Use `>=` version ranges — causes resolver conflicts downstream
- Add dependencies you don't actually use — check with `cargo-udeps`
- Forget `features = ["derive"]` on serde or `features = ["full"]` on tokio
- Leave `[profile.release]` at defaults in production — at minimum set `lto` and `overflow-checks`
- Put `#![deny(warnings)]` in source files — use CI flags instead

## Supply Chain Security

| Tool | Purpose |
|---|---|
| `cargo-audit` | CVE scanning against RustSec database |
| `cargo-vet` | Manual audit tracking (used by Firefox, Chrome) |
| `cargo-geiger` | Unsafe code count in dependency tree |
| `cargo-semver-checks` | Detect accidental API breaking changes |

Minimum CI baseline: `cargo audit`.

## Re-Export Patterns

```rust
// Internal organization
mod error;
mod config;
mod client;

// Flat public API
pub use error::Error;
pub use config::Config;
pub use client::Client;
```

Items should be reachable via exactly one canonical path.

## Prelude Module

```rust
// src/prelude.rs
pub use crate::Config;
pub use crate::Error;
pub use crate::Result;
pub use crate::traits::MyMainTrait;

// Extension trait imported for side effect only:
pub use crate::FooExt as _;
```

Contains only re-exports. Include enough for the library's common tasks. Don't include items with common names that could clash.
