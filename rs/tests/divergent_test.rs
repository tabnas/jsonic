// The live cross-port divergence ledger.
//
// `test/spec/divergent.tsv` records each KNOWN split as the value each
// port actually produces. The Go runner asserts the `go` column and the
// TypeScript runner the `ts` column, from the same file.
//
// This runner asserts the `rust` column. Every column is read by HEADER
// NAME in all three runners, which is what let the `rust` column go in
// without any of them moving an index.
//
// The Rust port reproduces the TypeScript answer on every row today: the
// engine's string lexer consults the `string.replace` map before the
// control-character class, exactly as the canonical lexer does, so a
// mapped control character is legal string body here too. The column
// records that as a measured fact rather than a claim, and a row where
// Rust stops agreeing fails here, naming the row.
//
// `tabnas_support::Register` is not used, deliberately: it refuses a row
// whose runtime cells all agree, and the ledger keeps one such row on
// purpose (`string-replace-printable`, the control row that stops a fix
// to the row above it being read as a regression here).
//
// The property that matters is kept either way: a divergence that gets
// FIXED in any port fails that port's suite as loudly as one that
// regresses, and the row must then be deleted, at which point this runner
// stops seeing it.

mod common;

use tabnas_jsonic::make_with;
use tabnas_support::{load_spec, Failure, Runner, SpecOptions, Value};

use common::{spec_dir, to_failure, to_value};

/// Build the instance a ledger row asks for. The ledger spells options as
/// JSON so the SAME text drives every port; only the option shapes the
/// ledger actually uses are supported, and an unknown one is an error
/// rather than a silent stock parser, since a row that quietly ran
/// without its options would assert the wrong thing.
fn make_from_ledger_opts(raw: &str) -> Result<tabnas::Tabnas, String> {
    if raw == "-" || raw.is_empty() {
        return Ok(tabnas_jsonic::make());
    }
    let spec: serde_json::Value =
        serde_json::from_str(raw).map_err(|error| format!("bad opts {raw:?}: {error}"))?;

    let mut pairs = Vec::new();
    let mut known = false;
    if let Some(replace) = spec
        .get("string")
        .and_then(|string| string.get("replace"))
        .and_then(serde_json::Value::as_object)
    {
        for (key, value) in replace {
            let mut chars = key.chars();
            let (Some(from), None) = (chars.next(), chars.next()) else {
                return Err(format!("string.replace key must be one char: {key:?}"));
            };
            let Some(to) = value.as_str() else {
                return Err(format!("string.replace value must be a string: {value}"));
            };
            pairs.push((from, to.to_string()));
        }
        known = true;
    }

    let mut sep = None;
    if let Some(value) = spec.get("number").and_then(|number| number.get("sep")) {
        let Some(text) = value.as_str() else {
            return Err(format!("number.sep must be a string: {value}"));
        };
        sep = Some(text.to_string());
        known = true;
    }

    if !known {
        return Err(format!(
            "unsupported ledger opts (extend make_from_ledger_opts): {raw}"
        ));
    }

    Ok(make_with(move |options| {
        for (from, to) in pairs {
            options.string.replace.insert(from, to);
        }
        if let Some(sep) = sep {
            options.number.sep = Some(sep);
        }
    }))
}

/// Every runtime column the ledger is expected to carry. Named rather
/// than inferred from the header, so a column lost in an edit fails here
/// instead of silently leaving a port unasserted.
const RUNTIMES: &[&str] = &["go", "ts", "rust"];

#[test]
fn every_ledger_row_is_reproduced_from_the_rust_column() {
    let spec = load_spec(spec_dir().join("divergent.tsv"), &SpecOptions::default())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        !spec.rows.is_empty(),
        "divergent.tsv has no rows; if the ledger is empty, delete the file and its runners"
    );
    for want in RUNTIMES {
        assert!(
            spec.header.iter().any(|name| name == want),
            "divergent.tsv has no {want:?} column (header: {})",
            spec.header.join(", ")
        );
    }

    let runner = Runner::new_with_row(|input, row| {
        let parser = make_from_ledger_opts(row.named("opts"))
            .map_err(|message| Failure::message(format!("{}: {message}", row.location())))?;
        parser
            .parse(input)
            .map(|value| to_value(&value))
            .map_err(to_failure)
    })
    .input("input")
    .expected("rust");

    let mut failures = Vec::new();
    for row in &spec.rows {
        let name = row.named("name");
        assert!(
            !row.named("justification").trim().is_empty(),
            "{name}: a ledger row must carry a justification"
        );
        let input = row.unesc_named("input");
        if let Err(error) = runner.check_row(row, &input, row.named("rust")) {
            failures.push(format!(
                "{name}: the Rust side of the ledger is stale.\n  {error}\n  \
                 If Rust now AGREES with every other runtime column, the divergence is fixed: \
                 delete this row. If the change is deliberate, update the `rust` cell."
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn a_ledger_row_needs_its_options_to_mean_anything() {
    // A row whose options this runner cannot build must fail, not run
    // against a stock parser.
    let Err(error) = make_from_ledger_opts(r#"{"number":{"hex":false}}"#) else {
        panic!("an unsupported option shape must not build a parser");
    };
    assert!(error.contains("unsupported ledger opts"), "{error}");
    let stock = make_from_ledger_opts("-").unwrap_or_else(|error| panic!("{error}"));
    assert!(matches!(
        stock.parse("a:1").map(|value| to_value(&value)),
        Ok(Value::Object(_))
    ));
}
