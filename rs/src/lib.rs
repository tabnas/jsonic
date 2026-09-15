// Copyright (c) 2026 tabnas, MIT License

// The engine's error carries a code, position, hint and a formatted
// report, so it is large by design and `Result<_, TabnasError>` trips
// clippy's `result_large_err`. The engine and the json plugin both allow
// the lint at their own crate roots for the same reason.
#![allow(clippy::result_large_err)]

//! The relaxed jsonic grammar, as a plugin for the `tabnas` engine.
//!
//! jsonic is standard JSON with the restrictions lifted: unquoted keys,
//! implicit objects and arrays, trailing and optional commas, comments,
//! single and backtick strings, multiline strings, and non-decimal
//! numbers.
//!
//! It is built by LAYERING on [`tabnas_json`], exactly as the canonical
//! TypeScript and the Go port do: the standard-JSON core supplies the
//! `val` / `map` / `list` / `pair` / `elem` rules and the engine's
//! native-value `$`-builtins that construct the value tree, and this
//! crate weaves its relaxed alternates around them.

use tabnas::{GrammarError, GrammarSpec, Tabnas, Value};

pub use tabnas::TabnasError as JsonicError;

/// This crate's version. It MUST equal `ts/package.json` "version".
pub const VERSION: &str = "0.1.0";

/// The relaxations applied over the strict-JSON core, and the alternates
/// that need them.
///
/// Two of these are not "relaxations" in the usual sense but repairs to
/// what the json layer deliberately locked down, and getting either
/// wrong makes the whole grammar inert:
///
///   - `rule.include` is `"json"` there, which keeps ONLY json-tagged
///     alternates. Every alternate this crate adds carries its own tag,
///     so leaving the filter in place would exclude all of them.
///   - `number.check` is json's strict-number preflight hook. jsonic
///     accepts hex, octal, binary and separator forms, so the hook has
///     to come off or it rejects them before the lexer sees them.
fn jsonic_document() -> serde_json::Value {
    serde_json::json!({
        "v": 2,
        // The relaxed surface, mirroring `ts/src/defaults.ts`. Most of
        // these are not "relaxations" of an engine default but REPAIRS of
        // what the json layer locked down, so the values are taken from
        // jsonic's own defaults rather than from what looks permissive.
        "options": {
            // Both undo a json-layer lock rather than widening a default.
            // `rule.include` is "json" there, keeping ONLY json-tagged
            // alternates, which would exclude every alternate below;
            // `number.check` is json's strict-number preflight, which
            // rejects hex, octal, binary and separator forms before the
            // lexer is reached.
            "rule": { "include": "", "finish": true },
            "number": {
                "check": null, "exclude": null,
                "lex": true, "hex": true, "oct": true, "bin": true, "sep": "_",
            },

            // Text runs end at a FIXED token, so declaring them is what
            // stops a text run swallowing structure. Without this,
            // `[1,2,]` parses as `[1,2,"]"]`: text lexing is on, but the
            // run has no ender and eats the bracket.
            "fixed": {
                "lex": true,
                "token": {
                    "#OB": "{", "#CB": "}",
                    "#OS": "[", "#CS": "]",
                    "#CL": ":", "#CA": ",",
                },
            },
            "text": { "lex": true },

            "string": {
                "lex": true,
                "chars": "'\"`",
                "multiChars": "`",
                "escapeChar": "\\",
                // json DELETED v / ' / ` by mapping them to null. jsonic
                // needs them back, so they are re-declared with values
                // rather than merely un-suppressed.
                "escape": {
                    "b": "\u{0008}", "f": "\u{000c}", "n": "\n", "r": "\r",
                    "t": "\t", "v": "\u{000b}",
                    "\"": "\"", "'": "'", "`": "`", "\\": "\\", "/": "/",
                },
                "allowUnknown": true,
                "escapeStrict": false,
                "abandon": false,
            },

            "comment": {
                "lex": true,
                "def": {
                    "hash":  { "line": true,  "start": "#",  "lex": true, "eatline": false },
                    "slash": { "line": true,  "start": "//", "lex": true, "eatline": false },
                    "multi": { "line": false, "start": "/*", "end": "*/", "lex": true, "eatline": false },
                },
            },

            "value": {
                "lex": true,
                "def": {
                    "true":  { "val": true },
                    "false": { "val": false },
                    "null":  { "val": null },
                },
            },

            "map": { "extend": true },
            "list": { "property": true },
            "lex": { "empty": true },
            "safe": { "key": true },

            // json locks KEY to `["#ST"]` -- quoted strings only. That one
            // line is what keeps `{a:1}` from parsing however many relaxed
            // alternates are added, because the `#KEY #CL` alts below
            // never match an unquoted key.
            "tokenSet": {
                "KEY": ["#TX", "#NR", "#ST", "#VL"],
                "VAL": ["#TX", "#NR", "#ST", "#VL"],
            },
        },

        "rule": {
            // HYPOTHESIS CHECK: json parsed its own `#KEY #CL` alt while
            // tokenSet.KEY was locked to ["#ST"], so that alt is frozen to
            // quoted keys. Re-declaring it here, AFTER the set is widened,
            // should make `{a:1}` parse.
            "pair": {
                "open": {
                    "alts": [
                        { "s": "#KEY #CL", "p": "val", "u": { "pair": true },
                          "a": "@key$", "g": "map,pair,key,jsonic" },
                    ],
                },
            },

            "val": {
                "open": {
                    "alts": [
                        // A pair key at top level: `a: ...`, an implicit
                        // map. `@reset$` mirrors json's #OB/#OS opens, so
                        // val-close coalesces to the pushed map rather
                        // than the inherited parent container.
                        { "s": "#KEY #CL", "c": { "d": 0 }, "p": "map", "b": 2,
                          "a": "@reset$", "g": "pair,jsonic,top" },

                        // A pair dive: `a:b: ...`. Without `@reset$` a
                        // dive inside an explicit map (`{a:b:1}`)
                        // coalesces a's value to the OUTER map, which is
                        // a circular self-reference.
                        { "s": "#KEY #CL", "p": "map", "b": 2,
                          "n": { "pk": 1 }, "a": "@reset$", "g": "pair,jsonic" },

                        // A plain value. Replaces json's own #VAL open,
                        // which `delete: [2]` removes below.
                        { "s": "#VAL", "a": "@reset$", "g": "val,json" },

                        // Implicit ends: `{a:}` -> {"a":null}.
                        { "s": ["#CB #CS"], "b": 1, "c": { "d": { "$gt": 0 } },
                          "a": "@reset$", "g": "val,imp,null,jsonic" },

                        // Implicit list at top level opening on a comma:
                        // `,` -> [null]. Allocated here because this path
                        // does not reach @list-bo's promotion and json's
                        // `@array$` only runs for `[`.
                        { "s": "#CA", "c": { "d": 0 }, "p": "list", "b": 1,
                          "a": "@array$", "k": { "array$": { "implicit": true } },
                          "g": "list,imp,jsonic" },

                        // Implicitly null before a comma.
                        { "s": "#CA", "b": 1, "a": "@reset$",
                          "g": "list,val,imp,null,jsonic" },

                        { "s": "#ZZ", "g": "jsonic" },
                    ],
                    // APPEND, not prepend: json's strict opens must still
                    // be tried first. `delete: [2]` drops json's #VAL open
                    // so the re-declared one above carries jsonic's tags.
                    "inject": { "append": true, "delete": [2] },
                },

                "close": {
                    "alts": [
                        { "s": ["#CB #CS"], "b": 1, "g": "val,json,close",
                          "e": "@val-close-error" },

                        // Implicit comma-separated list, top level only.
                        { "s": "#CA",
                          "c": { "n.dlist": { "$lte": 0 }, "n.dmap": { "$lte": 0 } },
                          "r": "list", "u": { "implist": true },
                          "g": "list,val,imp,comma,jsonic" },

                        // Implicit space-separated list, top level only.
                        { "c": { "n.dlist": { "$lte": 0 }, "n.dmap": { "$lte": 0 } },
                          "r": "list", "u": { "implist": true },
                          "g": "list,val,imp,space,jsonic", "b": 1 },

                        { "s": "#ZZ", "g": "end,jsonic" },
                    ],
                    // `move: [1, -1]` sends json's "there is more JSON"
                    // close to the end, so the implicit-list closes get
                    // their chance first.
                    "inject": { "append": true, "move": [1, -1] },
                },
            },
        },
    })
}

/// Install the relaxed jsonic grammar on `parser`.
///
/// Installs the standard-JSON core first, then layers on it. A second
/// serialized grammar MERGES into the rules already present rather than
/// replacing them, and by default its alternates are PREPENDED, so a
/// relaxed alternate is tried before the strict one it widens.
pub fn jsonic(parser: &mut Tabnas) -> Result<(), GrammarError> {
    tabnas_json::json(parser)?;

    // BEFORE the document: the engine snapshots the reference tables when
    // a grammar is installed, so a name the document uses must already be
    // registered or the install fails with "unknown ... function
    // reference".
    //
    // `@val-close-error`: a `}` or `]` at depth 0 has nothing to close,
    // so it is an error there and an ordinary close everywhere else.
    parser.alt_error("@val-close-error", |rule, context| {
        if rule.d == 0 {
            context.t.first().cloned()
        } else {
            None
        }
    });

    let spec = GrammarSpec::from_value(jsonic_document())?;
    parser.grammar(&spec)?;
    Ok(())
}

/// Build a jsonic parser instance.
pub fn make() -> Tabnas {
    let mut parser = Tabnas::new();
    jsonic(&mut parser).expect("the jsonic grammar document is fixed and valid");
    parser
}

/// Parse a jsonic source string with the shared default parser.
pub fn parse(src: &str) -> Result<Value, JsonicError> {
    use std::sync::OnceLock;
    static DEFAULT: OnceLock<Tabnas> = OnceLock::new();
    DEFAULT.get_or_init(make).parse(src)
}
