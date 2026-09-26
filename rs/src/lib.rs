// Copyright (c) 2013-2026 Richard Rodger, MIT License

// The engine's error carries a code, position, hint and a formatted
// report, so it is large by design and `Result<_, TabnasError>` trips
// clippy's `result_large_err`. The engine allows the lint at its own
// crate root for the same reason; boxing here would make `parse` return
// a different shape from `Tabnas::parse` and from the other two ports.
#![allow(clippy::result_large_err)]

//! The jsonic relaxed-JSON grammar plugin for the `tabnas` parsing engine.
//!
//! jsonic accepts standard JSON and then relaxes it for humans: unquoted
//! keys, implicit objects and arrays, comments, trailing commas, single
//! and backtick quoted strings, and path diving (`a:b:1` is
//! `{"a":{"b":1}}`).
//!
//! The standard-JSON core (`val` / `map` / `list` / `pair` / `elem`) comes
//! from the [`tabnas_json`] plugin; this crate installs that core and
//! weaves the relaxed alternates and lifecycle actions around it, exactly
//! as the canonical TypeScript grammar in `ts/src/grammar.ts` does.
//!
//! ```
//! let value = tabnas_jsonic::parse("a:1, b:[x,y,z]")?;
//! assert_eq!(value.to_string(), r#"{"a":1,"b":["x","y","z"]}"#);
//! # Ok::<(), tabnas_jsonic::JsonicError>(())
//! ```
//!
//! TypeScript is canonical: `ts/src/grammar.ts` and `ts/src/defaults.ts`
//! define behaviour, option defaults and the order of alternates. The
//! shared fixtures in `test/spec/*.tsv` are the parity contract across
//! TypeScript, Go and Rust.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, OnceLock};

use indexmap::IndexMap;
use regex::Regex;
use serde_json::json;
use tabnas::{
    ActionError, Context, GrammarError, GrammarSpec, LexCheckResult, ListRef, Options, Plugin,
    PluginError, Rule, Tabnas, Text, Tin, Token, Value, TIN_OB, TIN_OS, TIN_ST, TIN_TX, TIN_ZZ,
};

/// This crate's version. It MUST equal `ts/package.json` "version": the
/// release orchestrator rewrites both, and `tests/version_test.rs` fails
/// the build if they drift. Mirrors `VERSION` in `ts/src/jsonic.ts` and
/// `const VERSION` in `go/jsonic.go`.
pub const VERSION: &str = "0.7.2";

/// The README's Rust examples run as doctests, so a stale one fails the
/// gate rather than misleading the reader. Its `toml` and `bash` fences
/// are skipped; rustdoc runs only the `rust` ones.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
mod readme_examples {}

/// The error a failed parse produces, re-exported so callers need not
/// depend on the engine crate directly. Mirrors the TypeScript
/// `export { TabnasError as JsonicError }`.
pub use tabnas::TabnasError as JsonicError;

// ---------------------------------------------------------------------------
// Named function references.
//
// The serialized grammar names every closure, and the closures are
// registered on the instance BEFORE the document is installed. Each
// lifecycle reference carries its phase suffix (`/append`, `/replace`) on
// purpose: the engine resolves a bare `@map-bo`, `@list-bo`, `@pair-bc`,
// `@elem-bc`, `@val-bc` or `@pairkey` to its own builtin first, so a
// registration under one of those names would be shadowed silently.
// ---------------------------------------------------------------------------

/// Alternate error: with `rule.finish` off, an unterminated structure is
/// `end_of_source` rather than an implicit close.
const FINISH: &str = "@finish";
/// Alternate action: capture the pair key from the first open token.
const PAIRKEY: &str = "@jsonic-pairkey";
/// Takes over the `val` before-close phase from `tabnas_json`.
const VAL_BC: &str = "@val-bc/replace";
/// After-close: restore a primitive value a plugin set in `val` open.
const VAL_AC: &str = "@val-ac/append";
const MAP_BO: &str = "@map-bo/append";
const MAP_BC: &str = "@map-bc/append";
const LIST_BO: &str = "@list-bo/append";
const LIST_BC: &str = "@list-bc/append";
const PAIR_BC: &str = "@pair-bc/append";
/// Takes over the `elem` before-close phase from `tabnas_json`.
const ELEM_BC: &str = "@elem-bc/replace";
const VAL_CLOSE_ERROR: &str = "@val-close-error";
const ELEM_DOUBLE_COMMA: &str = "@elem-double-comma";
const ELEM_SINGLE_COMMA: &str = "@elem-single-comma";
const ELEM_PAIR_ERR: &str = "@elem-pair-err";
const ELEM_CLOSE_ERR: &str = "@elem-close-err";
/// The strict-JSON number preflight hook [`make_json`] binds by name.
const STRICT_NUMBER_CHECK: &str = "jsonic-strict-number";

/// The name every jsonic plugin registers under, and so the namespace of
/// its plugin options.
const PLUGIN_NAME: &str = "jsonic";

// ---------------------------------------------------------------------------
// Options
// ---------------------------------------------------------------------------

/// The option branding jsonic layers on the engine's relaxed defaults.
///
/// `ts/src/defaults.ts` was compared field by field against the engine's
/// `Options::default()`: the lexer, value, map, list, info, rule and
/// result defaults are identical, so only the error identity and the
/// jsonic hint catalogue travel here. The `error` templates are the
/// engine's own, re-stated by jsonic rather than declared, and so are
/// left alone.
fn jsonic_options_document() -> serde_json::Value {
    json!({
        "options": {
            "errmsg": {
                "name": "jsonic",
                "link": "https://github.com/tabnas/jsonic",
            },
            "hint": {
                "unknown": "\nSince the error is unknown, this is probably a bug inside jsonic\nitself, or a plugin. Please consider posting a github issue - thanks!\n\nCode: {code}, Details: \n{details}",
                "unexpected": "\nThe character(s) {src} were not expected at this point as they do not\nmatch the expected syntax, even under the relaxed jsonic rules. If it\nis not obviously wrong, the actual syntax error may be elsewhere. Try\ncommenting out larger areas around this point until you get no errors,\nthen remove the comments in small sections until you find the\noffending syntax. NOTE: Also check if any plugins you are using\nexpect different syntax in this case.",
                "invalid_unicode": "\nThe escape sequence {src} does not encode a valid unicode code point\nnumber. You may need to validate your string data manually using test\ncode to see how JavaScript will interpret it. Also consider that your\ndata may have become corrupted, or the escape sequence has not been\ngenerated correctly.",
                "invalid_ascii": "\nThe escape sequence {src} does not encode a valid ASCII character. You\nmay need to validate your string data manually using test code to see\nhow JavaScript will interpret it. Also consider that your data may\nhave become corrupted, or the escape sequence has not been generated\ncorrectly.",
                "unprintable": "\nString values cannot contain unprintable characters (character codes\nbelow 32). The character {src} is unprintable. You may need to remove\nthese characters from your source data. Also check that it has not\nbecome corrupted.",
                "unterminated_string": "\nThis string has no end quote.",
                "unterminated_comment": "\nThis comment is never closed.",
                "unknown_rule": "\nNo rule named $rulename is defined. This is probably an error in the\ngrammar of a plugin.",
                "end_of_source": "\nUnexpected end of source.",
            },
        },
    })
}

/// Apply jsonic's option branding to `parser`.
fn apply_options(parser: &mut Tabnas) -> Result<(), GrammarError> {
    let spec = GrammarSpec::from_value(jsonic_options_document())?;
    parser.grammar(&spec)?;
    Ok(())
}

/// Put back the relaxed lexer profile after [`tabnas_json::json`] has
/// applied its strict one.
///
/// `tabnas_json` has one entry point that installs its OPTIONS as well as
/// its RULES, where the TypeScript `registerJsonGrammar` installs rules
/// alone. jsonic wants the rules only: every field the strict profile
/// changes is restored to what it was before the call, so a caller's own
/// setting (the strict `KEY` token set of [`make_json`], say) survives
/// the layering rather than being reset to the engine default.
fn restore_relaxed(options: &mut Options, relaxed: &Options) {
    options.text.lex = relaxed.text.lex;
    options.number.hex = relaxed.number.hex;
    options.number.oct = relaxed.number.oct;
    options.number.bin = relaxed.number.bin;
    options.number.sep.clone_from(&relaxed.number.sep);
    options.number.check.clone_from(&relaxed.number.check);
    options.string.chars.clone_from(&relaxed.string.chars);
    options
        .string
        .multi_chars
        .clone_from(&relaxed.string.multi_chars);
    options.string.allow_unknown = relaxed.string.allow_unknown;
    options.string.escape_strict = relaxed.string.escape_strict;
    options.string.escape.clone_from(&relaxed.string.escape);
    options.comment.lex = relaxed.comment.lex;
    options.map.extend = relaxed.map.extend;
    options.lex.empty = relaxed.lex.empty;
    options.rule.finish = relaxed.rule.finish;
    options.rule.include.clone_from(&relaxed.rule.include);
    match relaxed.token_set.get("KEY") {
        Some(key) => {
            options.token_set.insert("KEY".to_string(), key.clone());
        }
        None => {
            options.token_set.remove("KEY");
        }
    }
    options.parse.budget = relaxed.parse.budget.clone();
}

// ---------------------------------------------------------------------------
// The grammar document
// ---------------------------------------------------------------------------

/// The relaxed alternates, phase one: the `rs.open(...)` / `rs.close(...)`
/// calls of `ts/src/grammar.ts` in order, one entry per rule and state.
///
/// The engine applies an `inject` object's `delete` and `move` to the
/// EXISTING alternates before the new ones go in, where TypeScript inserts
/// first and modifies after. Three of jsonic's calls depend on that order:
/// the `move` that sends json's "there is more JSON" alternate to the end
/// of `val.close`, and the second `.open(..., {append: true})` on `map`
/// and `list`. Those are the second phase, [`jsonic_document_append`],
/// which runs once this one is installed, so the final lists are the
/// TypeScript ones alternate for alternate.
///
/// The option-conditional alternates (`map.child`, `list.child`, and the
/// `list.property || list.pair` guard on a pair inside a list) are decided
/// against `options` when the document is built, as `p.cfg.*` decides
/// them in TypeScript and `cfg.*` in Go.
fn jsonic_document(options: &Options) -> serde_json::Value {
    let list_pairs_allowed = options.list.property || options.list.pair;

    let mut pair_open = vec![
        // Re-declare the key alt so it binds jsonic's pairkey (which uses
        // the key token's SOURCE for number and value-keyword keys, e.g.
        // `1:x` -> "1", `true:1` -> "true"), replacing the strict
        // tabnas_json version that uses the decoded token value. The
        // `clear` below drops tabnas_json's pair open alts first.
        json!({ "s": "#KEY #CL", "p": "val", "u": { "pair": true }, "a": PAIRKEY,
                "g": "map,pair,key,json" }),
        // Ignore initial comma: {,a:1.
        json!({ "s": "#CA", "g": "map,pair,comma,jsonic" }),
    ];
    if options.map.child {
        // map.child: bare colon `:value` stores value on child$ property.
        pair_open.push(
            json!({ "s": "#CL", "p": "val", "u": { "done": true, "child": true },
                               "g": "map,pair,child,jsonic" }),
        );
    }

    let mut elem_pair = json!({
        "s": "#KEY #CL", "p": "val",
        "n": { "pk": 1, "dmap": 1 },
        "u": { "done": true, "pair": true, "list": true },
        "a": PAIRKEY,
        "g": "elem,pair,jsonic",
    });
    if !list_pairs_allowed {
        elem_pair["e"] = json!(ELEM_PAIR_ERR);
    }
    let mut elem_open = vec![
        // Empty commas insert null elements. Note that close consumes a
        // comma, so b:2 works.
        json!({ "s": "#CA #CA", "b": 2, "u": { "done": true }, "a": ELEM_DOUBLE_COMMA,
                "g": "list,elem,imp,null,jsonic" }),
        json!({ "s": "#CA", "u": { "done": true }, "a": ELEM_SINGLE_COMMA,
                "g": "list,elem,imp,null,jsonic" }),
        elem_pair,
    ];
    if options.list.child {
        // list.child: bare colon `:value` stores value on child$ property.
        elem_open.push(json!({ "s": "#CL", "p": "val",
                               "u": { "done": true, "child": true, "list": true },
                               "g": "elem,child,jsonic" }));
    }

    json!({
        // The schema version of the native-value builtins this grammar
        // binds to (object/array/reset, and json's key/setval/push/value).
        "v": 2,

        "rule": {
            "val": {
                "open": {
                    "alts": [
                        // A pair key: `a: ...`. Implicit map at top level.
                        // @reset$ clears the parent-seeded node (mirrors
                        // json's #OB/#OS open alts) so val-close coalesces
                        // to the pushed map, not the inherited parent
                        // container.
                        { "s": "#KEY #CL", "c": { "d": 0 }, "p": "map", "b": 2, "a": "@reset$",
                          "g": "pair,jsonic,top" },

                        // A pair dive: `a:b: ...`. Increment counter n.pk
                        // to indicate pair-key depth (for extensions).
                        // a:9 -> pk=undef, a:b:9 -> pk=1, a:b:c:9 -> pk=2.
                        { "s": "#KEY #CL", "p": "map", "b": 2, "n": { "pk": 1 }, "a": "@reset$",
                          "g": "pair,jsonic" },

                        // A plain value: `x` `"x"` `1` `true` ....
                        { "s": "#VAL", "a": "@reset$", "g": "val,json" },

                        // Implicit ends `{a:}` -> {"a":null},
                        // `[a:]` -> [{"a":null}].
                        { "s": ["#CB #CS"], "b": 1, "c": { "d": { "$gt": 0 } }, "a": "@reset$",
                          "g": "val,imp,null,jsonic" },

                        // Implicit list at top level starting with a
                        // comma: `,` -> [null]. Allocate the (implicit)
                        // array here: this path does not go through
                        // list-bo's implist promotion, and json's @array$
                        // only runs for `[`.
                        { "s": "#CA", "c": { "d": 0 }, "p": "list", "b": 1, "a": "@array$",
                          "k": { "array$": { "implicit": true } }, "g": "list,imp,jsonic" },

                        // Value is implicitly null when empty before commas.
                        { "s": "#CA", "b": 1, "a": "@reset$", "g": "list,val,imp,null,jsonic" },

                        { "s": "#ZZ", "g": "jsonic" },
                    ],
                    "inject": { "append": true, "delete": [2] },
                },
                "close": {
                    "alts": [
                        // Explicitly close map or list: `}`, `]`
                        { "s": ["#CB #CS"], "b": 1, "g": "val,json,close", "e": VAL_CLOSE_ERROR },

                        // Implicit list (comma sep) only allowed at top
                        // level: `1,2`.
                        { "s": "#CA", "c": { "n.dlist": { "$lte": 0 }, "n.dmap": { "$lte": 0 } },
                          "r": "list", "u": { "implist": true },
                          "g": "list,val,imp,comma,jsonic" },

                        // Implicit list (space sep) only allowed at top
                        // level: `1 2`.
                        { "c": { "n.dlist": { "$lte": 0 }, "n.dmap": { "$lte": 0 } },
                          "r": "list", "u": { "implist": true },
                          "g": "list,val,imp,space,jsonic", "b": 1 },

                        { "s": "#ZZ", "g": "end,jsonic" },
                    ],
                    "inject": { "append": true },
                },
            },

            "map": {
                "open": [
                    // Auto-close; fail if rule.finish option is false.
                    // Allocate the (empty) object so `{` -> `{}` when
                    // finish is allowed.
                    { "s": "#OB #ZZ", "b": 1, "a": "@object$", "e": FINISH, "g": "end,jsonic" },
                ],
                "close": {
                    "alts": [
                        // Normal end of map, no path dive.
                        { "s": "#CB", "c": { "n.pk": { "$lte": 0 } }, "g": "end,json" },

                        // Not yet at end of path dive, keep ascending.
                        { "s": "#CB", "b": 1, "g": "path,close,jsonic" },

                        // End of implicit path.
                        { "s": ["#CA #CS #VAL"], "b": 1, "g": "end,path,jsonic" },

                        // Auto-close; fail if rule.finish option is false.
                        { "s": "#ZZ", "e": FINISH, "g": "end,jsonic" },
                    ],
                    "inject": { "append": true, "delete": [0] },
                },
            },

            "list": {
                "open": [
                    // An implicit (bracket-less) list continues with an
                    // element: the first value was promoted in list-bo.
                    { "c": { "prev.u.implist": { "$eq": true } }, "p": "elem" },
                ],
                "close": {
                    "alts": [
                        // Fail if rule.finish option is false.
                        { "s": "#ZZ", "e": FINISH, "g": "end,jsonic" },
                    ],
                    "inject": { "append": true },
                },
            },

            "pair": {
                "open": {
                    "alts": pair_open,
                    "inject": { "append": true, "clear": true },
                },
                "close": {
                    "alts": [
                        // End of map, reset implicit depth counter so that
                        // a:b:c:1,d:2 -> {a:{b:{c:1}},d:2}
                        { "s": "#CB", "c": { "n.pk": { "$lte": 0 } }, "b": 1,
                          "g": "map,pair,close,json" },

                        // Ignore trailing comma at end of map.
                        { "s": "#CA #CB", "c": { "n.pk": { "$lte": 0 } }, "b": 1,
                          "g": "map,pair,comma,jsonic" },

                        { "s": "#CA #ZZ", "g": "end,jsonic" },

                        // Comma means a new pair at same pair-key level.
                        { "s": "#CA", "c": { "n.pk": { "$lte": 0 } }, "r": "pair",
                          "g": "map,pair,sync,json" },

                        // Comma means a new pair if implicit top level map.
                        { "s": "#CA", "c": { "n.dmap": { "$lte": 1 } }, "r": "pair",
                          "g": "map,pair,sync,jsonic" },

                        // Value means a new pair if implicit top level map.
                        { "s": "#KEY", "c": { "n.dmap": { "$lte": 1 } }, "r": "pair", "b": 1,
                          "g": "map,pair,imp,sync,jsonic" },

                        // End of implicit path (eg. a:b:1), keep closing
                        // until pk=0.
                        { "s": ["#CB #CA #CS #KEY"], "c": { "n.pk": { "$gt": 0 } }, "b": 1,
                          "g": "map,pair,imp,path,close,jsonic" },

                        // Can't close a map with `]`
                        { "s": "#CS", "e": ELEM_CLOSE_ERR, "g": "end,jsonic" },

                        // Fail if auto-close option is false.
                        { "s": "#ZZ", "e": FINISH, "g": "map,pair,end,json" },

                        // Who needs commas anyway?
                        { "r": "pair", "b": 1, "g": "map,pair,imp,jsonic" },
                    ],
                    "inject": { "append": true, "delete": [0, 1] },
                },
            },

            "elem": {
                "open": elem_open,
                "close": {
                    "alts": [
                        // Ignore trailing comma.
                        { "s": ["#CA", "#CS #ZZ"], "b": 1, "g": "list,elem,comma,jsonic" },

                        // Next element.
                        { "s": "#CA", "r": "elem", "g": "list,elem,sync,json" },

                        // End of list.
                        { "s": "#CS", "b": 1, "g": "list,elem,close,json" },

                        // Fail if auto-close option is false.
                        { "s": "#ZZ", "e": FINISH, "g": "list,elem,end,json" },

                        // Can't close a list with `}`
                        { "s": "#CB", "e": ELEM_CLOSE_ERR, "g": "end,jsonic" },

                        // Who needs commas anyway?
                        { "r": "elem", "b": 1, "g": "list,elem,imp,jsonic" },
                    ],
                    "inject": { "delete": [-1, -2] },
                },
            },
        },
    })
}

/// The relaxed alternates, phase two. See [`jsonic_document`].
fn jsonic_document_append() -> serde_json::Value {
    json!({
        "v": 2,
        "rule": {
            "val": {
                // Move "There's more JSON" to end.
                "close": { "alts": [], "inject": { "move": [1, -1] } },
            },
            "map": {
                "open": {
                    "alts": [
                        // Pair from implicit map (no braces). json's
                        // map-open alts only match `#OB`, so the brace-less
                        // entry must allocate the container itself:
                        // @object$ with the static implicit:true flag.
                        { "s": "#KEY #CL", "p": "pair", "b": 2, "a": "@object$",
                          "k": { "object$": { "implicit": true } },
                          "g": "pair,list,val,imp,jsonic" },
                    ],
                    "inject": { "append": true },
                },
            },
            "list": {
                "open": {
                    "alts": [
                        // Initial comma [, will insert null as [null,
                        { "s": "#CA", "p": "elem", "b": 1, "g": "list,elem,val,imp,jsonic" },

                        // Another element.
                        { "p": "elem", "g": "list,elem,jsonic" },
                    ],
                    "inject": { "append": true },
                },
            },
        },
    })
}

// ---------------------------------------------------------------------------
// Node helpers
// ---------------------------------------------------------------------------

/// Assign a rule's node: the Rust spelling of TypeScript `r.node = v`.
///
/// A pushed or replaced rule SHARES its parent's node cell, so writing
/// through `rule.node.borrow_mut()` would overwrite the parent's node too.
/// Assigning installs a fresh cell instead. The cell is borrowed directly
/// only to mutate a container the rule genuinely shares (push onto the
/// enclosing list, insert into the enclosing map).
fn set_node(rule: &mut Rule, value: Value) {
    rule.node = Rc::new(RefCell::new(value));
}

/// Take a container out of its `Arc`, copying only when it is shared.
fn unwrap_arc<T: Clone>(shared: Arc<T>) -> T {
    Arc::try_unwrap(shared).unwrap_or_else(|shared| (*shared).clone())
}

fn map_get(node: &Value, key: &str) -> Option<Value> {
    match node {
        Value::Object(map) => map.get(key).cloned(),
        Value::MapRef(map) => map.value.get(key).cloned(),
        _ => None,
    }
}

/// Insert into a map node. A non-map node (a list carrying a `key:value`
/// pair under `list.property`) takes nothing, as in the Go port: a JSON
/// array has no properties to carry.
fn map_insert(node: &mut Value, key: String, value: Value) {
    match node {
        Value::Object(map) => {
            Arc::make_mut(map).insert(key, value);
        }
        Value::MapRef(map) => {
            Arc::make_mut(map).value.insert(key, value);
        }
        _ => {}
    }
}

fn list_push(node: &mut Value, value: Value) {
    match node {
        Value::Array(items) => Arc::make_mut(items).push(value),
        Value::ListRef(list) => Arc::make_mut(list).value.push(value),
        _ => {}
    }
}

/// A scalar a plugin deliberately set: not the no-value sentinel, not
/// null, not a container. TypeScript's test is `'object' !== typeof`, so
/// a `Text` (a `String` object there) counts as a container.
fn is_primitive(value: &Value) -> bool {
    matches!(value, Value::Bool(_) | Value::Number(_) | Value::String(_))
}

fn flag(rule: &Rule, name: &str) -> bool {
    matches!(rule.u.get(name), Some(Value::Bool(true)))
}

/// The completed child's node, with "no value" read as `null`, as JSON has
/// no `undefined`.
fn child_or_null(rule: &Rule) -> Value {
    if rule.child_node.is_undefined() {
        Value::Null
    } else {
        rule.child_node.clone()
    }
}

fn is_fixed(tin: Tin, options: &Options) -> bool {
    options.fixed.tokens.values().any(|token| token.tin == tin)
}

/// Whether a matched token carries no value, as `r.o0.resolveVal` reads
/// it in the canonical engine: end of source, or a fixed token that is
/// still as the lexer made it. This engine's lexer gives a fixed token its
/// own source text as `val` (`}` carries `"}"`), where the canonical one
/// gives it nothing, so the source text IS the no-value. Anything else on
/// a fixed token was put there by a plugin action in `val` open (the
/// `parser-mixed-token` shape, `r.o0.val = '@' + r.o1.val`) and is a
/// value like any other.
fn carries_no_value(token: &Token, options: &Options) -> bool {
    token.tin == TIN_ZZ
        || (is_fixed(token.tin, options)
            && matches!(&token.val, Value::String(text) if text == token.src.as_str()))
}

/// Recursively merge `overlay` into `base`: objects by key, arrays by
/// index, everything else replaced by the overlay. The port of the
/// `deep()` utility the TypeScript grammar merges duplicate keys with,
/// over the engine's native values, keeping the base's container kind
/// (a `MapRef` stays a `MapRef` with its metadata). The prototype
/// pollution guard is the same one: `__proto__`, `constructor` and
/// `prototype` keys of the overlay are skipped.
pub fn deep_merge(base: Value, overlay: Value) -> Value {
    const DANGEROUS: [&str; 3] = ["__proto__", "constructor", "prototype"];
    fn merge_entries(base: &mut IndexMap<String, Value>, overlay: IndexMap<String, Value>) {
        for (key, value) in overlay {
            if DANGEROUS.contains(&key.as_str()) {
                continue;
            }
            let previous = base.get(&key).cloned().unwrap_or(Value::Undefined);
            base.insert(key, deep_merge(previous, value));
        }
    }
    fn merge_items(base: &mut Vec<Value>, overlay: Vec<Value>) {
        for (index, value) in overlay.into_iter().enumerate() {
            if index < base.len() {
                let previous = std::mem::replace(&mut base[index], Value::Undefined);
                base[index] = deep_merge(previous, value);
            } else {
                base.push(value);
            }
        }
    }
    match (base, overlay) {
        (base, Value::Undefined) => base,
        (Value::Object(base), Value::Object(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_entries(&mut base, unwrap_arc(overlay));
            Value::object(base)
        }
        (Value::Object(base), Value::MapRef(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_entries(&mut base, unwrap_arc(overlay).value);
            Value::object(base)
        }
        (Value::MapRef(base), Value::Object(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_entries(&mut base.value, unwrap_arc(overlay));
            Value::MapRef(Arc::new(base))
        }
        (Value::MapRef(base), Value::MapRef(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_entries(&mut base.value, unwrap_arc(overlay).value);
            Value::MapRef(Arc::new(base))
        }
        (Value::Array(base), Value::Array(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_items(&mut base, unwrap_arc(overlay));
            Value::array(base)
        }
        (Value::Array(base), Value::ListRef(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_items(&mut base, unwrap_arc(overlay).value);
            Value::array(base)
        }
        (Value::ListRef(base), Value::Array(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_items(&mut base.value, unwrap_arc(overlay));
            Value::ListRef(Arc::new(base))
        }
        (Value::ListRef(base), Value::ListRef(overlay)) => {
            let mut base = unwrap_arc(base);
            merge_items(&mut base.value, unwrap_arc(overlay).value);
            Value::ListRef(Arc::new(base))
        }
        (_, overlay) => overlay,
    }
}

/// Combine a repeated value with the one already stored: the `map.merge`
/// function when the caller set one, else a deep merge under `map.extend`,
/// else the later value replaces the earlier.
fn combine(previous: Value, value: Value, rule: &mut Rule, context: &mut Context) -> Value {
    if let Some(merge) = context.options.map.merge.clone() {
        merge(previous, value, rule, context)
    } else if context.options.map.extend {
        deep_merge(previous, value)
    } else {
        value
    }
}

// ---------------------------------------------------------------------------
// The parse budget
// ---------------------------------------------------------------------------

/// How many nested containers a parse may hold before it is refused.
///
/// The engine's parse loop is iterative, but the value it hands back is a
/// tree the engine walks with the call stack to display, convert or drop,
/// one frame per level, and a source of a few thousand `[` ended the
/// process with a stack overflow, an abort rather than an error, before
/// this budget existed (past 6,000 levels in a release build and 1,500 in
/// a debug build on a 2 MiB thread). TypeScript and Go have no limit,
/// which is recorded in `DIVERGENCE.md`; a document a person writes does
/// not come near this one. The number is the one `tabnas_json` uses, so
/// the two Rust crates bound nesting the same way, and it is the depth
/// `serde_json` accepts.
const DEPTH_LIMIT: usize = 127;

/// How many containers are open at this point in the parse: the `map` and
/// `list` rules on the stack, plus the rule the loop is working on, which
/// the engine hands over separately as `context.rule`. Counted from the
/// rule NAMES rather than `rule_stack.len()`, because the stack holds
/// about three rules per level (`val`, then `map` or `list`, then `pair`
/// or `elem`) and a length-based limit would encode that ratio. A pair
/// dive (`a:b:c:1`) opens one implicit map per key, so it counts too.
fn depth(context: &Context) -> usize {
    let is_container = |name: &str| name == "map" || name == "list";
    let ancestors = context
        .rule_stack
        .iter()
        .filter(|rule| is_container(&rule.name))
        .count();
    let current = usize::from(
        context
            .rule
            .as_ref()
            .is_some_and(|rule| is_container(&rule.name)),
    );
    ancestors + current
}

/// The budget check: `DEPTH_LIMIT` levels parse, the next one is refused
/// with the engine's `cancel` code.
fn within_depth_limit(context: &Context) -> bool {
    depth(context) <= DEPTH_LIMIT
}

// ---------------------------------------------------------------------------
// The closures the grammar names
// ---------------------------------------------------------------------------

/// Resolve the matched open token to its value, as `r.o0.resolveVal` does:
/// the end-of-source token and an untouched fixed token carry no value
/// (see [`carries_no_value`]), and under `info.text` a string keeps its
/// quote character.
fn resolve_token(rule: &mut Rule, context: &mut Context) -> Value {
    let Some(token) = rule.o0().cloned() else {
        return Value::Undefined;
    };
    if carries_no_value(&token, &context.options) {
        return Value::Undefined;
    }
    let value = token.resolve_val(rule, context);
    if context.options.info.text && matches!(token.tin, TIN_ST | TIN_TX) {
        if let Value::String(string) = value {
            let quote = if token.tin == TIN_ST {
                token
                    .src
                    .as_str()
                    .chars()
                    .next()
                    .map(String::from)
                    .unwrap_or_default()
            } else {
                String::new()
            };
            return Value::Text(Text { quote, string });
        }
    }
    value
}

/// jsonic's `val` before-close coalescing, ordered: child container, then
/// a deliberate primitive a plugin set in `val` open, then the matched
/// scalar token (which beats a stale parent-seeded container), then a
/// deliberate container a plugin set, else no value (implicit null).
///
/// The child is read through [`Rule::has_child_value`] /
/// [`Rule::child_value`], never the raw `child_node` field. A child that
/// never installed a node cell of its own wrote into THIS rule's cell,
/// and the engine then leaves `child_node` undefined rather than hold a
/// second copy-on-write handle on the container this rule is about to
/// write into again. The accessors answer from the node in that case,
/// which is what the canonical `r.child.node` reads — there the two are
/// the same object. Reading the field directly makes such a val look
/// childless, and `,` then parses as `null` instead of `[null]`.
fn val_before_close(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    // Stash the value a plugin set in a val OPEN action (before any
    // coalescing). json's @value$ close ALT action still runs after this
    // and re-resolves the matched token, which would overwrite a plugin
    // value; `val_after_close` restores it. A normal value rule @reset$s
    // its node in open, so this is undefined except when a plugin
    // deliberately set it.
    let openval = rule.node.borrow().clone();
    rule.u_mut().insert("openval".to_string(), openval.clone());

    let node = if rule.has_child_value() {
        rule.child_value()
    } else if is_primitive(&openval) {
        return Ok(());
    } else if rule.os() != 0 {
        resolve_token(rule, context)
    } else if !openval.is_undefined() {
        return Ok(());
    } else {
        Value::Undefined
    };
    set_node(rule, node);
    Ok(())
}

/// After-close: json's @value$ close alt re-resolves the matched token and
/// so overwrites a value a plugin set in a val open action. Restore the
/// plugin's value, but only a PRIMITIVE one set with no child: a
/// parent-seeded stale node is always a container.
///
/// The same re-resolution reads a fixed token's value, and this engine's
/// fixed tokens carry their source text where the canonical engine's
/// carry nothing: `a:,b:` would give `a` the value `","`. The value the
/// canonical @value$ produces for a valueless token is put back here.
///
/// "With no child" is asked through [`Rule::has_child_value`] for the
/// reason `val_before_close` reads the child through it.
fn val_after_close(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    if rule.has_child_value() {
        return Ok(());
    }
    let openval = rule.u.get("openval").cloned().unwrap_or(Value::Undefined);
    if is_primitive(&openval) {
        set_node(rule, openval);
    } else if rule
        .o0()
        .is_some_and(|token| carries_no_value(token, &context.options))
    {
        set_node(rule, Value::Undefined);
    }
    Ok(())
}

fn bump(rule: &mut Rule, counter: &str) {
    let depth = rule.n.get(counter).copied().unwrap_or(0);
    rule.n_mut().insert(counter.to_string(), depth + 1);
}

fn map_before_open(rule: &mut Rule, _context: &mut Context) -> Result<(), ActionError> {
    bump(rule, "dmap");
    Ok(())
}

/// Increment the list depth and, for an implicit (bracket-less) list,
/// allocate the array and promote the already-parsed first value into it.
/// json's @array$ never runs here: its list-open alts only match `[`.
fn list_before_open(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    bump(rule, "dlist");
    let promoted = rule.prev_rule.as_ref().and_then(|prev| {
        matches!(prev.u.get("implist"), Some(Value::Bool(true))).then(|| prev.node.borrow().clone())
    });
    if let Some(first) = promoted {
        let first = if first.is_undefined() {
            Value::Null
        } else {
            first
        };
        let list = if context.options.info.list {
            Value::ListRef(Arc::new(ListRef {
                value: vec![first],
                implicit: true,
                child: None,
                meta: IndexMap::new(),
            }))
        } else {
            Value::array(vec![first])
        };
        // In place, not a fresh cell: a replacing rule shares the cell of
        // the rule it replaced, and that rule's snapshot is `prev`. This
        // is `r.prev.node = r.node` in the canonical grammar.
        *rule.node.borrow_mut() = list;
    }
    Ok(())
}

fn map_before_close(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    if context.options.info.map {
        let implicit = rule.o0().is_none_or(|token| token.tin != TIN_OB);
        if let Value::MapRef(map) = &mut *rule.node.borrow_mut() {
            Arc::make_mut(map).implicit = implicit;
        }
    }
    Ok(())
}

fn list_before_close(rule: &mut Rule, _context: &mut Context) -> Result<(), ActionError> {
    let implicit = rule.o0().is_none_or(|token| token.tin != TIN_OS);
    if let Value::ListRef(list) = &mut *rule.node.borrow_mut() {
        Arc::make_mut(list).implicit = implicit;
    }
    Ok(())
}

/// Set `key: value` on the node, the port of the TypeScript `pairval`.
/// The previous value at the key is read straight off the node so a
/// repeated key (`a:1,a:2`) or a deep object can merge or extend.
fn pairval(rule: &mut Rule, context: &mut Context) {
    let Some(Value::String(key)) = rule.u.get("key").cloned() else {
        return;
    };
    let value = child_or_null(rule);

    // Do not set unsafe keys on arrays (objects have no prototype chain).
    if flag(rule, "list")
        && context.options.safe.key
        && (key == "__proto__" || key == "constructor")
    {
        return;
    }

    // Drop keys that match the info marker to preserve metadata.
    if context.options.info.map && key == context.options.info.marker {
        return;
    }

    let previous = {
        let node = rule.node.borrow();
        map_get(&node, &key)
    };
    let value = match previous {
        None | Some(Value::Null) | Some(Value::Undefined) => value,
        Some(previous) => combine(previous, value, rule, context),
    };
    map_insert(&mut rule.node.borrow_mut(), key, value);
}

/// Store a bare-colon `:value` on the map's `child$` property, merging
/// with an earlier one the way a repeated key merges.
fn map_child(rule: &mut Rule, context: &mut Context) {
    let value = child_or_null(rule);
    let previous = {
        let node = rule.node.borrow();
        map_get(&node, "child$")
    };
    let value = match previous {
        None | Some(Value::Undefined) => value,
        Some(previous) => combine(previous, value, rule, context),
    };
    map_insert(&mut rule.node.borrow_mut(), "child$".to_string(), value);
}

/// Store a bare-colon `:value` on the list. A TypeScript array takes it as
/// a `child$` property; here the list becomes a [`ListRef`] carrying it
/// as `child`, the same wrapper `info.list` produces, so a list that
/// received one is distinguishable from its plain elements.
fn list_child(rule: &mut Rule, context: &mut Context) {
    let value = child_or_null(rule);
    let previous = {
        let node = rule.node.borrow();
        match &*node {
            Value::ListRef(list) => list.child.as_deref().cloned(),
            _ => None,
        }
    };
    let value = match previous {
        None => value,
        Some(previous) => combine(previous, value, rule, context),
    };
    let mut node = rule.node.borrow_mut();
    let mut list = match std::mem::replace(&mut *node, Value::Undefined) {
        Value::Array(items) => ListRef {
            value: unwrap_arc(items),
            implicit: false,
            child: None,
            meta: IndexMap::new(),
        },
        Value::ListRef(list) => unwrap_arc(list),
        other => {
            *node = other;
            return;
        }
    };
    list.child = Some(Box::new(value));
    *node = Value::ListRef(Arc::new(list));
}

fn pair_before_close(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    if flag(rule, "pair") {
        pairval(rule, context);
    }
    if flag(rule, "child") {
        map_child(rule, context);
    }
    Ok(())
}

/// jsonic's `elem` before-close: the done-guarded push (json's strict
/// version pushes every child, which would double-add the done-flagged
/// implicit nulls, pairs and children), then `list.pair` / `list.property`
/// pairs, then `list.child` values.
fn elem_before_close(rule: &mut Rule, context: &mut Context) -> Result<(), ActionError> {
    if !flag(rule, "done") && !rule.child_node.is_undefined() {
        let child = rule.child_node.clone();
        list_push(&mut rule.node.borrow_mut(), child);
    }

    if flag(rule, "pair") {
        if context.options.list.pair {
            // list.pair: push pair as object element into the list.
            if let Some(Value::String(key)) = rule.u.get("key").cloned() {
                let mut pair = IndexMap::new();
                pair.insert(key, child_or_null(rule));
                list_push(&mut rule.node.borrow_mut(), Value::object(pair));
            }
        } else {
            pairval(rule, context);
        }
    }

    if flag(rule, "child") {
        list_child(rule, context);
    }
    Ok(())
}

/// Capture the pair key from the first open token: the decoded value of a
/// text or string token, the SOURCE of anything else, so `1:x` keys "1"
/// and `true:1` keys "true".
fn pairkey(rule: &mut Rule, _context: &mut Context) -> Result<(), ActionError> {
    let Some(token) = rule.o0().cloned() else {
        return Ok(());
    };
    let key = if matches!(token.tin, TIN_ST | TIN_TX) {
        match token.val {
            Value::String(text) => text,
            Value::Text(text) => text.string,
            Value::Number(number) if number.fract() == 0.0 && number.is_finite() => {
                format!("{number:.0}")
            }
            other => other.to_string(),
        }
    } else {
        token.src.as_str().to_string()
    };
    rule.u_mut().insert("key".to_string(), Value::String(key));
    Ok(())
}

fn push_null(rule: &mut Rule, _context: &mut Context) -> Result<(), ActionError> {
    list_push(&mut rule.node.borrow_mut(), Value::Null);
    Ok(())
}

/// Register every closure the grammar document names. Registered before
/// the document is installed, because the document is what looks them
/// up; re-registering on an instance that already has them is harmless.
fn register_refs(parser: &mut Tabnas) {
    parser.alt_error(FINISH, |_rule, context| {
        if context.options.rule.finish {
            return None;
        }
        // The CODE, not just the rejection: with finish off an
        // unterminated structure is `end_of_source`, as in TypeScript.
        let mut token = context.t0()?.clone();
        token.bad("end_of_source");
        Some(token)
    });
    parser.alt_error(VAL_CLOSE_ERROR, |rule, context| {
        if rule.d == 0 {
            context.t0().cloned()
        } else {
            None
        }
    });
    parser.alt_error(ELEM_PAIR_ERR, |_rule, context| context.t0().cloned());
    parser.alt_error(ELEM_CLOSE_ERR, |rule, _context| rule.c0().cloned());

    parser.action_with_context(PAIRKEY, pairkey);
    parser.action_with_context(ELEM_DOUBLE_COMMA, push_null);
    parser.action_with_context(ELEM_SINGLE_COMMA, push_null);

    parser.state_action_ref(VAL_BC, val_before_close);
    parser.state_action_ref(VAL_AC, val_after_close);
    parser.state_action_ref(MAP_BO, map_before_open);
    parser.state_action_ref(MAP_BC, map_before_close);
    parser.state_action_ref(LIST_BO, list_before_open);
    parser.state_action_ref(LIST_BC, list_before_close);
    parser.state_action_ref(PAIR_BC, pair_before_close);
    parser.state_action_ref(ELEM_BC, elem_before_close);
}

/// Whether the relaxed grammar is already on this instance, judged by the
/// one thing only jsonic installs: its replacement of the `val`
/// before-close phase.
fn grammar_installed(parser: &Tabnas) -> bool {
    parser
        .rules
        .get("val")
        .is_some_and(|spec| spec.bc.iter().any(|action| action == VAL_BC))
}

// ---------------------------------------------------------------------------
// Public surface
// ---------------------------------------------------------------------------

/// Register the relaxed-JSON grammar rules (`val` / `map` / `list` /
/// `pair` / `elem`) on `parser` WITHOUT applying jsonic's option branding,
/// so other grammar plugins can layer their own syntax on the jsonic core
/// while managing their own option set. The counterpart of the TypeScript
/// `registerJsonicGrammar` export and the Go `RegisterJsonicGrammar`.
///
/// The standard-JSON core is installed first, through
/// [`tabnas_json::json`], then the relaxed alternates and lifecycle
/// actions are woven around it, and a parse budget refusing nesting past
/// 127 containers (with the engine's `cancel` code) goes on last, unless
/// the instance already carries a budget of its own. Registration is
/// idempotent: an instance that already carries the grammar is left
/// alone, so a plugin re-run cannot double the alternates.
///
/// ```
/// let mut parser = tabnas::Tabnas::new();
/// tabnas_jsonic::register_jsonic_grammar(&mut parser)?;
/// assert_eq!(parser.parse("a:b:1")?.to_string(), r#"{"a":{"b":1}}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn register_jsonic_grammar(parser: &mut Tabnas) -> Result<(), GrammarError> {
    if grammar_installed(parser) {
        return Ok(());
    }
    register_refs(parser);

    // tabnas_json's one entry point applies its strict OPTIONS with its
    // rules. jsonic wants the rules alone, so the profile in force before
    // the call is put back after it.
    let relaxed = parser.config();
    tabnas_json::json(parser)?;
    parser
        .set_options(|options| restore_relaxed(options, &relaxed))
        .map_err(|error| GrammarError(error.0))?;

    let options = parser.config();
    let phase_one = GrammarSpec::from_value(jsonic_document(&options))?;
    parser.grammar(&phase_one)?;
    let phase_two = GrammarSpec::from_value(jsonic_document_append())?;
    parser.grammar(&phase_two)?;

    // AFTER the documents, as tabnas_json sets its own: `grammar` applies
    // a document's options, and an options pass that does not mention
    // `parse.budget` is not required to preserve one. A budget the caller
    // set before the grammar (through `make_with`) survived
    // `restore_relaxed` above and wins; only an instance with none gets
    // jsonic's. Every iteration, because a sampled check would let the
    // parse run past the limit by however many levels the sample missed.
    if parser.config().parse.budget.on_check.is_none() {
        parser.parse_budget(1, within_depth_limit);
    }
    Ok(())
}

/// Install jsonic on `parser`: the idiomatic engine plugin.
///
/// On a bare engine it applies jsonic's option defaults (the engine
/// already lexes relaxed JSON; this adds the jsonic error and hint
/// branding) and then registers the relaxed-JSON grammar. Use it before
/// any plugin that builds on jsonic's rules.
///
/// ```
/// let mut parser = tabnas::Tabnas::new();
/// tabnas_jsonic::jsonic(&mut parser)?;
/// let value = parser.parse("a:1,b:[x,y,z]")?;
/// assert_eq!(value.to_string(), r#"{"a":1,"b":["x","y","z"]}"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn jsonic(parser: &mut Tabnas) -> Result<(), GrammarError> {
    apply_options(parser)?;
    register_jsonic_grammar(parser)
}

/// The plugin form of [`jsonic`], for [`Tabnas::use_plugin`]. Installed
/// this way the grammar is re-applied to derived instances, as every
/// native plugin is.
///
/// ```
/// let mut parser = tabnas::Tabnas::new();
/// parser.use_plugin(tabnas_jsonic::plugin(), None)?;
/// assert_eq!(parser.parse("x,y,z")?.to_string(), r#"["x","y","z"]"#);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn plugin() -> Plugin {
    Plugin::new(PLUGIN_NAME, |parser, _options| {
        jsonic(parser).map_err(|error| PluginError(error.0))
    })
}

/// The grammar-only plugin [`make_with`] registers, so that a derived
/// instance rebuilds the grammar against its own options without the
/// branding overwriting a caller's error identity. The Go `Make` registers
/// its `grammarPlugin` for the same reason.
fn grammar_plugin() -> Plugin {
    Plugin::new(PLUGIN_NAME, |parser, _options| {
        register_jsonic_grammar(parser).map_err(|error| PluginError(error.0))
    })
}

/// Build a relaxed-JSON parser with caller options: the counterpart of
/// `Jsonic.make(options)` and the Go `Make(opts)`.
///
/// jsonic's branding goes on first, then `configure` runs over the
/// options, so a caller's own `errmsg.name` or lexer setting wins, and
/// then the grammar is installed against the finished options, which is
/// what decides the option-conditional alternates (`map.child`,
/// `list.child`, `list.pair`). `rule.include` and `rule.exclude` are
/// honoured by the engine at parse time, so they may be set here too.
///
/// Infallible by design: the documents are fixed literals, so a failure
/// here is a bug in this crate rather than anything a caller did.
///
/// ```
/// let parser = tabnas_jsonic::make_with(|options| options.number.lex = false);
/// assert_eq!(parser.parse("a:1, b:2")?.to_string(), r#"{"a":"1","b":"2"}"#);
/// # Ok::<(), tabnas_jsonic::JsonicError>(())
/// ```
pub fn make_with(configure: impl FnOnce(&mut Options)) -> Tabnas {
    let mut parser = Tabnas::new();
    apply_options(&mut parser).expect("the jsonic option document is fixed and valid");
    parser
        .set_options(configure)
        .expect("the caller's options apply to a fresh engine");
    parser
        .use_plugin(grammar_plugin(), None)
        .expect("the jsonic grammar documents are fixed and valid");
    parser
}

/// Build a relaxed-JSON parser with the default options.
///
/// ```
/// let parser = tabnas_jsonic::make();
/// assert_eq!(parser.parse("a:b:c:1")?.to_string(), r#"{"a":{"b":{"c":1}}}"#);
/// assert!(parser.parse("}").is_err());
/// # Ok::<(), tabnas_jsonic::JsonicError>(())
/// ```
pub fn make() -> Tabnas {
    make_with(|_| {})
}

/// Exactly a standard JSON number.
fn strict_number() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?$")
            .expect("the strict-number pattern is a literal and compiles")
    })
}

/// The candidate literal starting at `src`, up to the next JSON structural
/// character or whitespace: the run the engine's lenient number matcher
/// would consider.
fn leading_literal(src: &str) -> &str {
    let end = src
        .find(|c: char| c.is_whitespace() || matches!(c, ',' | '}' | ']' | ':' | '"' | '/'))
        .unwrap_or(src.len());
    &src[..end]
}

/// Reject anything the engine's lenient number matcher would accept that
/// standard JSON does not: a leading `+`, leading zeros, a bare leading
/// or trailing `.`. The TypeScript `number.exclude` is a negative
/// lookahead the `regex` crate cannot express, so the positive pattern
/// plus an inversion lives in a `check` hook, the shape the Go port uses.
fn strict_number_check(src: &str) -> LexCheckResult {
    let literal = leading_literal(src);
    let starts_number = literal
        .chars()
        .next()
        .is_some_and(|c| c == '-' || c == '+' || c == '.' || c.is_ascii_digit());
    if !starts_number || strict_number().is_match(literal) {
        LexCheckResult::Continue
    } else {
        LexCheckResult::Skip
    }
}

/// The strict-JSON option profile of `Jsonic.make('json')`, as one
/// document so the number hook can be bound by name.
fn strict_json_document() -> serde_json::Value {
    json!({
        "options": {
            "text": { "lex": false },
            "number": {
                "hex": false, "oct": false, "bin": false, "sep": null,
                "check": STRICT_NUMBER_CHECK,
            },
            "string": {
                "chars": "\"",
                "multiChars": "",
                "allowUnknown": false,
                "escape": { "v": null },
                // No `\xHH` or `\u{...}`; strict JSON has only `\uXXXX`.
                "escapeStrict": true,
            },
            "comment": { "lex": false },
            "map": { "extend": false },
            "lex": { "empty": false },
            "rule": { "finish": false, "include": "json" },
            // Strict JSON keys are quoted strings only, never text,
            // numbers or keywords: `{1:1}` and `{null:null}` are errors.
            //
            // The three trailing nulls are load-bearing, and are exactly
            // what the canonical `ts/src/grammar.ts` writes. The engine
            // overlays a token set INDEX-WISE, as TypeScript's deep merge
            // treats an array: a null clears that position and the
            // positions beyond the overlay's length keep the default. A
            // bare `["#ST"]` therefore replaces only slot 0 of the default
            // `KEY: ['#TX', '#NR', '#ST', '#VL']` and leaves `#NR`, `#ST`
            // and `#VL` live, so the strict parser still accepts `{1:1}`.
            "tokenSet": { "KEY": ["#ST", null, null, null] },
        },
    })
}

/// Build a parser that accepts strict JSON only: the counterpart of
/// `Jsonic.make('json')` and the Go `MakeJSON`.
///
/// It installs the full jsonic grammar, restricts it to the `json`-tagged
/// alternates and tightens the lexer: no unquoted keys or values, no
/// comments, no hex, octal or binary numbers, no leading zeros, no
/// trailing commas, no single or backtick quoted strings, no empty input.
///
/// ```
/// let strict = tabnas_jsonic::make_json();
/// assert_eq!(strict.parse(r#"{"a":[1,2]}"#)?.to_string(), r#"{"a":[1,2]}"#);
/// assert!(strict.parse("a:1,").is_err());
/// assert!(strict.parse("[01]").is_err());
/// # Ok::<(), tabnas_jsonic::JsonicError>(())
/// ```
pub fn make_json() -> Tabnas {
    let mut parser = Tabnas::new();
    apply_options(&mut parser).expect("the jsonic option document is fixed and valid");
    parser.lex_check_ref(STRICT_NUMBER_CHECK, strict_number_check);
    let strict = GrammarSpec::from_value(strict_json_document())
        .expect("the strict-JSON option document is fixed and valid");
    parser
        .grammar(&strict)
        .expect("the strict-JSON options apply to a fresh engine");
    parser
        .use_plugin(grammar_plugin(), None)
        .expect("the jsonic grammar documents are fixed and valid");
    parser
}

/// A parser with the jsonic configuration but no grammar rules, for
/// building a grammar from scratch. The counterpart of `Jsonic.empty()`
/// and the Go `Empty`.
pub fn empty() -> Tabnas {
    let mut parser = make();
    for name in parser.rule_names() {
        parser.define_rule(name, |spec| {
            spec.clear();
        });
    }
    parser
}

/// Parse a jsonic source string with the shared default parser.
///
/// The engine is built once, on first use, and reused after that, as the
/// other two runtimes do (`sync.Once` in `go/jsonic.go`, the root
/// instance in `ts/src/jsonic.ts`). Reuse is safe: [`Tabnas::parse`]
/// takes `&self` and builds a fresh parse context per call, and `Tabnas`
/// is `Send + Sync`, so concurrent callers share one installed grammar
/// instead of each rebuilding it, which is what dominates a small parse.
///
/// Use [`make`] or [`make_with`] instead when the parser needs
/// configuring: that returns a fresh instance and leaves this one alone.
///
/// ```
/// let value = tabnas_jsonic::parse("a:1, b:2")?;
/// assert_eq!(value.to_string(), r#"{"a":1,"b":2}"#);
/// assert_eq!(tabnas_jsonic::parse("]").unwrap_err().code, "unexpected");
/// # Ok::<(), tabnas_jsonic::JsonicError>(())
/// ```
pub fn parse(src: &str) -> Result<Value, JsonicError> {
    static DEFAULT: OnceLock<Tabnas> = OnceLock::new();
    DEFAULT.get_or_init(make).parse(src)
}
