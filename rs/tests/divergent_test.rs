// The live cross-port divergence ledger.
//
// `test/spec/divergent.tsv` records each KNOWN split as the value each
// port actually produces. The Go runner asserts the `go` column and the
// TypeScript runner the `ts` column, from the same file.
//
// This runner asserts the `ts` column. The Rust port reproduces the
// TypeScript answer on every row: the engine's string lexer consults the
// `string.replace` map before the control-character class, exactly as
// the canonical lexer does, so a mapped control character is legal string
// body here too. A `rust` column would therefore duplicate `ts` on every
// row, and `ts/test/divergent.test.js` asserts the file has exactly six
// columns, so the column cannot be added without a change under `ts/`.
// Until that lands, this is the executable claim: Rust follows
// TypeScript, and a row where it stops doing so fails here, naming the
// row, which is the moment the column (and `tabnas_support::Register`)
// becomes necessary.
//
// The property that matters is kept either way: a divergence that gets
// FIXED in Go fails the Go suite as loudly as one that regresses, and the
// row must then be deleted, at which point this runner stops seeing it.

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
    let Some(replace) = spec
        .get("string")
        .and_then(|string| string.get("replace"))
        .and_then(serde_json::Value::as_object)
    else {
        return Err(format!(
            "unsupported ledger opts (extend make_from_ledger_opts): {raw}"
        ));
    };
    let mut pairs = Vec::new();
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
    Ok(make_with(move |options| {
        for (from, to) in pairs {
            options.string.replace.insert(from, to);
        }
    }))
}

#[test]
fn every_ledger_row_is_reproduced_from_the_typescript_column() {
    let spec = load_spec(spec_dir().join("divergent.tsv"), &SpecOptions::default())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        !spec.rows.is_empty(),
        "divergent.tsv has no rows; if the ledger is empty, delete the file and its runners"
    );

    let runner = Runner::new_with_row(|input, row| {
        let parser = make_from_ledger_opts(row.named("opts"))
            .map_err(|message| Failure::message(format!("{}: {message}", row.location())))?;
        parser
            .parse(input)
            .map(|value| to_value(&value))
            .map_err(to_failure)
    })
    .input("input")
    .expected("ts");

    let mut failures = Vec::new();
    for row in &spec.rows {
        let name = row.named("name");
        assert!(
            !row.named("justification").trim().is_empty(),
            "{name}: a ledger row must carry a justification"
        );
        let input = row.unesc_named("input");
        if let Err(error) = runner.check_row(row, &input, row.named("ts")) {
            failures.push(format!(
                "{name}: the Rust port no longer reproduces the TypeScript column.\n  {error}\n  \
                 If this is deliberate, the ledger needs a `rust` column (and the TypeScript \
                 runner's column count relaxed) recording what Rust now produces."
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
