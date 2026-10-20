//! Compares skrifa with FreeType and HarfBuzz over a set of fonts.
//!
//! ```text
//! crater dump <fonts.txt> <ppem>...                      > skrifa.txt
//! crater compare <oracle.txt> <skrifa.txt> [key=value]... > result.json
//! ```
//!
//! `fonts.txt` has one `<id> <path>` line per font. A ppem of 0 means font
//! units. The `key=value` pairs are recorded in the result, to say which
//! fontations ref and oracle versions produced it.

mod compare;
mod dump;
mod record;

use std::collections::BTreeMap;
use std::io::{BufWriter, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("dump") if args.len() >= 3 => run_dump(&args[1], &args[2..]),
        Some("compare") if args.len() >= 3 => run_compare(&args[1], &args[2], &args[3..]),
        _ => {
            eprintln!("usage: crater dump <fonts.txt> <ppem>...");
            eprintln!("       crater compare <oracle.txt> <skrifa.txt> [key=value]...");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("crater: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run_dump(list: &str, sizes: &[String]) -> Result<(), String> {
    let sizes: Vec<u32> = sizes
        .iter()
        .map(|s| s.parse().map_err(|_| format!("bad ppem {s}")))
        .collect::<Result<_, _>>()?;
    let list = std::fs::read_to_string(list).map_err(|e| format!("{list}: {e}"))?;
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in list.lines() {
        let Some((id, path)) = line.split_once(' ') else {
            continue;
        };
        let data = std::fs::read(path).map_err(|e| format!("{path}: {e}"))?;
        dump::dump_font(&mut out, id, data, &sizes).map_err(|e| e.to_string())?;
    }
    out.flush().map_err(|e| e.to_string())
}

fn run_compare(oracle: &str, skrifa: &str, meta: &[String]) -> Result<(), String> {
    let read = |path: &str| std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"));
    let oracle = record::parse(&read(oracle)?);
    let skrifa = record::parse(&read(skrifa)?);
    let meta: BTreeMap<String, String> = meta
        .iter()
        .filter_map(|pair| pair.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let report = compare::compare(&oracle, &skrifa, meta);
    serde_json::to_writer_pretty(std::io::stdout().lock(), &report).map_err(|e| e.to_string())
}
