// Every shared fixture must be run by this suite.
//
// test/spec/ looked like a cross-port corpus and largely was not: at one
// point 39 of its 64 files were referenced by neither runner. A corpus
// nothing executes reports green while measuring nothing, which is the
// failure mode the whole directory exists to prevent. The Go and
// TypeScript suites gate this in `go/spec_registration_test.go`; this is
// the Rust half of the same gate.
//
// Fixtures needing a bespoke runner (extra columns, non-parse semantics)
// are listed in BESPOKE_SHAPE with the reason and the test that runs them.
// That list is the ONLY escape, and it is asserted to be accurate: an entry
// that is in fact standard-shaped fails too, so the exemption cannot
// outlive its justification.

mod common;

use std::fs;

use common::spec_dir;

/// Fixtures whose column shape the generic parity runner cannot express.
/// Each is run by its own dedicated test; the value is the reason.
const BESPOKE_SHAPE: &[(&str, &str)] = &[
    (
        "feature-list-child",
        "3 cols: input, expected_array, expected_child; run by parity_test.rs list_child_fixtures",
    ),
    (
        "feature-list-child-deep",
        "3 cols: input, expected_array, expected_child; run by parity_test.rs list_child_fixtures",
    ),
    (
        "feature-list-child-pair",
        "3 cols: input, expected_array, expected_child; run by parity_test.rs list_child_fixtures",
    ),
    (
        "feature-list-child-pair-deep",
        "3 cols: input, expected_array, expected_child; run by parity_test.rs list_child_fixtures",
    ),
    (
        "lex",
        "2 cols: input, token stream; asserts TOKENS not values, so it is run by lex_test.rs",
    ),
    (
        "divergent",
        "6 cols: name, opts, input, go, ts, justification; the parity-debt ledger, \
         each port asserts its own column, so it is run by divergent_test.rs",
    ),
    (
        "utility-deep",
        "5 cols: arg1..arg4, expected; util.deep, not a parse; run by utility_test.rs",
    ),
    (
        "utility-modlist",
        "3 cols: input, opts, expected; util.modlist, not a parse; run by utility_test.rs",
    ),
    (
        "utility-str",
        "3 cols: input, maxlen, expected; util.str, not a parse; run by utility_test.rs",
    ),
    (
        "utility-strinject",
        "3 cols: template, values, expected; util.strinject, not a parse; run by utility_test.rs",
    ),
];

fn header(name: &str) -> String {
    let path = spec_dir().join(format!("{name}.tsv"));
    let body =
        fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    body.lines().next().unwrap_or_default().to_string()
}

#[test]
fn every_spec_fixture_is_registered_in_the_rust_runner() {
    let parity = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/parity_test.rs"))
        .expect("the parity runner is readable");

    let mut names: Vec<String> = fs::read_dir(spec_dir())
        .expect("the spec directory lists")
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter_map(|name| name.strip_suffix(".tsv").map(str::to_string))
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "no fixtures under {}",
        spec_dir().display()
    );

    let mut problems = Vec::new();
    for name in &names {
        if let Some((_, reason)) = BESPOKE_SHAPE.iter().find(|(exempt, _)| exempt == name) {
            // The exemption must still be true: a standard 2-column
            // input/expected file has no business being exempt.
            if header(name) == "input\texpected" {
                problems.push(format!(
                    "{name} is exempt as {reason:?} but has the standard shape; \
                     remove it from BESPOKE_SHAPE and register it"
                ));
            }
            continue;
        }
        if !parity.contains(&format!("\"{name}.tsv\"")) {
            problems.push(format!(
                "{name}.tsv is not registered in the Rust runner (add it to \
                 tests/parity_test.rs, or a BESPOKE_SHAPE entry saying why not)"
            ));
        }
    }
    for (exempt, _) in BESPOKE_SHAPE {
        if !names.iter().any(|name| name == exempt) {
            problems.push(format!(
                "{exempt} is exempt in BESPOKE_SHAPE but there is no such fixture"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
