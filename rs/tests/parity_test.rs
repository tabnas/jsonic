// The shared conformance fixtures, every standard-shaped one.
//
// `test/spec/*.tsv` is the parity contract: the TypeScript suite
// (`ts/test/alignment.test.js`), the Go suite (`go/alignment_test.go`,
// `go/feature_tsv_test.go`) and this file run the same rows, and each
// builds the parser for a file the same way. A row green in one runtime
// and red in another is a failure, not a discrepancy.
//
// The bespoke-shaped fixtures (the three-column list-child files, the
// token-stream corpus, the divergence register and the utility files)
// have their own runners beside this one; `registration_test.rs` is the
// tripwire that every file is run by one of them.

mod common;

use tabnas::{CommentDef, Options, Tabnas};
use tabnas_jsonic::{make, make_json, make_with};
use tabnas_support::{equal_value, format_value, load_spec, parse_expect, report, SpecOptions};

use common::{run_fixture, spec_dir, to_failure, to_value};

/// The fixtures the stock parser runs: `jsonic_test.go`'s
/// `parserTSVFiles` plus the `alignment-*` files `alignment_test.go`
/// runs with a bare `Make()`.
const STOCK: &[&str] = &[
    "alignment-empty.tsv",
    "alignment-errors.tsv",
    "alignment-map-merge.tsv",
    "alignment-number-prefix-separator.tsv",
    "alignment-number-text.tsv",
    "alignment-safe-key.tsv",
    "alignment-structure.tsv",
    "alignment-utf8.tsv",
    "alignment-values.tsv",
    "comma-implicit-comma.tsv",
    "comma-optional-comma.tsv",
    "feature-debug-cases.tsv",
    "feature-implicit-map.tsv",
    "feature-implicit-object.tsv",
    "feature-nested-space-pairs.tsv",
    "fv-arrays.tsv",
    "fv-comma.tsv",
    "fv-deep.tsv",
    "fv-drop-outs.tsv",
    "fv-numbers.tsv",
    "fv-subobj.tsv",
    "fv-types.tsv",
    "fv-works.tsv",
    "happy.tsv",
    "jsonic-basic-array-tree.tsv",
    "jsonic-basic-json.tsv",
    "jsonic-basic-mixed-tree.tsv",
    "jsonic-basic-object-tree.tsv",
    "jsonic-funky-keys.tsv",
    "jsonic-process-array.tsv",
    "jsonic-process-implicit-object.tsv",
    "jsonic-process-mixed-nodes.tsv",
    "jsonic-process-object-tree.tsv",
    "jsonic-process-scalars.tsv",
    "jsonic-process-text.tsv",
    "jsonic-process-whitespace.tsv",
    "lex-errors.tsv",
];

fn comment(line: bool, start: &str, end: &str, lex: bool) -> CommentDef {
    CommentDef {
        line,
        start: start.to_string(),
        end: end.to_string(),
        lex,
        suffixes: Vec::new(),
        suffix_matcher: None,
        eat_line: false,
    }
}

fn with_comment(name: &str, def: CommentDef) -> impl FnOnce(&mut Options) {
    let name = name.to_string();
    move |options: &mut Options| {
        options.comment.definitions.insert(name, def);
    }
}

/// The option-bearing fixtures, each with the parser its Go runner builds:
/// (file, parser). `lex-errors` runs three times, as Go runs it, so a lex
/// error is not masked by a generic `unexpected` in any grammar variant.
fn option_fixtures() -> Vec<(&'static str, Tabnas)> {
    vec![
        (
            "lex-errors.tsv",
            make_with(|o| o.rule.exclude = "jsonic,imp".into()),
        ),
        (
            "lex-errors.tsv",
            make_with(|o| o.rule.exclude = "jsonic,imp,comma".into()),
        ),
        (
            "exclude-strict-json.tsv",
            make_with(|o| o.rule.exclude = "jsonic,imp".into()),
        ),
        (
            "exclude-strict-json-errors.tsv",
            make_with(|o| o.rule.exclude = "jsonic,imp".into()),
        ),
        (
            "exclude-comma.tsv",
            make_with(|o| o.rule.exclude = "comma".into()),
        ),
        (
            "exclude-comma-errors.tsv",
            make_with(|o| o.rule.exclude = "comma".into()),
        ),
        (
            "rule-finish-errors.tsv",
            make_with(|o| o.rule.finish = false),
        ),
        (
            "string-allow-control.tsv",
            make_with(|o| o.string.allow_control = true),
        ),
        (
            "include-json.tsv",
            make_with(|o| o.rule.include = "json".into()),
        ),
        (
            "include-json-errors.tsv",
            make_with(|o| o.rule.include = "json".into()),
        ),
        // Strict-JSON mode: unlike rule.include alone, `make_json` also
        // tightens the lexer, so the number grammar is exactly RFC 8259
        // and `\xHH` escapes are rejected.
        ("alignment-strict-json-mode.tsv", make_json()),
        ("alignment-strict-json-mode-errors.tsv", make_json()),
        // Comment suffixes: a hash line comment that terminates at a
        // custom `@@` suffix, and a block comment that also accepts `!!`.
        (
            "feature-comment-suffix-line.tsv",
            make_with(|o| {
                let hash = o.comment.definitions.get_mut("hash").expect("hash def");
                hash.suffixes = vec!["@@".to_string()];
            }),
        ),
        (
            "feature-comment-suffix-block.tsv",
            make_with(|o| {
                let multi = o.comment.definitions.get_mut("multi").expect("multi def");
                multi.suffixes = vec!["!!".to_string()];
            }),
        ),
        // Comment def option merge: adding a def keeps the default markers,
        // a partial def for a default name keeps the fields it leaves
        // alone, removing a def removes only that marker, an explicit
        // block conversion turns a line def into a block one, and a def
        // for a new name without `lex` is inactive.
        (
            "feature-comment-def-add.tsv",
            make_with(with_comment("semi", comment(true, ";", "", true))),
        ),
        (
            "feature-comment-def-tweak.tsv",
            make_with(|o| {
                o.comment
                    .definitions
                    .get_mut("hash")
                    .expect("hash def")
                    .eat_line = true;
            }),
        ),
        (
            "feature-comment-def-remove.tsv",
            make_with(|o| {
                o.comment.definitions.shift_remove("hash");
            }),
        ),
        (
            "feature-comment-def-remove-errors.tsv",
            make_with(|o| {
                o.comment.definitions.shift_remove("hash");
            }),
        ),
        (
            "feature-comment-def-block-conv.tsv",
            make_with(with_comment("hash", comment(false, "#", "@@", true))),
        ),
        (
            "feature-comment-def-nolex-errors.tsv",
            make_with(with_comment("semi", comment(true, ";", "", false))),
        ),
        // These need non-default options.
        ("feature-list-pair.tsv", make_with(|o| o.list.pair = true)),
        ("feature-map-child.tsv", make_with(|o| o.map.child = true)),
        (
            "feature-map-child-deep.tsv",
            make_with(|o| {
                o.map.child = true;
                o.list.child = true;
            }),
        ),
    ]
}

#[test]
fn stock_fixtures() {
    let mut failures = Vec::new();
    for file in STOCK {
        failures.extend(run_fixture(file, make()));
    }
    report(Ok(failures));
}

#[test]
fn option_bearing_fixtures() {
    let mut failures = Vec::new();
    for (file, parser) in option_fixtures() {
        failures.extend(run_fixture(file, parser));
    }
    report(Ok(failures));
}

/// The three-column list-child fixtures: `input`, `expected_array`,
/// `expected_child`. With `list.child` on, a bare `:value` element lands
/// on the list's `child` rather than among its elements, so the parse
/// result is a `ListRef` whose `value` and `child` are compared
/// separately, as `go/feature_tsv_test.go`'s `runListChildTSV` does.
fn run_list_child(file: &str, parser: &Tabnas) -> Vec<String> {
    let spec = load_spec(spec_dir().join(file), &SpecOptions::default())
        .unwrap_or_else(|error| panic!("{error}"));
    let mut failures = Vec::new();
    for row in &spec.rows {
        let at = row.location();
        let input = row.unesc_named("input");
        let want_array = match parse_expect(row.named("expected_array")) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("{at}: expected_array: {error}"));
                continue;
            }
        };
        let want_child = match parse_expect(row.named("expected_child")) {
            Ok(value) => value,
            Err(error) => {
                failures.push(format!("{at}: expected_child: {error}"));
                continue;
            }
        };
        let (got_array, got_child) = match parser.parse(&input) {
            Err(error) => {
                failures.push(format!(
                    "{at}: parse({input:?}) failed: {}",
                    to_failure(error)
                ));
                continue;
            }
            Ok(tabnas::Value::ListRef(list)) => (
                to_value(&tabnas::Value::array(list.value.clone())),
                list.child
                    .as_deref()
                    .map_or(tabnas_support::Value::Undefined, to_value),
            ),
            Ok(other @ tabnas::Value::Array(_)) => {
                (to_value(&other), tabnas_support::Value::Undefined)
            }
            Ok(other) => {
                failures.push(format!(
                    "{at}: parse({input:?}) is not a list: {}",
                    format_value(&to_value(&other))
                ));
                continue;
            }
        };
        if !equal_value(&got_array, &want_array) {
            failures.push(format!(
                "{at}: parse({input:?}) elements\n  got:      {}\n  expected: {}",
                format_value(&got_array),
                format_value(&want_array)
            ));
        }
        if !equal_value(&got_child, &want_child) {
            failures.push(format!(
                "{at}: parse({input:?}) child\n  got:      {}\n  expected: {}",
                format_value(&got_child),
                format_value(&want_child)
            ));
        }
    }
    failures
}

#[test]
fn list_child_fixtures() {
    let child = make_with(|o| o.list.child = true);
    let child_pair = make_with(|o| {
        o.list.child = true;
        o.list.pair = true;
    });
    let mut failures = Vec::new();
    failures.extend(run_list_child("feature-list-child.tsv", &child));
    failures.extend(run_list_child("feature-list-child-deep.tsv", &child));
    failures.extend(run_list_child("feature-list-child-pair.tsv", &child_pair));
    failures.extend(run_list_child(
        "feature-list-child-pair-deep.tsv",
        &child_pair,
    ));
    report(Ok(failures));
}
