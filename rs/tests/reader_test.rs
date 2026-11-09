// A parse from a reader (`Tabnas::parse_reader`, admin ADR-30).
//
// jsonic's constructors and plugin leave `lex.window` off. `make()` is the
// base the relaxed grammars build on (css, ini, json5, toml, yaml, zon and
// csv start from it), and each of them declares the window on its own
// whole parser once its own matchers read through it; a window declared
// here would reach grammars whose matchers do not yet.
//
// What jsonic installs reads the source only through the window all the
// same. The one hook here that reads the text, the strict-number check of
// `make_json()`, reads the literal through `Lexer::window` and asks the
// window to fill where the literal runs to its end. `parity_test.rs` and
// `divergent_test.rs` hold every fixture row to that, each parser with the
// window set, from a reader under windows of a few characters; this file
// holds the token-stream corpus, numbers at the window's edge, a literal
// longer than the window and a long document to it.

mod common;

use std::fmt::Write;

use common::{fixture_from_reader, outcome, same_from_reader, windowed, Pieces};
use tabnas_jsonic::{empty, jsonic, make, make_json, plugin};

#[test]
fn jsonic_declares_no_window() {
    assert!(!make().options.lex.window, "make()");
    assert!(!make_json().options.lex.window, "make_json()");
    assert!(!empty().options.lex.window, "empty()");
    let mut installed = tabnas::Tabnas::new();
    jsonic(&mut installed).expect("jsonic installs");
    assert!(!installed.options.lex.window, "jsonic()");
    let mut used = tabnas::Tabnas::new();
    used.use_plugin(plugin(), None)
        .expect("the plugin installs");
    assert!(!used.options.lex.window, "plugin()");
}

/// The token-stream corpus's inputs (`lex.tsv`, which `lex_test.rs`
/// lexes), parsed by a stock parser with the window set.
#[test]
fn the_token_corpus_parses_the_same_from_a_reader() {
    let (compared, failures) = fixture_from_reader("lex.tsv", make());
    assert!(compared > 0, "lex.tsv has no rows");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Numbers in strict JSON, under windows of a few characters, so that the
/// window's edge cuts each literal at every place: the check judges the
/// literal the whole text shows it, valid or not.
///
/// Where the check skips the number matcher on a literal the edge cut short
/// (`1.` of `1.5`), the keyword matcher after it reads on to the edge and
/// has the token cut again over a wider window, the check with it. With
/// that matcher off nothing after the check reads on, and the check has to
/// read the literal whole itself: the second parser holds it to that.
#[test]
fn strict_numbers_parse_the_same_from_a_reader() {
    let mut keywords_off = make_json();
    keywords_off
        .set_options(|options| options.value.lex = false)
        .expect("value.lex sets");
    let parsers = [windowed(make_json()), windowed(keywords_off)];
    let documents = [
        "[-1, 0, -0, 0.5, -12.5e-3, 1e5, 1E+2, 12345678901234567890]",
        "[1,22,333,4444,55555,666666,7777777]",
        "{\"a\":-1.5e3,\"b\":2,\"c\":[0.25]}",
        "1",
        "-0.0",
        "  7  ",
        "[01]",
        "[+1]",
        "[.5]",
        "[1.]",
        "[1.e3]",
        "[-]",
        "[--1]",
        "[1e]",
        "[1e+]",
        "[0x1F]",
        "[1_000]",
        "[1/2]",
        "[1 2]",
        "[-1.5e3x]",
        "[1,-]",
    ];
    let mut compared = 0;
    let mut failures = Vec::new();
    for parser in &parsers {
        for document in documents {
            let (count, found) = same_from_reader(parser, "strict number", document);
            compared += count;
            failures.extend(found);
        }
    }
    assert_eq!(compared, parsers.len() * documents.len() * 12);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Literals longer than the engine's window, which reaches 64 KiB past the
/// cursor before each token: the check reads on past the window's edge to
/// the literal's end, in strict JSON and in the relaxed grammar alike.
#[test]
fn a_literal_longer_than_the_window_parses_the_same_from_a_reader() {
    let digits = "0".repeat(70_000);
    let documents = [
        format!("[1{digits}]"),
        format!("[1.{digits}1, 2]"),
        format!("[1.{digits}x]"),
        format!("[-{digits}]"),
        format!("[1{digits}e5,2]"),
        format!("[{digits}]"),
    ];
    for parser in [windowed(make_json()), windowed(make())] {
        for document in &documents {
            let whole = outcome(parser.parse(document));
            for size in [4096, 64 * 1024 + 3] {
                let read = outcome(parser.parse_reader(Pieces::new(document, size)));
                assert!(
                    read == whole,
                    "{}... in pieces of {size}\n  whole:  {}\n  reader: {}",
                    &document[..12],
                    &whole[..whole.len().min(200)],
                    &read[..read.len().min(200)]
                );
            }
        }
    }
}

/// A long document, from a reader in pieces of 4 KiB, parses the same as
/// its text, under the relaxed grammar and, as JSON, under strict JSON; so
/// does the same document with an error at its end. One read is under the
/// engine's window, and one under a window of 256 characters, which moves
/// on hundreds of times before the end, where the error is placed as the
/// whole text places it. (A debug build parses jsonic at a few hundred
/// kilobytes a second, which bounds the length.)
#[test]
fn a_long_document_parses_the_same_from_a_reader() {
    let mut relaxed = String::new();
    for i in 0..1_500 {
        writeln!(
            relaxed,
            "r{i}: {{ id: {i}, name: \"ñ{i}\\u00e9\", tags: [a, 'b', `c`], n: -1.5e3, x: 0x1F }}, # row {i}"
        )
        .expect("a String takes a write");
    }
    let strict = format!(
        "[{}]",
        (0..2_400)
            .map(|i| format!(
                "{{\"id\":{i},\"name\":\"ñ{i}\",\"tags\":[\"a\",\"b\"],\"n\":-1.5e3}}"
            ))
            .collect::<Vec<_>>()
            .join(",\n")
    );
    let cases = [
        (windowed(make()), format!("{{\n{relaxed}}}\n"), true),
        (
            windowed(make()),
            format!("{{\n{relaxed}z: \"tail\u{1}\"\n}}\n"),
            false,
        ),
        (windowed(make_json()), strict.clone(), true),
        (
            windowed(make_json()),
            format!("{}, 01]", &strict[..strict.len() - 1]),
            false,
        ),
    ];
    let small = tabnas::WindowSizes {
        margin: 256,
        block: 256,
        chunk: 4096,
    };
    for (parser, text, ok) in &cases {
        assert!(text.len() > 128 << 10, "{}", text.len());
        let whole = outcome(parser.parse(text));
        assert_eq!(
            whole.starts_with("ok "),
            *ok,
            "{}",
            &whole[..whole.len().min(200)]
        );
        for (how, read) in [
            (
                "the engine's window",
                parser.parse_reader(Pieces::new(text, 4096)),
            ),
            (
                "a window of 256",
                parser.parse_reader_with_sizes(Pieces::new(text, 4096), small),
            ),
        ] {
            let read = outcome(read);
            assert!(
                read == whole,
                "under {how}\n  whole:  {}\n  reader: {}",
                &whole[..whole.len().min(200)],
                &read[..read.len().min(200)]
            );
        }
    }
}
