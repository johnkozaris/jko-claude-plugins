//! seam-probe — generic NDJSON probe for embedded-runtime seams.
//!
//! Two real transports + symbol-table inspection + self-documentation.
//! No app-specific knowledge in the binary; manifests describe app
//! surfaces externally.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod ffi;
mod inspect;
mod manifest;
mod output;
#[cfg(unix)]
mod socket;
mod vocab;

#[derive(Parser)]
#[command(
    name = "seam-probe",
    version,
    about = "NDJSON probe for FFI dylibs and Unix-socket seams.",
    long_about = "\
seam-probe is a generic, manifest-driven CLI for probing embedded-runtime\n\
seams. It carries no knowledge about any specific app: the FFI mode reads\n\
a manifest describing the library's symbol surface; the socket mode takes\n\
a framing flag. NDJSON in / NDJSON out everywhere.\n\
\n\
Run `seam-probe vocab` to see the stdin/stdout contract."
)]
struct Cli {
    #[command(subcommand)]
    mode: Mode,
}

#[derive(Subcommand)]
enum Mode {
    /// Drive a shared library via dlopen + a manifest-described surface.
    Ffi {
        /// Path to the shared library (.so / .dylib / .dll).
        #[arg(long)]
        lib: PathBuf,
        /// Path to the manifest JSON describing the FFI surface.
        #[arg(long)]
        manifest: PathBuf,
        /// Suppress callback events; errors, return codes, and controls remain.
        #[arg(long, default_value_t = false)]
        no_events: bool,
        /// Total milliseconds allowed for stop plus callback draining.
        #[arg(long, default_value_t = 2000)]
        shutdown_grace_ms: u64,
    },

    /// Connect to a Unix domain socket and ferry framed/raw bytes.
    #[cfg(unix)]
    Socket {
        /// Path to the Unix socket.
        #[arg(long)]
        path: PathBuf,
        /// Framing mode.
        #[arg(long, value_enum, default_value_t = socket::Framing::Be32)]
        framing: socket::Framing,
        /// Suppress frame events; errors, return codes, and controls remain.
        #[arg(long, default_value_t = false)]
        no_events: bool,
    },

    /// Dump exported symbols from a shared library (Mach-O / ELF / PE).
    Inspect {
        #[arg(long)]
        lib: PathBuf,
    },

    /// Print the NDJSON I/O contract.
    Vocab,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.mode {
        Mode::Ffi {
            lib,
            manifest,
            no_events,
            shutdown_grace_ms,
        } => {
            ffi::run(ffi::Args {
                lib,
                manifest,
                no_events,
                shutdown_grace_ms,
            })
            .await
        }
        #[cfg(unix)]
        Mode::Socket {
            path,
            framing,
            no_events,
        } => {
            socket::run(socket::Args {
                path,
                framing,
                no_events,
            })
            .await
        }
        Mode::Inspect { lib } => inspect::run(inspect::Args { lib }),
        Mode::Vocab => {
            vocab::run();
            Ok(())
        }
    }
}
