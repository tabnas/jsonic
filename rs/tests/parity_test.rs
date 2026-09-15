// The shared conformance fixtures: the scoreboard for this port.
//
// Reports per-file counts so a run says WHERE the port stands, not just
// that it is not finished.

mod common;

use common::spec;
use tabnas::{GrammarSpec, Tabnas};

fn parser_for(opts: Option<&str>) -> Tabnas {
    let mut parser = tabnas_jsonic::make();
    if let Some(opts) = opts {
        let document = serde_json::json!({
            "v": 2,
            "options": serde_json::from_str::<serde_json::Value>(opts)
                .unwrap_or_else(|e| panic!("fixture opts are not JSON: {opts} ({e})")),
        });
        let spec = GrammarSpec::from_value(document).expect("opts document is valid");
        parser.grammar(&spec).expect("opts apply");
    }
    parser
}

/// serde_json's rendering of the engine's value, for textual comparison
/// with the fixture's raw-JSON `expected` cell.
fn render(value: &tabnas::Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}

#[test]
fn spec() {
    let mut total = 0usize;
    let mut passed = 0usize;
    let mut report = Vec::new();

    for file in spec::files() {
        let rows = spec::rows(&file);
        let mut ok = 0usize;
        let mut first_failures = Vec::new();

        for row in &rows {
            total += 1;
            let result = parser_for(row.opts.as_deref()).parse(&row.input);
            let got = match &result {
                Ok(value) => render(value),
                Err(error) => format!("ERROR:{}", error.code),
            };

            let want = &row.expected;
            let agree = if let Some(code) = want.strip_prefix("ERROR:") {
                matches!(&result, Err(error) if error.code == code)
            } else {
                match (&result, serde_json::from_str::<serde_json::Value>(want)) {
                    (Ok(value), Ok(expected)) => {
                        serde_json::to_string(value).ok() == serde_json::to_string(&expected).ok()
                    }
                    _ => false,
                }
            };

            if agree {
                ok += 1;
                passed += 1;
            } else if first_failures.len() < 3 {
                first_failures.push(format!(
                    "      {}:{} {:?} -> {}, want {}",
                    row.file, row.line, row.input, got, want
                ));
            }
        }

        report.push(format!(
            "  {:>4}/{:<4} {}{}",
            ok,
            rows.len(),
            file,
            if first_failures.is_empty() {
                String::new()
            } else {
                format!("\n{}", first_failures.join("\n"))
            }
        ));
    }

    println!("\n=== jsonic rs: shared fixture scoreboard ===");
    for line in &report {
        println!("{line}");
    }
    println!("\n  TOTAL {passed}/{total}\n");

    assert_eq!(passed, total, "{} of {total} fixture rows fail", total - passed);
}
