//! Socket mode — connect to a Unix domain socket, ferry framed/raw bytes.
//!
//! Framing modes:
//! - `be32`: 4-byte big-endian u32 length prefix (gRPC-style, matches
//!   most line-of-business RPC servers).
//! - `be64`: 8-byte big-endian u64 length prefix (very-large-frame variants).
//! - `varint`: protobuf-style LEB128 length prefix (compact for tiny msgs).
//! - `none`: raw bytes through, socat-equivalent. Each stdin line becomes
//!   one `send`/`raw` payload; outbound bytes emitted in fixed-size chunks.
//!
//! Stream behaviour follows the websocat/grpcurl playbook: bounded stdin
//! reads, CancellationToken for SIGINT, graceful close on EOF.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio_stream::StreamExt;
use tokio_util::codec::{Encoder, FramedRead, LengthDelimitedCodec};
use tokio_util::sync::CancellationToken;

use crate::output::{self, NdjsonWriter};

const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;
const RAW_READ_CHUNK: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub(crate) enum Framing {
    Be32,
    Be64,
    Varint,
    None,
}

pub(crate) struct Args {
    pub(crate) path: PathBuf,
    pub(crate) framing: Framing,
    pub(crate) no_events: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum InputOp {
    /// Send `payload` as one frame (framed modes) or as bytes (raw mode).
    /// `payload` may be a JSON value (serialised before send) or a string
    /// of hex-encoded bytes via `payload_hex` for binary fuzzing.
    Send {
        #[serde(default)]
        payload: Option<Value>,
        #[serde(default)]
        payload_hex: Option<String>,
    },
    /// Raw bytes from a hex string. Identical to `send`+`payload_hex`;
    /// kept for ergonomic parity with `seam-probe ffi`.
    Raw {
        hex: String,
    },
    SleepMs {
        ms: u64,
    },
    Stop,
    #[serde(other)]
    Unknown,
}

#[derive(Serialize)]
struct RcLine {
    ts: String,
    kind: &'static str,
    op: &'static str,
    rc: i32,
    bytes_sent: usize,
}

#[derive(Serialize)]
struct InboundFrame {
    ts: String,
    kind: &'static str,
    bytes: usize,
    /// Decoded as JSON when possible; otherwise omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    payload: Option<Value>,
    /// Always emitted as hex so binary content survives.
    hex: String,
}

pub(crate) async fn run(args: Args) -> anyhow::Result<()> {
    let writer = Arc::new(NdjsonWriter::new());
    output::control(
        &writer,
        &format!(
            "connecting to {} (framing={:?})",
            args.path.display(),
            args.framing
        ),
    );

    let stream = match UnixStream::connect(&args.path).await {
        Ok(s) => s,
        Err(e) => {
            output::error(
                &writer,
                &format!("connect failed: {e}"),
                Some(serde_json::json!({ "path": args.path.display().to_string() })),
            );
            std::process::exit(2);
        }
    };
    output::control(&writer, "socket connected");

    let cancel = CancellationToken::new();
    let cancel_signal = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        cancel_signal.cancel();
    });

    match args.framing {
        Framing::None => run_raw(args, stream, writer, cancel).await,
        framed => run_framed(framed, stream, writer, cancel, args.no_events).await,
    }
}

async fn run_framed(
    framing: Framing,
    stream: UnixStream,
    writer: Arc<NdjsonWriter>,
    cancel: CancellationToken,
    no_events: bool,
) -> anyhow::Result<()> {
    let codec = match framing {
        Framing::Be32 => LengthDelimitedCodec::builder()
            .length_field_type::<u32>()
            .big_endian()
            .max_frame_length(MAX_FRAME_BYTES)
            .new_codec(),
        Framing::Be64 => LengthDelimitedCodec::builder()
            .length_field_type::<u64>()
            .big_endian()
            .max_frame_length(MAX_FRAME_BYTES)
            .new_codec(),
        Framing::Varint => {
            // tokio-util's LengthDelimitedCodec doesn't ship varint; v1
            // errors rather than silently mis-frame.
            output::error(
                &writer,
                "varint framing is not yet implemented in seam-probe v1",
                Some(serde_json::json!({
                    "workaround": "use --framing none and build the varint prefix in the payload",
                })),
            );
            std::process::exit(2);
        }
        Framing::None => unreachable!("raw routed elsewhere"),
    };

    let (read_half, mut write_half) = stream.into_split();
    let mut frame_reader = FramedRead::new(read_half, codec.clone());

    let event_writer = Arc::clone(&writer);
    let cancel_for_events = cancel.clone();
    let event_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                () = cancel_for_events.cancelled() => break,
                next = frame_reader.next() => match next {
                    Some(Ok(bytes)) => {
                        if no_events { continue; }
                        emit_inbound(&event_writer, &bytes);
                    }
                    Some(Err(e)) => {
                        output::error(&event_writer, &format!("frame read error: {e}"), None);
                        break;
                    }
                    None => {
                        output::control(&event_writer, "socket closed by peer");
                        break;
                    }
                },
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut line_no: u64 = 0;
    let mut encode_codec = codec;
    'outer: loop {
        tokio::select! {
            () = cancel.cancelled() => {
                output::control(&writer, "cancelled (SIGINT)");
                break 'outer;
            }
            line = lines.next_line() => {
                match line {
                    Ok(Some(text)) => {
                        line_no += 1;
                        if text.trim().is_empty() { continue; }
                        let op: InputOp = match serde_json::from_str(&text) {
                            Ok(o) => o,
                            Err(e) => {
                                output::error(&writer, "invalid stdin line", Some(serde_json::json!({
                                    "line_no": line_no, "line": text, "parse_error": e.to_string(),
                                })));
                                continue;
                            }
                        };
                        if matches!(op, InputOp::Stop) {
                            output::control(&writer, "stop requested via stdin");
                            break 'outer;
                        }
                        handle_framed_op(&writer, &mut encode_codec, &mut write_half, op).await;
                    }
                    Ok(None) => {
                        output::control(&writer, "stdin EOF");
                        break 'outer;
                    }
                    Err(e) => {
                        output::error(&writer, &format!("stdin read error: {e}"), None);
                        break 'outer;
                    }
                }
            }
        }
    }

    output::control(&writer, "closing socket");
    let _ = write_half.shutdown().await;
    cancel.cancel();
    let _ = event_task.await;
    Ok(())
}

async fn handle_framed_op<W>(
    w: &NdjsonWriter,
    codec: &mut LengthDelimitedCodec,
    write_half: &mut W,
    op: InputOp,
) where
    W: tokio::io::AsyncWrite + Unpin,
{
    match op {
        InputOp::Send { .. } | InputOp::Raw { .. } => {
            let Some(bytes) = build_payload_bytes(w, &op) else {
                return;
            };
            let len = bytes.len();
            let mut framed = BytesMut::new();
            if let Err(e) = codec.encode(Bytes::from(bytes), &mut framed) {
                output::error(w, &format!("frame encode failed: {e}"), None);
                return;
            }
            let rc = match write_half.write_all(&framed).await {
                Ok(()) => 0,
                Err(e) => {
                    output::error(w, &format!("frame write failed: {e}"), None);
                    -1
                }
            };
            w.emit(&RcLine {
                ts: output::now_iso(),
                kind: "rc",
                op: "send",
                rc,
                bytes_sent: len,
            });
        }
        InputOp::SleepMs { ms } => tokio::time::sleep(Duration::from_millis(ms)).await,
        InputOp::Stop => unreachable!("Stop handled in outer loop"),
        InputOp::Unknown => output::error(w, "unknown stdin op", None),
    }
}

fn build_payload_bytes(w: &NdjsonWriter, op: &InputOp) -> Option<Vec<u8>> {
    match op {
        InputOp::Send {
            payload,
            payload_hex,
        } => {
            if let Some(hex) = payload_hex.as_deref() {
                Some(decode_hex_or_error(w, hex)?)
            } else if let Some(p) = payload {
                serde_json::to_vec(p)
                    .map_err(|e| {
                        output::error(w, &format!("payload serialise failed: {e}"), None);
                    })
                    .ok()
            } else {
                Some(Vec::new())
            }
        }
        InputOp::Raw { hex } => Some(decode_hex_or_error(w, hex)?),
        _ => Some(Vec::new()),
    }
}

fn decode_hex_or_error(w: &NdjsonWriter, hex: &str) -> Option<Vec<u8>> {
    let trimmed: String = hex.chars().filter(|c| !c.is_ascii_whitespace()).collect();
    if !trimmed.len().is_multiple_of(2) {
        output::error(w, "payload_hex has odd length", None);
        return None;
    }
    let mut out = Vec::with_capacity(trimmed.len() / 2);
    let bytes = trimmed.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = hex_nibble(bytes[i]);
        let lo = hex_nibble(bytes[i + 1]);
        match (hi, lo) {
            (Some(h), Some(l)) => out.push((h << 4) | l),
            _ => {
                output::error(w, "payload_hex has non-hex chars", None);
                return None;
            }
        }
        i += 2;
    }
    Some(out)
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn emit_inbound(w: &NdjsonWriter, bytes: &[u8]) {
    let mut hex = String::with_capacity(bytes.len() * 2);
    const NIBBLE: &[u8; 16] = b"0123456789abcdef";
    for b in bytes {
        hex.push(NIBBLE[(b >> 4) as usize] as char);
        hex.push(NIBBLE[(b & 0x0f) as usize] as char);
    }
    let payload: Option<Value> = serde_json::from_slice(bytes).ok();
    w.emit(&InboundFrame {
        ts: output::now_iso(),
        kind: "frame",
        bytes: bytes.len(),
        payload,
        hex,
    });
}

async fn run_raw(
    _args: Args,
    stream: UnixStream,
    writer: Arc<NdjsonWriter>,
    cancel: CancellationToken,
) -> anyhow::Result<()> {
    let (mut read_half, mut write_half) = stream.into_split();

    let event_writer = Arc::clone(&writer);
    let cancel_for_events = cancel.clone();
    let event_task = tokio::spawn(async move {
        let mut buf = vec![0_u8; RAW_READ_CHUNK];
        loop {
            tokio::select! {
                () = cancel_for_events.cancelled() => break,
                read = read_half.read(&mut buf) => match read {
                    Ok(0) => {
                        output::control(&event_writer, "socket closed by peer");
                        break;
                    }
                    Ok(n) => emit_inbound(&event_writer, &buf[..n]),
                    Err(e) => {
                        output::error(&event_writer, &format!("read error: {e}"), None);
                        break;
                    }
                },
            }
        }
    });

    let stdin = tokio::io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut line_no: u64 = 0;
    'outer: loop {
        tokio::select! {
            () = cancel.cancelled() => {
                output::control(&writer, "cancelled (SIGINT)");
                break 'outer;
            }
            line = lines.next_line() => {
                match line {
                    Ok(Some(text)) => {
                        line_no += 1;
                        if text.trim().is_empty() { continue; }
                        let op: InputOp = match serde_json::from_str(&text) {
                            Ok(o) => o,
                            Err(e) => {
                                output::error(&writer, "invalid stdin line", Some(serde_json::json!({
                                    "line_no": line_no, "line": text, "parse_error": e.to_string(),
                                })));
                                continue;
                            }
                        };
                        if matches!(op, InputOp::Stop) {
                            output::control(&writer, "stop requested via stdin");
                            break 'outer;
                        }
                        let bytes = match build_payload_bytes(&writer, &op) {
                            Some(b) => b,
                            None => continue,
                        };
                        match op {
                            InputOp::Send { .. } | InputOp::Raw { .. } => {
                                let len = bytes.len();
                                let rc = match write_half.write_all(&bytes).await {
                                    Ok(()) => 0,
                                    Err(e) => {
                                        output::error(&writer, &format!("write failed: {e}"), None);
                                        -1
                                    }
                                };
                                writer.emit(&RcLine {
                                    ts: output::now_iso(),
                                    kind: "rc",
                                    op: "send",
                                    rc,
                                    bytes_sent: len,
                                });
                            }
                            InputOp::SleepMs { ms } => tokio::time::sleep(Duration::from_millis(ms)).await,
                            InputOp::Stop => unreachable!(),
                            InputOp::Unknown => output::error(&writer, "unknown stdin op", None),
                        }
                    }
                    Ok(None) => {
                        output::control(&writer, "stdin EOF");
                        break 'outer;
                    }
                    Err(e) => {
                        output::error(&writer, &format!("stdin read error: {e}"), None);
                        break 'outer;
                    }
                }
            }
        }
    }

    output::control(&writer, "closing socket");
    let _ = write_half.shutdown().await;
    cancel.cancel();
    let _ = event_task.await;
    Ok(())
}
