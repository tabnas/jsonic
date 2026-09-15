// Loader for the shared `test/spec/*.tsv` conformance fixtures.
//
// `@tabnas/support` has no Rust half, so this is the third independent
// implementation of the one format, beside the TypeScript runner and the
// Go one. It is therefore the loader that can DRIFT: keep it to the
// format `../../test/AGENTS.md` pins.
//
// Two rules from that file are load-bearing and easy to get wrong:
//
//   - A HEADER ROW NAMES THE COLUMNS. This repo has eight different
//     header shapes, so columns are found by name, never by position.
//   - ESCAPING APPLIES TO THE `input` COLUMN ONLY. Every other column is
//     taken as written, so `expected` is raw JSON and JSON's own escape
//     rules apply to it. Decoding both collapses one layer twice and
//     quietly changes what a row asserts.

use std::fs;
use std::path::{Path, PathBuf};

pub struct Row {
    pub file: String,
    pub line: usize,
    pub input: String,
    pub expected: String,
    /// `make()` options as a JSON object, when the fixture carries an
    /// `opts` column and the cell is not empty.
    pub opts: Option<String>,
}

/// Undo the escapes the TSV format uses for characters that cannot appear
/// raw in a tab-separated line. `input` only.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

pub fn spec_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("rs/ has a parent")
        .join("test")
        .join("spec")
}

/// A comment line is a `#` line WITH NO TAB; a data row always has one.
fn is_comment(line: &str) -> bool {
    line.trim_start().starts_with('#') && !line.contains('\t')
}

/// The column names of a fixture, from its header row.
pub fn header(file: &str) -> Vec<String> {
    let path = spec_dir().join(file);
    let body =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    for line in body.lines() {
        if line.trim().is_empty() || is_comment(line) {
            continue;
        }
        return line.split('\t').map(str::to_string).collect();
    }
    panic!("{} has no header row", path.display());
}

/// Every data row of one fixture file, in file order.
///
/// Only the `input` / `expected` (and optional `opts`) shape is returned;
/// a file with any other shape is a different kind of fixture and is
/// rejected loudly rather than silently mis-read.
pub fn rows(file: &str) -> Vec<Row> {
    let path = spec_dir().join(file);
    let body =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let mut columns: Option<Vec<String>> = None;
    let mut out = Vec::new();

    for (index, raw) in body.lines().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.trim().is_empty() || is_comment(line) {
            continue;
        }
        let cells: Vec<&str> = line.split('\t').collect();

        let Some(columns) = columns.as_ref() else {
            columns = Some(cells.iter().map(|c| c.to_string()).collect());
            continue;
        };

        let at = |name: &str| columns.iter().position(|c| c == name);
        let (Some(input_at), Some(expected_at)) = (at("input"), at("expected")) else {
            panic!(
                "{}: not an input/expected fixture (columns: {})",
                path.display(),
                columns.join(", ")
            );
        };
        let cell = |at: usize| cells.get(at).copied().unwrap_or("");

        out.push(Row {
            file: file.to_string(),
            line: index + 1,
            input: unescape(cell(input_at)),
            expected: cell(expected_at).to_string(),
            opts: at("opts")
                .map(cell)
                .filter(|o| !o.trim().is_empty())
                .map(str::to_string),
        });
    }

    assert!(!out.is_empty(), "{} has no data rows", path.display());
    out
}

/// The fixture files whose shape this runner understands: the core parse
/// corpus. The rest (list-child, lex tokens, the utility helpers, the
/// divergence ledger) are different shapes with their own runners.
pub fn files() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(spec_dir())
        .expect("the spec directory exists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".tsv"))
        // The `utility-*` family tests helper FUNCTIONS (list
        // modification, string truncation, template injection), not the
        // parser. Several carry `input`/`expected` columns, so a shape
        // filter alone lets them in -- and they then score as passes for
        // the wrong reason: `utility-modlist` reported 58/78 because the
        // parser echoes a JSON array back unchanged. A fixture that
        // passes without exercising what it names is worse than one that
        // fails. They need their own runners if this port ever grows the
        // utilities.
        .filter(|name| !name.starts_with("utility-"))
        .filter(|name| {
            let columns = header(name);
            columns.iter().any(|c| c == "input") && columns.iter().any(|c| c == "expected")
        })
        .collect();
    names.sort();
    // An empty list would run zero rows and report green having tested
    // nothing. A rename or a deletion under test/spec has to be loud.
    assert!(
        !names.is_empty(),
        "{} holds no input/expected fixtures; the parity suite would test nothing",
        spec_dir().display()
    );
    names
}
