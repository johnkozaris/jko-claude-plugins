//! Inspect a shared library: list exported symbols. Cross-platform via the
//! `object` crate (Mach-O, ELF, PE).

use std::path::PathBuf;

use object::{Object, ObjectSymbol, SymbolKind};
use serde::Serialize;

use crate::output::{self, NdjsonWriter};

pub(crate) struct Args {
    pub(crate) lib: PathBuf,
}

#[derive(Serialize)]
struct SymbolLine<'a> {
    ts: String,
    kind: &'static str,
    name: &'a str,
    /// "function", "data", or "other".
    sym_kind: &'static str,
    address: u64,
    size: u64,
}

#[derive(Serialize)]
struct SummaryLine {
    ts: String,
    kind: &'static str,
    format: &'static str,
    total: usize,
    functions: usize,
    data: usize,
}

pub(crate) fn run(args: Args) -> anyhow::Result<()> {
    let writer = NdjsonWriter::new();
    let data = std::fs::read(&args.lib)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", args.lib.display()))?;
    let obj = object::File::parse(&*data)
        .map_err(|e| anyhow::anyhow!("parse {}: {e}", args.lib.display()))?;

    let format = match obj.format() {
        object::BinaryFormat::Elf => "elf",
        object::BinaryFormat::MachO => "mach-o",
        object::BinaryFormat::Pe => "pe",
        object::BinaryFormat::Coff => "coff",
        object::BinaryFormat::Wasm => "wasm",
        object::BinaryFormat::Xcoff => "xcoff",
        _ => "unknown",
    };

    let mut total = 0_usize;
    let mut functions = 0_usize;
    let mut data_count = 0_usize;

    for symbol in obj.symbols() {
        if !symbol.is_global() || !symbol.is_definition() || symbol.is_undefined() {
            continue;
        }
        let Ok(name) = symbol.name() else { continue };
        if name.is_empty() {
            continue;
        }
        let sym_kind = match symbol.kind() {
            SymbolKind::Text => "function",
            SymbolKind::Data => "data",
            _ => "other",
        };
        match sym_kind {
            "function" => functions += 1,
            "data" => data_count += 1,
            _ => {}
        }
        total += 1;
        writer.emit(&SymbolLine {
            ts: output::now_iso(),
            kind: "symbol",
            name,
            sym_kind,
            address: symbol.address(),
            size: symbol.size(),
        });
    }

    writer.emit(&SummaryLine {
        ts: output::now_iso(),
        kind: "summary",
        format,
        total,
        functions,
        data: data_count,
    });
    Ok(())
}
