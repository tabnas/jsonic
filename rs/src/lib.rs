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
        "options": {
            // See above: both of these undo a json-layer restriction
            // rather than relaxing a lexer default.
            "rule": { "include": "" },
            "number": { "check": null },

            // The relaxed lexer surface.
            "text": { "lex": true },
            "comment": { "lex": true },
            "map": { "extend": true },
            "lex": { "empty": true },
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
