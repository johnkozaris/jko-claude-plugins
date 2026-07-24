//! FFI mode entry point.
//!
//! Workflow:
//! 1. Load the manifest.
//! 2. `dlopen` the library (RTLD_NOW; never `dlclose`).
//! 3. Resolve start/stop + lane + op symbols.
//! 4. Bind callback trampoline slots.
//! 5. Build a process-lifetime callback table and pass its pointer to start.
//! 6. Drive an NDJSON stdin loop dispatching `send`/`call` messages onto
//!    the resolved symbols.
//! 7. On EOF / `stop` / SIGINT: run lifecycle stop on a dedicated thread,
//!    enforce the total grace deadline, then exit.
//!
//! Safety notes:
//! - We never call `dlclose`. The `Library` is leaked into a `'static`
//!   reference. This is required because the runtime may have spawned
//!   worker threads holding pointers into the library's `.text` segment;
//!   unloading while a callback is in flight is undefined behaviour.
//! - The start ABI takes a pointer to a callback struct. The backing table is
//!   over-allocated to 64 pointers and leaked for the process lifetime so a
//!   runtime may retain it safely. Trailing slots use an aborting sentinel.

use std::ffi::{CString, c_char, c_void};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use libloading::{Library, Symbol};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_stream::StreamExt;
use tokio_util::codec::{FramedRead, LinesCodec, LinesCodecError};
use tokio_util::sync::CancellationToken;

use crate::manifest::{self, CallbackKind, OpKind};
use crate::output::{self, NdjsonWriter};

mod trampolines;
use trampolines::Slot;

/// Maximum bytes accepted in a stdin `send` payload.
const MAX_PAYLOAD_BYTES: usize = 8 * 1024 * 1024;
const MAX_INPUT_LINE_BYTES: usize = MAX_PAYLOAD_BYTES;

pub(crate) struct Args {
    pub(crate) lib: PathBuf,
    pub(crate) manifest: PathBuf,
    pub(crate) no_events: bool,
    pub(crate) shutdown_grace_ms: u64,
}

type StartFn = unsafe extern "C" fn(cb_struct: *const c_void, user: *mut c_void) -> *mut c_void;
type StopFn = unsafe extern "C" fn(handle: *mut c_void);
type LaneFn = unsafe extern "C" fn(handle: *mut c_void, json: *const u8, len: usize) -> i32;
type OpHandleCstrFn = unsafe extern "C" fn(handle: *mut c_void, arg: *const c_char) -> i32;
type OpHandleOnlyFn = unsafe extern "C" fn(handle: *mut c_void) -> i32;

enum ResolvedOp<'a> {
    HandleCstr(Symbol<'a, OpHandleCstrFn>),
    HandleOnly(Symbol<'a, OpHandleOnlyFn>),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum InputOp {
    /// Call a manifest-declared lane. `payload` is forwarded as JSON bytes.
    Send {
        lane: String,
        payload: Value,
    },
    /// Call a manifest-declared ad-hoc op (e.g. `terminal_connect`).
    /// Field is `name` not `op` to avoid collision with the tag.
    Call {
        name: String,
        #[serde(default)]
        arg: Option<String>,
    },
    SleepMs {
        ms: u64,
    },
    Stop,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
struct RcLine<'a> {
    ts: String,
    kind: &'static str,
    op: &'static str,
    name: &'a str,
    rc: i32,
}

pub(crate) async fn run(args: Args) -> anyhow::Result<()> {
    let writer = Arc::new(NdjsonWriter::new());
    output::control(
        &writer,
        &format!(
            "loading manifest {} for library {}",
            args.manifest.display(),
            args.lib.display(),
        ),
    );
    let manifest = manifest::load(&args.manifest)?;
    if let Some(label) = manifest.label.as_deref() {
        output::control(&writer, &format!("manifest label: {label}"));
    }

    // SAFETY: We never call `dlclose`. The library is leaked into a
    // 'static reference so callbacks dispatched from runtime-owned worker
    // threads always find live code mapped in our address space until
    // the process exits.
    let lib: &'static Library = match unsafe { Library::new(&args.lib) } {
        Ok(l) => Box::leak(Box::new(l)),
        Err(e) => {
            output::error(
                &writer,
                &format!("dlopen failed: {e}"),
                Some(serde_json::json!({ "lib": args.lib.display().to_string() })),
            );
            std::process::exit(2);
        }
    };

    // Resolve lifecycle symbols.
    let start_fn: Symbol<'static, StartFn> =
        unsafe { lib.get(manifest.lifecycle.start_symbol.as_bytes()) }.map_err(|e| {
            anyhow::anyhow!(
                "resolve start symbol {}: {e}",
                manifest.lifecycle.start_symbol
            )
        })?;
    let stop_fn: Symbol<'static, StopFn> =
        unsafe { lib.get(manifest.lifecycle.stop_symbol.as_bytes()) }.map_err(|e| {
            anyhow::anyhow!(
                "resolve stop symbol {}: {e}",
                manifest.lifecycle.stop_symbol
            )
        })?;

    // Resolve and index lane symbols.
    let mut lane_table: Vec<(String, Symbol<'static, LaneFn>)> = Vec::new();
    for lane in &manifest.lanes {
        let sym = unsafe { lib.get::<LaneFn>(lane.symbol.as_bytes()) }
            .map_err(|e| anyhow::anyhow!("resolve lane symbol {}: {e}", lane.symbol))?;
        lane_table.push((lane.name.clone(), sym));
    }

    // Resolve op symbols.
    let mut op_table: Vec<(String, ResolvedOp<'static>)> = Vec::new();
    for op in &manifest.ops {
        let resolved = match op.kind {
            OpKind::HandleCstr => ResolvedOp::HandleCstr(
                unsafe { lib.get::<OpHandleCstrFn>(op.symbol.as_bytes()) }
                    .map_err(|e| anyhow::anyhow!("resolve op symbol {}: {e}", op.symbol))?,
            ),
            OpKind::HandleOnly => ResolvedOp::HandleOnly(
                unsafe { lib.get::<OpHandleOnlyFn>(op.symbol.as_bytes()) }
                    .map_err(|e| anyhow::anyhow!("resolve op symbol {}: {e}", op.symbol))?,
            ),
        };
        op_table.push((op.name.clone(), resolved));
    }

    // Build callback struct: assign trampolines per manifest order; fill
    // unused slots with the panic-on-call sentinel so a runtime that
    // expects more fields than the manifest declared aborts loudly
    // rather than invoking uninitialised memory.
    let mut cb_struct: [*const c_void; manifest::MAX_CALLBACK_FIELDS] =
        [trampolines::unused_slot_ptr(); manifest::MAX_CALLBACK_FIELDS];
    let mut json_idx = 0_usize;
    let mut json_sid_idx = 0_usize;
    let mut raw_seq_idx = 0_usize;
    for (i, field) in manifest.callback_struct.iter().enumerate() {
        match field.kind {
            CallbackKind::Json => {
                trampolines::bind_json_slot(
                    json_idx,
                    Slot {
                        field: field.name.clone(),
                        writer: Arc::clone(&writer),
                        no_events: args.no_events,
                    },
                );
                cb_struct[i] = trampolines::json_trampoline_ptr(json_idx);
                json_idx += 1;
            }
            CallbackKind::JsonWithSid => {
                trampolines::bind_json_with_sid_slot(
                    json_sid_idx,
                    Slot {
                        field: field.name.clone(),
                        writer: Arc::clone(&writer),
                        no_events: args.no_events,
                    },
                );
                cb_struct[i] = trampolines::json_with_sid_trampoline_ptr(json_sid_idx);
                json_sid_idx += 1;
            }
            CallbackKind::RawWithSeq => {
                trampolines::bind_raw_seq_slot(
                    raw_seq_idx,
                    Slot {
                        field: field.name.clone(),
                        writer: Arc::clone(&writer),
                        no_events: args.no_events,
                    },
                );
                cb_struct[i] = trampolines::raw_seq_trampoline_ptr(raw_seq_idx);
                raw_seq_idx += 1;
            }
        }
    }

    output::control(
        &writer,
        &format!(
            "starting runtime via `{}` with {} callbacks ({} json, {} json_with_sid, \
             {} raw_with_seq), {} lanes, {} ops",
            manifest.lifecycle.start_symbol,
            manifest.callback_struct.len(),
            json_idx,
            json_sid_idx,
            raw_seq_idx,
            lane_table.len(),
            op_table.len(),
        ),
    );

    // Keep the callback table alive because runtimes may retain its pointer
    // after start returns.
    let cb_struct = Box::leak(Box::new(cb_struct));
    // SAFETY: start_fn uses the documented pointer-based ABI. cb_struct is a
    // process-lifetime allocation whose prefix matches the manifest-declared
    // function-pointer fields.
    let handle: *mut c_void =
        unsafe { (*start_fn)(cb_struct.as_ptr().cast(), std::ptr::null_mut()) };
    if handle.is_null() {
        output::error(&writer, "start returned null handle", None);
        std::process::exit(3);
    }
    output::control(&writer, "runtime started; ready for stdin");

    let cancel = CancellationToken::new();
    let cancel_for_signal = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_for_signal.cancel();
    });

    let stdin = tokio::io::stdin();
    let mut lines = FramedRead::new(stdin, LinesCodec::new_with_max_length(MAX_INPUT_LINE_BYTES));
    let mut line_no: u64 = 0;
    'outer: loop {
        tokio::select! {
            () = cancel.cancelled() => {
                output::control(&writer, "cancelled (SIGINT)");
                break 'outer;
            }
            line = lines.next() => {
                match line {
                    Some(Ok(text)) => {
                        line_no += 1;
                        if text.trim().is_empty() {
                            continue;
                        }
                        let op: InputOp = match serde_json::from_str(&text) {
                            Ok(o) => o,
                            Err(e) => {
                                output::error(
                                    &writer,
                                    "invalid stdin line",
                                    Some(serde_json::json!({
                                        "line_no": line_no,
                                        "line": text,
                                        "parse_error": e.to_string(),
                                    })),
                                );
                                continue;
                            }
                        };
                        if matches!(op, InputOp::Stop) {
                            output::control(&writer, "stop requested via stdin");
                            break 'outer;
                        }
                        // SAFETY: handle and resolved symbols live for
                        // the lifetime of `run`; manifest declares the
                        // signatures we call through.
                        unsafe {
                            handle_op(&writer, handle, &lane_table, &op_table, op).await;
                        }
                    }
                    Some(Err(LinesCodecError::MaxLineLengthExceeded)) => {
                        line_no += 1;
                        output::error(
                            &writer,
                            "stdin line exceeds maximum length",
                            Some(serde_json::json!({
                                "line_no": line_no,
                                "max_bytes": MAX_INPUT_LINE_BYTES,
                            })),
                        );
                        break 'outer;
                    }
                    Some(Err(LinesCodecError::Io(e))) => {
                        output::error(&writer, &format!("stdin read error: {e}"), None);
                        break 'outer;
                    }
                    None => {
                        output::control(&writer, "stdin EOF");
                        break 'outer;
                    }
                }
            }
        }
    }

    stop_runtime(
        &writer,
        *stop_fn,
        handle,
        Duration::from_millis(args.shutdown_grace_ms),
    )
    .await;
    output::control(&writer, "exiting");
    std::process::exit(0)
}

async fn stop_runtime(w: &NdjsonWriter, stop_fn: StopFn, handle: *mut c_void, grace: Duration) {
    output::control(
        w,
        &format!("stopping runtime with {} ms total grace", grace.as_millis()),
    );

    let started = Instant::now();
    let handle_addr = handle as usize;
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    let stop_thread = std::thread::Builder::new()
        .name(String::from("seam-probe-stop"))
        .spawn(move || {
            // SAFETY: handle came from the matching start function and the
            // loaded library remains mapped for the process lifetime.
            unsafe { stop_fn(handle_addr as *mut c_void) };
            let _ = done_tx.send(());
        });

    if let Err(e) = stop_thread {
        output::error(
            w,
            &format!("failed to start runtime stop thread: {e}"),
            None,
        );
        return;
    }

    match tokio::time::timeout(grace, done_rx).await {
        Ok(Ok(())) => {
            let remaining = grace.saturating_sub(started.elapsed());
            if !remaining.is_zero() {
                output::control(
                    w,
                    &format!(
                        "runtime stopped; waiting {} ms for callbacks to drain",
                        remaining.as_millis()
                    ),
                );
                tokio::time::sleep(remaining).await;
            }
        }
        Ok(Err(_)) => {
            output::error(
                w,
                "runtime stop thread ended without reporting completion",
                None,
            );
        }
        Err(_) => {
            output::error(
                w,
                "runtime stop exceeded shutdown grace; forcing process exit",
                Some(serde_json::json!({ "grace_ms": grace.as_millis() })),
            );
        }
    }
}

async unsafe fn handle_op(
    w: &NdjsonWriter,
    handle: *mut c_void,
    lane_table: &[(String, Symbol<'static, LaneFn>)],
    op_table: &[(String, ResolvedOp<'static>)],
    op: InputOp,
) {
    match op {
        InputOp::Send { lane, payload } => {
            let bytes = match serde_json::to_vec(&payload) {
                Ok(b) => b,
                Err(e) => {
                    output::error(w, &format!("payload serialise failed: {e}"), None);
                    return;
                }
            };
            if bytes.len() > MAX_PAYLOAD_BYTES {
                output::error(
                    w,
                    "payload exceeds MAX_PAYLOAD_BYTES",
                    Some(serde_json::json!({
                        "len": bytes.len(),
                        "max": MAX_PAYLOAD_BYTES,
                    })),
                );
                return;
            }
            let entry = lane_table.iter().find(|(n, _)| *n == lane);
            let Some((_, sym)) = entry else {
                output::error(w, "unknown lane", Some(serde_json::json!({ "lane": lane })));
                return;
            };
            // SAFETY: lane symbol resolved with the canonical
            // (handle, json, len) -> i32 signature; we pass non-null,
            // length-bounded bytes and a live handle.
            let rc = unsafe { (**sym)(handle, bytes.as_ptr(), bytes.len()) };
            w.emit(&RcLine {
                ts: output::now_iso(),
                kind: "rc",
                op: "send",
                name: &lane,
                rc,
            });
        }
        InputOp::Call { name: op_name, arg } => {
            let entry = op_table.iter().find(|(n, _)| *n == op_name);
            let Some((_, resolved)) = entry else {
                output::error(w, "unknown op", Some(serde_json::json!({ "op": op_name })));
                return;
            };
            let rc = match resolved {
                ResolvedOp::HandleCstr(sym) => {
                    let arg_str = arg.as_deref().unwrap_or("");
                    let cstring = match CString::new(arg_str) {
                        Ok(c) => c,
                        Err(_) => {
                            output::error(w, "op arg contained interior NUL", None);
                            return;
                        }
                    };
                    // SAFETY: signature matches manifest declaration.
                    unsafe { (**sym)(handle, cstring.as_ptr()) }
                }
                ResolvedOp::HandleOnly(sym) => {
                    // SAFETY: signature matches manifest declaration.
                    unsafe { (**sym)(handle) }
                }
            };
            w.emit(&RcLine {
                ts: output::now_iso(),
                kind: "rc",
                op: "call",
                name: &op_name,
                rc,
            });
        }
        InputOp::SleepMs { ms } => tokio::time::sleep(Duration::from_millis(ms)).await,
        InputOp::Stop => unreachable!("Stop handled in outer loop"),
        InputOp::Unknown => output::error(w, "unknown stdin op", None),
    }
}
