//! Manifest schema for FFI mode.
//!
//! A manifest tells `seam-probe` how to drive an unfamiliar shared library
//! that follows the "callback struct + opaque handle + JSON command lanes"
//! pattern. The probe carries no app-specific knowledge; everything app-
//! specific lives in the manifest the user (or Claude) supplies.
//!
//! The manifest is *not* a binding. It only describes the **shape** of
//! exported symbols the probe needs to call (start, stop, lanes) and the
//! callback struct's field order + per-field signature kind.

use std::path::PathBuf;

use serde::Deserialize;

/// Schema version. Bumped on breaking changes to manifest layout.
pub(crate) const MANIFEST_SCHEMA_VERSION: u32 = 2;

/// Maximum number of fields in a callback struct. Raise if needed; the
/// probe pre-generates this many indexed trampolines per kind. 64 covers
/// every real-world FFI surface we've encountered.
pub(crate) const MAX_CALLBACK_FIELDS: usize = 64;

#[derive(Debug, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) schema_version: u32,

    /// Optional human-readable label echoed in control output.
    #[serde(default)]
    pub(crate) label: Option<String>,

    pub(crate) lifecycle: Lifecycle,

    /// Callback struct field order. `seam-probe` constructs the struct in
    /// this exact order, so it must match the runtime's C declaration.
    #[serde(default)]
    pub(crate) callback_struct: Vec<CallbackField>,

    /// Send lanes — `(handle, json_ptr, json_len) -> i32`.
    #[serde(default)]
    pub(crate) lanes: Vec<Lane>,

    /// Other ad-hoc operations that don't fit the json-lane shape.
    #[serde(default)]
    pub(crate) ops: Vec<Op>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Lifecycle {
    /// Symbol returning a handle. Signature: `(*const callbacks_struct, *mut user) -> *mut handle`.
    pub(crate) start_symbol: String,
    /// Symbol releasing a handle. Signature: `(*mut handle) -> ()`.
    pub(crate) stop_symbol: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CallbackField {
    /// Field name (cosmetic; used to label emitted events).
    pub(crate) name: String,
    /// Signature kind. Determines which trampoline pool the probe draws
    /// from. New kinds require a code change.
    pub(crate) kind: CallbackKind,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CallbackKind {
    /// `(json: *const u8, len: usize, user: *mut c_void)`. Most lanes.
    Json,
    /// `(sid: *const c_char, json: *const u8, len: usize, user: *mut c_void)`.
    /// Used for control-frame lanes scoped to a session id (e.g. terminal control).
    JsonWithSid,
    /// `(sid: *const c_char, seq: u64, bytes: *const u8, len: usize, user: *mut c_void)`.
    /// Used for hot-byte-stream lanes (e.g. terminal output).
    RawWithSeq,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Lane {
    /// Lane name (used as the `lane` field in stdin ops).
    pub(crate) name: String,
    /// Symbol exported by the library.
    pub(crate) symbol: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Op {
    /// Op name (used as the `op` field in stdin ops).
    pub(crate) name: String,
    /// Symbol exported by the library.
    pub(crate) symbol: String,
    /// Argument shape.
    pub(crate) kind: OpKind,
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OpKind {
    /// `(handle, *const c_char) -> i32`. C string argument (e.g. session id).
    HandleCstr,
    /// `(handle) -> i32`.
    HandleOnly,
}

pub(crate) fn load(path: &PathBuf) -> anyhow::Result<Manifest> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("read manifest {}: {e}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("parse manifest {}: {e}", path.display()))?;

    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        if manifest.schema_version == 1 {
            anyhow::bail!(
                "manifest schema_version 1 used the removed by-value start ABI; \
                 migrate to schema_version 2 and expose \
                 `start(const callbacks_t*, void*) -> void*`"
            );
        }
        anyhow::bail!(
            "manifest schema_version {} not supported (probe expects {})",
            manifest.schema_version,
            MANIFEST_SCHEMA_VERSION
        );
    }
    if manifest.callback_struct.len() > MAX_CALLBACK_FIELDS {
        anyhow::bail!(
            "manifest has {} callback fields; probe limit is {}",
            manifest.callback_struct.len(),
            MAX_CALLBACK_FIELDS
        );
    }
    Ok(manifest)
}
