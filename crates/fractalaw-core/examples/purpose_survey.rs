//! Read-only survey: published purpose (PURPOSE-CLASSIFICATION.md) for
//! `section_id \t text` lines on stdin, before the stem rule.
//! Prints `section_id \t purpose`.
use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        let Some((sid, text)) = line.split_once('\t') else { continue };
        let record = fractalaw_core::taxa::parse_v2(text, None);
        writeln!(out, "{sid}\t{}", record.purpose).unwrap();
    }
}
