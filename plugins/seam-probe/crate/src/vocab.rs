//! `seam-probe vocab` — print the NDJSON I/O contract verbatim.

const VOCAB: &str = include_str!("../VOCAB.md");

pub(crate) fn run() {
    print!("{VOCAB}");
}
