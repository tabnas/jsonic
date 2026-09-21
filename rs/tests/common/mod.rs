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
