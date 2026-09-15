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
            "map": {
                "open": {
                    // Auto-close `{` at end of source, allocating the
                    // empty object so `{` -> `{}` when finish is allowed.
                    "alts": [
                        { "s": "#OB #ZZ", "b": 1, "a": "@object$",
                          "e": "@finish", "g": "end,jsonic" },

                        // The brace-less entry to a map. json's map opens
                        // only match `#OB`, so this path allocates the
                        // container itself: `@object$` with the static
                        // implicit flag, the counterpart of json's
                        // `implicit: false`. Without it `a:1` has no way
                        // into the map rule at all.
                        //
                        // The canonical grammar adds this in a SECOND
                        // `.open()` call with `append: true`, after json's
                        // `#OB` alts. Here both groups are one ordered
                        // list, and it is placed after the auto-close alt
                        // for the same reason: `#OB` must still win when
                        // a brace is actually present.
                        { "s": "#KEY #CL", "p": "pair", "b": 2, "a": "@object$",
                          "k": { "object$": { "implicit": true } },
                          "g": "pair,list,val,imp,jsonic" },
                    ],
                    "inject": { "append": true },
                },
                // A SECOND open list, appended rather than prepended: the
                // brace-less entry to a map. json's map opens only match
                // `#OB`, so this path has to allocate the container
                // itself -- `@object$` with the static implicit flag,
                // the brace-less counterpart of json's `implicit: false`.
                // Without it `a:1` has no way into the map rule at all.
                "close": {
                    "alts": [
                        // Normal end of map, no path dive.
                        { "s": "#CB", "c": { "n.pk": { "$lte": 0 } }, "g": "end,json" },
                        // Mid path dive: keep ascending.
                        { "s": "#CB", "b": 1, "g": "path,close,jsonic" },
                        // End of an implicit path.
                        { "s": ["#CA #CS #VAL"], "b": 1, "g": "end,path,jsonic" },
                        { "s": "#ZZ", "e": "@finish", "g": "end,jsonic" },
                    ],
                    "inject": { "append": true, "delete": [0] },
                },
            },

            "list": {
                "open": {
                    // A bracket-less list promoted by `@list-bo`.
                    "alts": [
                        { "c": { "prev.u.implist": { "$eq": true } }, "p": "elem" },
                    ],
                },
                "close": {
                    "alts": [
                        { "s": "#ZZ", "e": "@finish", "g": "end,jsonic" },
                    ],
                    "inject": { "append": true },
                },
            },

            "pair": {
                "open": {
                    "alts": [
                        // Re-declared, not reused. json parsed its own
                        // `#KEY #CL` alt while `tokenSet.KEY` was locked
                        // to `["#ST"]`, so that alternate is FROZEN to
                        // quoted keys and widening the set cannot reach
                        // it. Re-declaring here, after the widening, is
                        // what makes `{a:1}` parse -- and it binds
                        // `@pairkey`, which keys on the token source for
                        // numbers and keywords where json uses the
                        // decoded value.
                        { "s": "#KEY #CL", "p": "val", "u": { "pair": true },
                          "a": "@pairkey", "g": "map,pair,key,json" },

                        // Ignore a leading comma: `{,a:1}`.
                        { "s": "#CA", "g": "map,pair,comma,jsonic" },
                    ],
                    // `clear` drops json's pair opens first; see above.
                    "inject": { "append": true, "clear": true },
                },
                "close": {
                    "alts": [
                        // End of map: reset the implicit depth counter so
                        // `a:b:c:1,d:2` -> {"a":{"b":{"c":1}},"d":2}.
                        { "s": "#CB", "c": { "n.pk": { "$lte": 0 } }, "b": 1,
                          "g": "map,pair,close,json" },
                        // Trailing comma at end of map.
                        { "s": "#CA #CB", "c": { "n.pk": { "$lte": 0 } }, "b": 1,
                          "g": "map,pair,comma,jsonic" },
                        // A SEQUENCE of two tokens (comma then end of
                        // source), not one token that is either. Written
                        // as `["#CA #ZZ"]` it matches any lone comma and
                        // ends the pair, so `{a:1,b:2}` stops at the
                        // comma.
                        { "s": ["#CA", "#ZZ"], "g": "end,jsonic" },
                        // A comma starts a new pair at the same level.
                        { "s": "#CA", "c": { "n.pk": { "$lte": 0 } }, "r": "pair",
                          "g": "map,pair,sync,json" },
                        { "s": "#CA", "c": { "n.dmap": { "$lte": 1 } }, "r": "pair",
                          "g": "map,pair,sync,jsonic" },
                        // A key starts a new pair in an implicit top map.
                        { "s": "#KEY", "c": { "n.dmap": { "$lte": 1 } }, "r": "pair",
                          "b": 1, "g": "map,pair,imp,sync,jsonic" },
                        // End of an implicit path: keep closing to pk=0.
                        { "s": ["#CB #CA #CS #KEY"], "c": { "n.pk": { "$gt": 0 } },
                          "b": 1, "g": "map,pair,imp,path,close,jsonic" },
                        // A `]` cannot close a map.
                        { "s": "#CS", "e": "@close-mismatch", "g": "end,jsonic" },
                        { "s": "#ZZ", "e": "@finish", "g": "map,pair,end,json" },
                        // Who needs commas anyway.
                        { "r": "pair", "b": 1, "g": "map,pair,imp,jsonic" },
                    ],
                    "inject": { "append": true, "delete": [0, 1] },
                },
            },

            "elem": {
                "open": {
                    "alts": [
                        // Empty commas insert nulls. Close consumes one
                        // comma, which is why `b: 2` is right here.
                        { "s": "#CA #CA", "b": 2, "u": { "done": true },
                          "a": "@elem-push-null", "g": "list,elem,imp,null,jsonic" },
                        { "s": "#CA", "u": { "done": true },
                          "a": "@elem-push-null", "g": "list,elem,imp,null,jsonic" },
                        // A pair inside a list: `[a:1]`.
                        { "s": "#KEY #CL", "p": "val", "n": { "pk": 1, "dmap": 1 },
                          "u": { "done": true, "pair": true, "list": true },
                          "a": "@pairkey", "g": "elem,pair,jsonic" },
                    ],
                },
                "close": {
                    "alts": [
                        // Trailing comma.
                        { "s": ["#CA", "#CS #ZZ"], "b": 1, "g": "list,elem,comma,jsonic" },
                        { "s": "#CA", "r": "elem", "g": "list,elem,sync,json" },
                        { "s": "#CS", "b": 1, "g": "list,elem,close,json" },
                        { "s": "#ZZ", "e": "@finish", "g": "list,elem,end,json" },
                        // A `}` cannot close a list.
                        { "s": "#CB", "e": "@close-mismatch", "g": "end,jsonic" },
                        // Who needs commas anyway.
                        { "r": "elem", "b": 1, "g": "list,elem,imp,jsonic" },
                    ],
                    "inject": { "delete": [-1, -2] },
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

/// Write `key: value` onto the rule's node, merging with any previous
/// value at that key.
///
/// The previous value is read STRAIGHT OFF THE NODE rather than threaded
/// through the rule, which is what lets a repeated key (`a:1,a:2`) or a
/// deep object (`a:b:1,a:c:2`) merge rather than clobber.
fn pairval(rule: &mut tabnas::Rule, context: &tabnas::Context) {
    let key = match rule.u.get("key") {
        Some(Value::String(key)) => key.clone(),
        _ => return,
    };

    // Unsafe keys are not set on a list, so a crafted document cannot
    // reach a prototype through the list path.
    if rule.u.get("list") == Some(&Value::Bool(true))
        && context.options.safe.key
        && (key == "__proto__" || key == "constructor")
    {
        return;
    }

    let mut val = rule.child_node.clone();
    if val == Value::Undefined {
        val = Value::Null;
    }

    let extend = context.options.map.extend;
    if let Value::Object(map) = &mut *rule.node.borrow_mut() {
        let merged = match map.get(&key) {
            Some(prev) if !matches!(prev, Value::Null | Value::Undefined) && extend => {
                deep_merge(prev.clone(), val)
            }
            _ => val,
        };
        map.insert(key, merged);
    }
}

/// Recursive object merge: `incoming` wins on a leaf, and two objects at
/// the same key merge rather than replace.
fn deep_merge(prev: Value, incoming: Value) -> Value {
    match (prev, incoming) {
        (Value::Object(mut base), Value::Object(over)) => {
            for (key, value) in over {
                let merged = match base.shift_remove(&key) {
                    Some(existing) => deep_merge(existing, value),
                    None => value,
                };
                base.insert(key, merged);
            }
            Value::Object(base)
        }
        (_, incoming) => incoming,
    }
}

/// A one-entry object, for `list.pair`.
///
/// Built through `serde_json` rather than by naming the engine's map type
/// directly, which would mean depending on `indexmap` at a version that
/// has to track the engine's. This crate already enables `preserve_order`,
/// so key order survives the round trip.
fn single_entry_object(key: String, value: Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(key, value.to_json());
    Value::from_json(&serde_json::Value::Object(object))
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

    // `@finish`: auto-closing an unterminated structure at end of source
    // is allowed only when `rule.finish` is on. When it is off, the
    // remaining structure is an error rather than something to close.
    parser.alt_error("@finish", |_rule, context| {
        if context.options.rule.finish {
            return None;
        }
        context.t.first().cloned().map(|mut token| {
            token.err = "end_of_source".to_string();
            token
        })
    });

    // `@close-mismatch`: a `]` cannot close a map, nor a `}` a list. The
    // canonical grammar writes this as an inline `(r) => r.c0`.
    parser.alt_error("@close-mismatch", |rule, _context| rule.c.first().cloned());

    // `@pairkey`: capture the key from the first open token.
    //
    // A quoted or unquoted string uses the DECODED value; anything else
    // (a number, a value keyword) uses the ORIGINAL SOURCE, so `1:x` keys
    // on "1" rather than on the number, and `__proto__:1` keeps its text.
    // json's own key action uses the decoded value throughout, which is
    // why this alternate has to be re-declared rather than reused.
    parser.action_with_match_ref("@pairkey", |rule, _context, _matched| {
        if let Some(token) = rule.o.first() {
            let key = match token.name.as_str() {
                "#ST" | "#TX" => match &token.val {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                },
                _ => token.src.clone(),
            };
            rule.u.insert("key".to_string(), Value::String(key));
        }
        Ok(None)
    });

    // `@map-bo` / `@list-bo`: container depth counters. The relaxed
    // alternates gate on these (`n.dmap`, `n.dlist`) to keep implicit
    // structures to the top level.
    parser.state_action_ref("@map-bo", |rule, _context| {
        let depth = rule.n.get("dmap").copied().unwrap_or(0);
        rule.n.insert("dmap".to_string(), depth + 1);
        Ok(())
    });

    parser.state_action_ref("@list-bo", |rule, _context| {
        let depth = rule.n.get("dlist").copied().unwrap_or(0);
        rule.n.insert("dlist".to_string(), depth + 1);

        // A bracket-less list: json's `@array$` only runs for `[`, so the
        // array is allocated here and the value already parsed as the
        // first element is promoted into it.
        let implist = rule
            .prev_rule
            .as_ref()
            .and_then(|prev| prev.u.get("implist"))
            .map(|flag| flag == &Value::Bool(true))
            .unwrap_or(false);
        if implist {
            if let Some(prev) = rule.prev_rule.clone() {
                let first = prev.node.borrow().clone();
                let promoted = Value::Array(vec![first]);
                *rule.node.borrow_mut() = promoted.clone();
                *prev.node.borrow_mut() = promoted;
            }
        }
        Ok(())
    });

    // `@val-bc/replace`: jsonic's val close coalescing.
    //
    // `/replace` takes ownership of the phase, so json's strict `@val-bc`
    // -- which simply overwrites the node from the matched token -- is
    // cleared rather than left to run alongside. The order below is the
    // whole of it, and each rung exists for a case the one above cannot
    // see:
    //
    //   1. a child container (the value WAS a map or list)
    //   2. else a deliberate PRIMITIVE a plugin set in a val open action.
    //      A stale parent-seeded node is always a container, so a
    //      non-object node here can only be intentional.
    //   3. else the matched scalar token. This beats a stale
    //      parent-seeded container.
    //   4. else a deliberate CONTAINER a plugin set (no token matched)
    //   5. else nothing, which is the implicit null.
    parser.state_action_ref("@val-bc/replace", |rule, context| {
        // Stash a plugin's open-action value before any coalescing; the
        // after-close hook restores it.
        let open_value = rule.node.borrow().clone();
        rule.u.insert("openval".to_string(), open_value.clone());

        let resolved = if rule.child_node != Value::Undefined {
            rule.child_node.clone()
        } else if !matches!(
            open_value,
            Value::Undefined | Value::Object(_) | Value::Array(_)
        ) && open_value != Value::Null
        {
            open_value
        } else if let Some(token) = rule.o.first().cloned() {
            token.resolve_val(rule, context)
        } else if open_value != Value::Undefined {
            open_value
        } else {
            Value::Undefined
        };

        *rule.node.borrow_mut() = resolved;
        Ok(())
    });

    // `@val-ac`: json's `@value$` close ALT action runs after the phase
    // above and re-resolves the matched token, which would overwrite a
    // value a plugin set in a val OPEN action. Restore it -- but only a
    // PRIMITIVE one with no child, since a parent-seeded stale node is
    // always a container.
    parser.state_action_ref("@val-ac", |rule, _context| {
        let restore = match rule.u.get("openval") {
            Some(value)
                if !matches!(
                    value,
                    Value::Undefined | Value::Null | Value::Object(_) | Value::Array(_)
                ) =>
            {
                Some(value.clone())
            }
            _ => None,
        };
        if let Some(value) = restore {
            if rule.child_node == Value::Undefined {
                *rule.node.borrow_mut() = value;
            }
        }
        Ok(())
    });

    // `@pair-bc` / `@elem-bc/replace`: the value-building layer.
    //
    // json's strict closes write the value for its own alternates. The
    // relaxed alternates above bypass those, so without these two the
    // grammar MATCHES more and PRODUCES less: clearing json's pair opens
    // in favour of `@pairkey` (which only captures the key) drops the
    // write entirely.
    parser.state_action_ref("@pair-bc", |rule, context| {
        if rule.u.get("pair") == Some(&Value::Bool(true)) {
            pairval(rule, context);
        }
        Ok(())
    });

    // `/replace` takes ownership of the phase: json's strict `@elem-bc`
    // pushes EVERY child node, which would double-add the done-flagged
    // elements jsonic produces (implicit nulls, pairs).
    parser.state_action_ref("@elem-bc/replace", |rule, context| {
        let done = rule.u.get("done") == Some(&Value::Bool(true));
        if !done && rule.child_node != Value::Undefined {
            let child = rule.child_node.clone();
            if let Value::Array(items) = &mut *rule.node.borrow_mut() {
                items.push(child);
            }
        }
        if rule.u.get("pair") == Some(&Value::Bool(true)) {
            if context.options.list.pair {
                // list.pair: the pair becomes an object element.
                let key = match rule.u.get("key") {
                    Some(Value::String(key)) => key.clone(),
                    _ => return Ok(()),
                };
                let mut val = rule.child_node.clone();
                if val == Value::Undefined {
                    val = Value::Null;
                }
                let pair = single_entry_object(key, val);
                if let Value::Array(items) = &mut *rule.node.borrow_mut() {
                    items.push(pair);
                }
            } else {
                pairval(rule, context);
            }
        }
        Ok(())
    });

    // `@elem-push-null`: an empty comma inserts a null element.
    parser.action_with_match_ref("@elem-push-null", |rule, _context, _matched| {
        if let Value::Array(items) = &mut *rule.node.borrow_mut() {
            items.push(Value::Null);
        }
        Ok(None)
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
