//! NDJSON output writer.
//!
//! All probe output is one JSON object per line on stdout. Writes are
//! mutex-guarded because callbacks fire on whichever worker thread the
//! probed runtime spawns — without serialisation, lines would interleave.
//!
//! Every line carries `ts` (RFC3339 nano via `time` crate) and `kind`.

use std::io::{Stdout, Write};
use std::sync::Mutex;

use serde::Serialize;
use time::OffsetDateTime;
use time::format_description::well_known::Iso8601;

pub(crate) struct NdjsonWriter {
    out: Mutex<Stdout>,
}

impl NdjsonWriter {
    pub(crate) fn new() -> Self {
        Self {
            out: Mutex::new(std::io::stdout()),
        }
    }

    pub(crate) fn emit<T: Serialize>(&self, value: &T) {
        // Build the line in a buffer first so a serialise failure doesn't
        // leave a half-written line on stdout.
        let line = match serde_json::to_string(value) {
            Ok(s) => s,
            Err(e) => format!(
                r#"{{"ts":"{}","kind":"error","msg":"output serialise failed","detail":"{}"}}"#,
                now_iso(),
                e.to_string().replace('"', "\\\""),
            ),
        };
        let mut guard = match self.out.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let _ = writeln!(*guard, "{line}");
        let _ = guard.flush();
    }
}

pub(crate) fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Iso8601::DEFAULT)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

#[derive(Serialize)]
struct ControlLine<'a> {
    ts: String,
    kind: &'static str,
    msg: &'a str,
}

pub(crate) fn control(w: &NdjsonWriter, msg: &str) {
    w.emit(&ControlLine {
        ts: now_iso(),
        kind: "control",
        msg,
    });
}

#[derive(Serialize)]
struct ErrorLine<'a> {
    ts: String,
    kind: &'static str,
    msg: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<serde_json::Value>,
}

pub(crate) fn error(w: &NdjsonWriter, msg: &str, detail: Option<serde_json::Value>) {
    w.emit(&ErrorLine {
        ts: now_iso(),
        kind: "error",
        msg,
        detail,
    });
}
