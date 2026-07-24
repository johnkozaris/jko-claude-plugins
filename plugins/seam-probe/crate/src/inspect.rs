//! Inspect a shared library: list exported symbols. Cross-platform via the
//! `object` crate (Mach-O, ELF, PE).

use std::collections::{BTreeMap, HashSet};
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

#[derive(Clone, Copy)]
struct SymbolMetadata {
    kind: SymbolKind,
    size: u64,
    is_global: bool,
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

    let mut metadata = BTreeMap::new();
    for symbol in obj.symbols().chain(obj.dynamic_symbols()) {
        if !symbol.is_definition() || symbol.is_undefined() {
            continue;
        }
        let Ok(name) = symbol.name() else { continue };
        if name.is_empty() {
            continue;
        }
        metadata
            .entry((String::from(name), symbol.address()))
            .or_insert(SymbolMetadata {
                kind: symbol.kind(),
                size: symbol.size(),
                is_global: symbol.is_global(),
            });
    }

    let mut seen = HashSet::new();
    let mut emit_symbol = |name: &str, address: u64, size: u64, symbol_kind: SymbolKind| {
        if !seen.insert((String::from(name), address)) {
            return;
        }
        let sym_kind = match symbol_kind {
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
            address,
            size,
        });
    };

    let exports = obj
        .exports()
        .map_err(|e| anyhow::anyhow!("read exports from {}: {e}", args.lib.display()))?;
    if exports.is_empty() {
        for ((name, address), meta) in &metadata {
            if meta.is_global {
                emit_symbol(name, *address, meta.size, meta.kind);
            }
        }
    } else {
        for export in exports {
            let Ok(name) = std::str::from_utf8(export.name()) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            let metadata = metadata.get(&(String::from(name), export.address()));
            emit_symbol(
                name,
                export.address(),
                metadata.map_or(0, |meta| meta.size),
                metadata.map_or(SymbolKind::Unknown, |meta| meta.kind),
            );
        }
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
