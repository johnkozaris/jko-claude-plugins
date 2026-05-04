//! Indexed callback trampolines.
//!
//! For each callback signature kind we pre-generate `MAX_CALLBACK_FIELDS`
//! distinct `extern "C" fn` items. Each item has a unique address and a
//! compile-time index `I` it uses to look up its per-instance state in
//! a static slot array.
//!
//! This is the standard pattern for runtime-pluggable C-ABI callbacks
//! without `libffi` runtime trampoline generation. Documented in the
//! `libloading` ecosystem and used by tools like `dyncall` for callbacks.

use std::ffi::{CStr, c_char, c_void};
use std::sync::{Arc, LazyLock, Mutex};

use seq_macro::seq;
use serde::Serialize;
use serde_json::Value;

use crate::manifest::MAX_CALLBACK_FIELDS;
use crate::output::{NdjsonWriter, now_iso};

/// Per-trampoline-instance state. Set by `bind_*_slot`, read by trampolines.
pub(crate) struct Slot {
    pub(crate) field: String,
    pub(crate) writer: Arc<NdjsonWriter>,
    pub(crate) no_events: bool,
}

static JSON_SLOTS: LazyLock<[Mutex<Option<Slot>>; MAX_CALLBACK_FIELDS]> =
    LazyLock::new(|| std::array::from_fn(|_| Mutex::new(None)));

static JSON_SID_SLOTS: LazyLock<[Mutex<Option<Slot>>; MAX_CALLBACK_FIELDS]> =
    LazyLock::new(|| std::array::from_fn(|_| Mutex::new(None)));

static RAW_SEQ_SLOTS: LazyLock<[Mutex<Option<Slot>>; MAX_CALLBACK_FIELDS]> =
    LazyLock::new(|| std::array::from_fn(|_| Mutex::new(None)));

#[derive(Serialize)]
struct EventLine<'a> {
    ts: String,
    kind: &'static str,
    callback: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_raw: Option<&'a str>,
}

#[derive(Serialize)]
struct JsonWithSidLine<'a> {
    ts: String,
    kind: &'static str,
    callback: &'a str,
    session_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payload_raw: Option<&'a str>,
}

#[derive(Serialize)]
struct TerminalDataLine<'a> {
    ts: String,
    kind: &'static str,
    callback: &'a str,
    session_id: &'a str,
    seq: u64,
    len: usize,
    /// Hex-encoded bytes. Hex (not base64) so Claude can grep for known
    /// byte sequences without a decode step.
    hex: String,
}

fn emit_json_event(slot: &Slot, json: &[u8]) {
    if slot.no_events {
        return;
    }
    let payload: Option<Value> = serde_json::from_slice(json).ok();
    let payload_raw: Option<String> = if payload.is_some() {
        None
    } else {
        Some(String::from_utf8_lossy(json).into_owned())
    };
    slot.writer.emit(&EventLine {
        ts: now_iso(),
        kind: "event",
        callback: &slot.field,
        payload,
        payload_raw: payload_raw.as_deref(),
    });
}

fn emit_raw_seq_event(slot: &Slot, sid_ptr: *const c_char, seq: u64, bytes: &[u8]) {
    if slot.no_events {
        return;
    }
    let sid = if sid_ptr.is_null() {
        ""
    } else {
        // SAFETY: sid_ptr is a NUL-terminated C string supplied by the
        // probed runtime. Treat invalid UTF-8 as empty rather than
        // panicking to keep the probe robust against malformed callers.
        unsafe { CStr::from_ptr(sid_ptr) }.to_str().unwrap_or("")
    };
    let mut hex = String::with_capacity(bytes.len() * 2);
    const NIBBLE: &[u8; 16] = b"0123456789abcdef";
    for b in bytes {
        hex.push(NIBBLE[(b >> 4) as usize] as char);
        hex.push(NIBBLE[(b & 0x0f) as usize] as char);
    }
    slot.writer.emit(&TerminalDataLine {
        ts: now_iso(),
        kind: "raw_with_seq",
        callback: &slot.field,
        session_id: sid,
        seq,
        len: bytes.len(),
        hex,
    });
}

fn emit_json_with_sid(slot: &Slot, sid_ptr: *const c_char, json: &[u8]) {
    if slot.no_events {
        return;
    }
    let sid = if sid_ptr.is_null() {
        ""
    } else {
        // SAFETY: sid_ptr is a NUL-terminated C string supplied by the runtime.
        unsafe { CStr::from_ptr(sid_ptr) }.to_str().unwrap_or("")
    };
    let payload: Option<Value> = serde_json::from_slice(json).ok();
    let payload_raw: Option<String> = if payload.is_some() {
        None
    } else {
        Some(String::from_utf8_lossy(json).into_owned())
    };
    slot.writer.emit(&JsonWithSidLine {
        ts: now_iso(),
        kind: "json_with_sid",
        callback: &slot.field,
        session_id: sid,
        payload,
        payload_raw: payload_raw.as_deref(),
    });
}

/// Trampoline that should never be reached. Installed in unused slots of
/// the over-allocated callback struct. If a runtime calls into this slot
/// it means the manifest under-declared the runtime's struct size.
unsafe extern "C" fn unused_callback_slot() {
    eprintln!(
        "seam-probe: runtime called callback slot beyond manifest \
         declarations. The manifest's `callback_struct` field count is \
         smaller than the runtime's actual struct. Aborting to avoid UB."
    );
    std::process::abort();
}

seq!(I in 0..64 {
    unsafe extern "C" fn json_trampoline_~I(
        json: *const u8,
        len: usize,
        _user: *mut c_void,
    ) {
        if json.is_null() || len == 0 {
            return;
        }
        // SAFETY: caller (the probed runtime) guarantees `json` points to
        // `len` valid bytes for the duration of the call.
        let bytes = unsafe { std::slice::from_raw_parts(json, len) };
        let guard = match JSON_SLOTS[I].lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(slot) = guard.as_ref() {
            emit_json_event(slot, bytes);
        }
    }

    unsafe extern "C" fn json_with_sid_trampoline_~I(
        sid: *const c_char,
        json: *const u8,
        len: usize,
        _user: *mut c_void,
    ) {
        let bytes: &[u8] = if json.is_null() || len == 0 {
            &[]
        } else {
            // SAFETY: caller guarantees `json` points to `len` valid bytes.
            unsafe { std::slice::from_raw_parts(json, len) }
        };
        let guard = match JSON_SID_SLOTS[I].lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(slot) = guard.as_ref() {
            emit_json_with_sid(slot, sid, bytes);
        }
    }

    unsafe extern "C" fn raw_seq_trampoline_~I(
        sid: *const c_char,
        seq: u64,
        bytes: *const u8,
        len: usize,
        _user: *mut c_void,
    ) {
        let slice: &[u8] = if bytes.is_null() || len == 0 {
            &[]
        } else {
            // SAFETY: caller guarantees `bytes` points to `len` valid
            // bytes for the duration of the call.
            unsafe { std::slice::from_raw_parts(bytes, len) }
        };
        let guard = match RAW_SEQ_SLOTS[I].lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(slot) = guard.as_ref() {
            emit_raw_seq_event(slot, sid, seq, slice);
        }
    }
});

/// Returns `*const c_void` function pointers to the indexed trampolines.
/// Exposed so the FFI driver can build the callback struct.
pub(crate) fn json_trampoline_ptr(index: usize) -> *const c_void {
    seq!(I in 0..64 {
        const JSON_PTRS: [unsafe extern "C" fn(*const u8, usize, *mut c_void); 64] = [
            #( json_trampoline_~I, )*
        ];
    });
    JSON_PTRS[index] as *const c_void
}

pub(crate) fn json_with_sid_trampoline_ptr(index: usize) -> *const c_void {
    seq!(I in 0..64 {
        const JSON_SID_PTRS: [unsafe extern "C" fn(
            *const c_char,
            *const u8,
            usize,
            *mut c_void,
        ); 64] = [
            #( json_with_sid_trampoline_~I, )*
        ];
    });
    JSON_SID_PTRS[index] as *const c_void
}

pub(crate) fn raw_seq_trampoline_ptr(index: usize) -> *const c_void {
    seq!(I in 0..64 {
        const RAW_SEQ_PTRS: [unsafe extern "C" fn(
            *const c_char,
            u64,
            *const u8,
            usize,
            *mut c_void,
        ); 64] = [
            #( raw_seq_trampoline_~I, )*
        ];
    });
    RAW_SEQ_PTRS[index] as *const c_void
}

pub(crate) fn unused_slot_ptr() -> *const c_void {
    unused_callback_slot as *const c_void
}

pub(crate) fn bind_json_slot(index: usize, slot: Slot) {
    let mut guard = match JSON_SLOTS[index].lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some(slot);
}

pub(crate) fn bind_json_with_sid_slot(index: usize, slot: Slot) {
    let mut guard = match JSON_SID_SLOTS[index].lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some(slot);
}

pub(crate) fn bind_raw_seq_slot(index: usize, slot: Slot) {
    let mut guard = match RAW_SEQ_SLOTS[index].lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    *guard = Some(slot);
}
