// Shared test helpers. Cargo compiles this module into EVERY integration
// test binary, so an item only one binary uses is dead code in the
// others; the allow keeps that from being a warning rather than hiding
// anything real.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use tabnas::Tabnas;
use tabnas_support::{find_spec_dir, Failure, Runner, Value};

/// The shared `test/spec` directory, found by walking up from the crate
/// rather than by counting `..` hops.
pub fn spec_dir() -> PathBuf {
    find_spec_dir(Some(Path::new(env!("CARGO_MANIFEST_DIR"))))
        .expect("a test/spec directory above rs/")
}

/// An engine value as the fixture data model, through JSON. `Undefined`
/// and non-finite numbers become `null`, which is what every fixture in
/// this repository expects of them: jsonic's vocabulary is JSON's.
pub fn to_value(value: &tabnas::Value) -> Value {
    Value::from(value.to_json())
}

/// A parse error as the runner's failure: the code the fixture pins, and
/// the rendered report for the failure message.
pub fn to_failure(error: tabnas::TabnasError) -> Failure {
    Failure::new(error.code.clone())
        .at(error.row, error.col)
        .with_message(error.to_string())
}

/// A runner over one parser instance.
pub fn runner(parser: Tabnas) -> Runner {
    Runner::new(move |input| {
        parser
            .parse(input)
            .map(|value| to_value(&value))
            .map_err(to_failure)
    })
}

/// Run one fixture file through `parser`, returning every failing row.
pub fn run_fixture(file: &str, parser: Tabnas) -> Vec<String> {
    runner(parser)
        .run_file(spec_dir().join(file))
        .unwrap_or_else(|error| vec![error.0])
}

// ---------------------------------------------------------------------------
// A parse from a reader (`Tabnas::parse_reader`, admin ADR-30)
// ---------------------------------------------------------------------------

/// A reader that hands its bytes over at most `size` at a time.
pub struct Pieces<'a> {
    bytes: &'a [u8],
    at: usize,
    size: usize,
}

impl<'a> Pieces<'a> {
    pub fn new(text: &'a str, size: usize) -> Self {
        Pieces {
            bytes: text.as_bytes(),
            at: 0,
            size,
        }
    }
}

impl std::io::Read for Pieces<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.size.min(buf.len()).min(self.bytes.len() - self.at);
        buf[..n].copy_from_slice(&self.bytes[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

/// `parser` with `lex.window` set, as a grammar built on jsonic sets it on
/// the whole parser it makes. jsonic's own constructors leave it off
/// (`reader_test.rs`), since they are the base of those grammars.
pub fn windowed(mut parser: Tabnas) -> Tabnas {
    parser
        .set_options(|options| options.lex.window = true)
        .expect("lex.window sets");
    parser
}

/// What a parse shows: its value, or its error's code, position, row,
/// column and token text. An error's excerpt is left out: a parse from a
/// reader shows a long line only as far back as the window reaches
/// (tabnas/parser `DIVERGENCE.md`), the one difference allowed.
pub fn outcome(parsed: Result<tabnas::Value, tabnas::TabnasError>) -> String {
    match parsed {
        Ok(value) => format!("ok {value:?}"),
        Err(e) => format!(
            "err {} at {} {}:{} {:?}",
            e.code, e.pos, e.row, e.col, e.src
        ),
    }
}

/// The pieces a reader hands its bytes over in.
pub const PIECES: [usize; 3] = [1, 3, 64];

/// The windows (margin and block, in characters) a reader is read under
/// besides the engine's own: a few characters, so that every matcher, and
/// the strict-number check, meets the window's edge.
pub const WINDOWS: [(usize, usize); 3] = [(1, 1), (3, 2), (64, 64)];

/// `input` parsed by `parser` whole, and from a reader in each of
/// [`PIECES`], under the engine's window and under each of [`WINDOWS`]:
/// how many reader parses were compared, and where they differ.
pub fn same_from_reader(parser: &Tabnas, at: &str, input: &str) -> (usize, Vec<String>) {
    let whole = outcome(parser.parse(input));
    let mut compared = 0;
    let mut failures = Vec::new();
    for size in PIECES {
        let mut check = |how: String, read: String| {
            compared += 1;
            if read != whole {
                failures.push(format!(
                    "{at}: {input:?} from a reader in pieces of {size}{how}\n  whole:  {whole}\n  reader: {read}"
                ));
            }
        };
        check(
            String::new(),
            outcome(parser.parse_reader(Pieces::new(input, size))),
        );
        for (margin, block) in WINDOWS {
            let sizes = tabnas::WindowSizes {
                margin,
                block,
                chunk: size,
            };
            check(
                format!(", margin {margin}, block {block}"),
                outcome(parser.parse_reader_with_sizes(Pieces::new(input, size), sizes)),
            );
        }
    }
    (compared, failures)
}

/// Every row of `file`, parsed by `parser` with `lex.window` set, whole
/// and from a reader ([`same_from_reader`]).
pub fn fixture_from_reader(file: &str, parser: Tabnas) -> (usize, Vec<String>) {
    let parser = windowed(parser);
    let spec = tabnas_support::load_spec(
        spec_dir().join(file),
        &tabnas_support::SpecOptions::default(),
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let mut compared = 0;
    let mut failures = Vec::new();
    for row in &spec.rows {
        let (n, found) = same_from_reader(&parser, &row.location(), &row.unesc_named("input"));
        compared += n;
        failures.extend(found);
    }
    (compared, failures)
}

/// ECMAScript `Number.prototype.toString`, which is how the shared
/// token-stream fixture spells a number: the shortest round-trip digits,
/// no exponent between 1e-7 and 1e21, and an exponent outside that.
pub fn js_number(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number.is_infinite() {
        return if number > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    if number == 0.0 {
        return "0".to_string();
    }
    let sign = if number < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest digits that round-trip, as `d.ddde<exp>`.
    let scientific = format!("{:e}", number.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("the exponent form always carries an e");
    let exponent: i32 = exponent.parse().expect("a decimal exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let k = digits.len() as i32;
    let n = exponent + 1;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        let (whole, fraction) = digits.split_at(n as usize);
        format!("{whole}.{fraction}")
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let (first, rest) = digits.split_at(1);
        let exponent_sign = if n - 1 < 0 { "-" } else { "+" };
        if rest.is_empty() {
            format!("{first}e{exponent_sign}{}", (n - 1).abs())
        } else {
            format!("{first}.{rest}e{exponent_sign}{}", (n - 1).abs())
        }
    };
    format!("{sign}{body}")
}
