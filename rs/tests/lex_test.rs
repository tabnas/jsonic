// The cross-port TOKEN-STREAM corpus.
//
// Every other shared fixture asserts a decoded VALUE. That is one level
// too late for the defect class this project keeps hitting: a run lexing
// #TX in one port and #NR in another, with the value difference only a
// downstream symptom. `lex.tsv` pins the token stream itself.
//
// Render convention, shared with `go/lexspec_test.go` and
// `ts/test/lexspec.test.js` and NOTHING else:
//
//     <name>;<sI>;<len>;<row>x<col>[;<val>]   space-separated, in order
//
// val is omitted for tokens whose value is not meaningful (#ZZ, #BD,
// fixed punctuation). It is the token's own JSON, so a number that lexes
// differently shows up here even when the parsed value would coincide.
// Non-finite numbers use the corpus markers, because the JSON encoders
// disagree about how to fail on them.

mod common;

use tabnas::lexer::Lexer;
use tabnas::{Token, Value, TIN_NR, TIN_ST, TIN_TX, TIN_VL};
use tabnas_support::{load_spec, SpecOptions};

use common::{js_number, spec_dir};

fn render_token(token: &Token, fixed: &tabnas::Options) -> String {
    let base = format!(
        "{};{};{};{}x{}",
        token.name.as_str(),
        token.site.si,
        token.len,
        token.site.ri,
        token.site.ci
    );
    // #ZZ (end of source) and fixed punctuation carry no meaningful value.
    // Go gives #ZZ an empty map and TS gives it undefined, so rendering it
    // would encode a representation difference as a token difference.
    let valued = matches!(token.tin, TIN_NR | TIN_ST | TIN_TX | TIN_VL)
        && !fixed.fixed.tokens.values().any(|f| f.tin == token.tin);
    if !valued || token.val.is_undefined() {
        return base;
    }
    let value = match &token.val {
        Value::Number(number) if number.is_infinite() && *number > 0.0 => {
            "\"@@Infinity\"".to_string()
        }
        Value::Number(number) if number.is_infinite() => "\"@@-Infinity\"".to_string(),
        Value::Number(number) if number.is_nan() => "\"@@NaN\"".to_string(),
        Value::Number(number) => js_number(*number),
        other => serde_json::to_string(&other.to_json()).expect("a token value is JSON"),
    };
    format!("{base};{value}")
}

/// Lex `src` with a stock jsonic instance and render the stream, stopping
/// at end of source or the first bad token, as the other two runners do.
fn dump_tokens(src: &str) -> String {
    let parser = tabnas_jsonic::make();
    let options = parser.config();
    let mut lexer = Lexer::new(src, options.clone());
    let mut out = Vec::new();
    for _ in 0..500 {
        match lexer.next_token() {
            Ok(token) => {
                let end = token.name.as_str() == "#ZZ";
                out.push(render_token(&token, &options));
                if end {
                    break;
                }
            }
            Err(error) => {
                out.push(format!(
                    "#BD;{};{};{}x{}",
                    error.pos,
                    error.src.len(),
                    error.row,
                    error.col
                ));
                break;
            }
        }
    }
    out.join(" ")
}

#[test]
fn token_streams_match_the_shared_corpus() {
    let spec = load_spec(spec_dir().join("lex.tsv"), &SpecOptions::default())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(!spec.rows.is_empty(), "lex.tsv has no rows");
    let mut failures = Vec::new();
    for row in &spec.rows {
        let input = row.unesc_named("input");
        let want = row.named("tokens");
        let got = dump_tokens(&input);
        if got != want {
            failures.push(format!(
                "{}: token stream differs for {:?}\n  got:  {got}\n  want: {want}",
                row.location(),
                row.named("input")
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} token stream(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn the_number_renderer_spells_numbers_as_javascript_does() {
    // The corpus writes numbers the way `JSON.stringify` does, which is
    // ECMAScript `Number::toString`: pinned here so a drift in the helper
    // reads as a helper failure rather than a lexer one.
    for (number, want) in [
        (255.0, "255"),
        (1000.0, "1000"),
        (0.5, "0.5"),
        (-1.5, "-1.5"),
        (18446744073709552000.0, "18446744073709552000"),
        (9223372036854776000.0, "9223372036854776000"),
        (1e21, "1e+21"),
        (1e-7, "1e-7"),
        (0.000001, "0.000001"),
        (1.5e-8, "1.5e-8"),
        (0.0, "0"),
        (123456789.123, "123456789.123"),
    ] {
        assert_eq!(js_number(number), want, "{number}");
    }
}
