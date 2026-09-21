// In-language tests: the behaviour the shared fixtures cannot pin. Each
// section names the Go test it mirrors (`go/*_test.go`) or the
// TypeScript one, so the three suites can be read side by side.

use std::sync::Arc;
use std::time::Instant;

use indexmap::IndexMap;
use tabnas::{GrammarSpec, Plugin, Tabnas, Text, Value, ValueDef};
use tabnas_jsonic::{
    deep_merge, empty, jsonic, make, make_json, make_with, parse, plugin, register_jsonic_grammar,
};

/// A value as compact JSON, which is how every expectation below is
/// written. Whole numbers are spelt as integers, as `JSON.stringify` and
/// `encoding/json` spell them, so the expectations read the same as the
/// Go and TypeScript ones.
fn json(value: &Value) -> String {
    fn integral(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Number(number) => match number.as_f64() {
                Some(f) if f.fract() == 0.0 && f.abs() < 9.0e15 => {
                    serde_json::Value::Number((f as i64).into())
                }
                _ => serde_json::Value::Number(number),
            },
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.into_iter().map(integral).collect())
            }
            serde_json::Value::Object(entries) => serde_json::Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, integral(value)))
                    .collect(),
            ),
            other => other,
        }
    }
    serde_json::to_string(&integral(value.to_json())).expect("engine values are JSON")
}

fn parsed(parser: &Tabnas, src: &str) -> String {
    json(
        &parser
            .parse(src)
            .unwrap_or_else(|error| panic!("{src:?}: {error}")),
    )
}

// --- README examples (go/readme_test.go, README.md) -------------------

#[test]
fn readme_quick_example() {
    assert_eq!(json(&parse("a:1, b:2").unwrap()), r#"{"a":1,"b":2}"#);
}

#[test]
fn readme_syntax_examples_all_read_the_same_document() {
    for src in [
        "a:1,b:B",
        "a:1\nb:B",
        "a:1\n// a:2\n# a:3\n/* b wants\n * to B\n */\nb:B",
    ] {
        assert_eq!(json(&parse(src).unwrap()), r#"{"a":1,"b":"B"}"#, "{src:?}");
    }
}

#[test]
fn readme_relaxations() {
    for (src, want) in [
        ("a:1", r#"{"a":1}"#),
        ("a:1,b:2", r#"{"a":1,"b":2}"#),
        ("a:1 b:2", r#"{"a":1,"b":2}"#),
        ("first-name: Sam", r#"{"first-name":"Sam"}"#),
        ("a,b", r#"["a","b"]"#),
        ("1, 2, 3", "[1,2,3]"),
        ("[x y z]", r#"["x","y","z"]"#),
        ("{a:1,b:2,}", r#"{"a":1,"b":2}"#),
        ("[1, 2", "[1,2]"),
        ("{a:{b:1", r#"{"a":{"b":1}}"#),
        ("a:", r#"{"a":null}"#),
        ("a:b:c:1", r#"{"a":{"b":{"c":1}}}"#),
        ("a:b:1, a:c:2", r#"{"a":{"b":1,"c":2}}"#),
        ("x:{a:1}, x:{b:2}", r#"{"x":{"a":1,"b":2}}"#),
        ("a:1,a:2", r#"{"a":2}"#),
        ("'hello'", r#""hello""#),
        ("`hello`", r#""hello""#),
        ("`line one\nline two`", r#""line one\nline two""#),
        ("{'b': `\\x42`}", r#"{"b":"B"}"#),
        ("0xFF", "255"),
        ("0o17", "15"),
        ("0b1010", "10"),
        ("1e3", "1000"),
        ("1_000_000", "1000000"),
        ("1a", r#""1a""#),
        ("1.2.3", r#""1.2.3""#),
        (
            "a:1   # hash comment\nb:2   // slash comment\nc:3   /* block\n comment */",
            r#"{"a":1,"b":2,"c":3}"#,
        ),
    ] {
        assert_eq!(json(&parse(src).unwrap()), want, "{src:?}");
    }
}

#[test]
fn readme_configured_instance() {
    // Disabling numbers makes them strings.
    let parser = make_with(|o| o.number.lex = false);
    assert_eq!(parsed(&parser, "a:1, b:2"), r#"{"a":"1","b":"2"}"#);

    // A custom number separator.
    let relaxed = make_with(|o| o.number.sep = Some(" ".into()));
    assert_eq!(parsed(&relaxed, "a: 1 000 000"), r#"{"a":1000000}"#);
}

// --- The idiomatic plugin (go/tabnas_plugin_test.go) --------------------

#[test]
fn the_plugin_installs_on_a_bare_engine() {
    let mut parser = Tabnas::new();
    jsonic(&mut parser).expect("installs");
    assert_eq!(
        parsed(&parser, "a:1,b:[x,y,z],c:{d:e} // tail"),
        r#"{"a":1,"b":["x","y","z"],"c":{"d":"e"}}"#
    );
    assert_eq!(parsed(&parser, "a:b:c:1"), r#"{"a":{"b":{"c":1}}}"#);

    let mut used = Tabnas::new();
    used.use_plugin(plugin(), None).expect("installs");
    assert_eq!(parsed(&used, "x,y,z"), r#"["x","y","z"]"#);
    assert_eq!(
        used.installed_plugins()
            .iter()
            .map(|p| p.name.clone())
            .collect::<Vec<_>>(),
        ["jsonic"]
    );
}

#[test]
fn the_plugin_applies_the_jsonic_branding() {
    // The engine ships the relaxed lexer defaults; the plugin layers on
    // the jsonic error identity, so failures carry the [jsonic/...] tag
    // and the jsonic hint and link.
    let mut parser = Tabnas::new();
    jsonic(&mut parser).expect("installs");
    let error = parser.parse("\"unterminated").unwrap_err();
    assert_eq!(error.code, "unterminated_string");
    let report = error.to_string();
    assert!(report.contains("[jsonic/unterminated_string]"), "{report}");
    assert!(report.contains("This string has no end quote."), "{report}");
    assert!(
        report.contains("https://github.com/tabnas/jsonic"),
        "{report}"
    );

    // make() carries the same identity.
    assert!(make()
        .parse("}")
        .unwrap_err()
        .to_string()
        .contains("[jsonic/unexpected]"));
}

#[test]
fn a_caller_option_wins_over_the_branding_in_make() {
    // Go's Make merges jsonic's branding as the base, so a caller's
    // errmsg.name survives; make_with does the same.
    let parser = make_with(|o| o.errmsg.name = "mine".into());
    assert!(parser
        .parse("}")
        .unwrap_err()
        .to_string()
        .contains("[mine/unexpected]"));
}

#[test]
fn a_plugin_layers_on_the_grammar_through_use_plugin() {
    // A second plugin builds on the grammar jsonic registered: keyword
    // values on top of jsonic's map/value rules. Register jsonic first.
    let yesno = Plugin::new("yesno", |parser, _options| {
        parser
            .set_options(|o| {
                for (word, value) in [("yes", true), ("no", false)] {
                    o.value.definitions.insert(
                        word.to_string(),
                        ValueDef {
                            val: Some(Value::Bool(value)),
                            matcher: None,
                            transform: None,
                            consume: false,
                        },
                    );
                }
            })
            .map(|_| ())
    });

    let mut parser = Tabnas::new();
    parser.use_plugin(plugin(), None).expect("jsonic");
    parser.use_plugin(yesno, None).expect("yesno");
    assert_eq!(
        parsed(&parser, "a:yes,b:no,c:[yes,no]"),
        r#"{"a":true,"b":false,"c":[true,false]}"#
    );
}

#[test]
fn a_plugin_extends_the_grammar_rules_through_a_document() {
    // A plugin adding syntax on top of the relaxed rules: a `~` token
    // that reads as the value 42, through a fresh val open alternate. The
    // val close coalescing then keeps the value the action set, which is
    // what the @val-bc/replace and @val-ac pair exist for.
    let answer = Plugin::new("answer", |parser, _options| {
        parser.token_with_source("#TILDE", "~");
        parser.action("@answer", |rule| {
            rule.node = std::rc::Rc::new(std::cell::RefCell::new(Value::Number(42.0)));
        });
        let spec = GrammarSpec::from_value(serde_json::json!({
            "rule": { "val": { "open": [
                { "s": "#TILDE", "a": "@answer", "g": "answer" }
            ] } }
        }))
        .map_err(|error| tabnas::PluginError(error.0))?;
        parser
            .grammar(&spec)
            .map(|_| ())
            .map_err(|error| tabnas::PluginError(error.0))
    });

    let mut parser = make();
    parser.use_plugin(answer, None).expect("answer");
    assert_eq!(parsed(&parser, "a:~,b:[~,1]"), r#"{"a":42,"b":[42,1]}"#);
    assert_eq!(parsed(&parser, "~"), "42");
}

#[test]
fn derive_rebuilds_the_grammar_against_the_child_options() {
    // A derived instance re-runs the registered grammar plugin, so the
    // child parses with the relaxed grammar under its own options, as a
    // TypeScript `instance.make(options)` child does.
    let parent = make();
    let child = parent.derive(|o| o.number.lex = false).expect("derives");
    assert_eq!(parsed(&child, "a:1, b:2"), r#"{"a":"1","b":"2"}"#);
    assert_eq!(parsed(&parent, "a:1, b:2"), r#"{"a":1,"b":2}"#);
}

#[test]
fn registering_the_grammar_twice_is_a_no_op() {
    let mut parser = make();
    let before: Vec<(String, usize, usize)> = parser
        .rule_specs()
        .iter()
        .map(|spec| (spec.name.clone(), spec.open.len(), spec.close.len()))
        .collect();
    jsonic(&mut parser).expect("installs again");
    register_jsonic_grammar(&mut parser).expect("registers again");
    let after: Vec<(String, usize, usize)> = parser
        .rule_specs()
        .iter()
        .map(|spec| (spec.name.clone(), spec.open.len(), spec.close.len()))
        .collect();
    assert_eq!(before, after);
    assert_eq!(parsed(&parser, "a:1,b:[2]"), r#"{"a":1,"b":[2]}"#);
}

#[test]
fn a_plugin_may_give_a_fixed_token_a_value_in_val_open() {
    // ts/test/custom.test.js 'parser-mixed-token': a val open alternate
    // matching a fixed token followed by text assigns the fixed token a
    // value (`r.o0.val = '@' + r.o1.val`), and an elem close alternate
    // starts the next element on the same pair. The value has to survive
    // jsonic's val coalescing and json's @value$ re-resolution, in both
    // hooks: this engine's fixed tokens carry their source text as `val`,
    // and only that untouched text reads as "no value". A plain `Q` and
    // the comment marker `/` are the two characters the canonical test
    // uses, so a marker that is also a comment start is covered.
    for marker in ["Q", "/"] {
        let mut parser = make();
        parser.token_with_source("#T/", marker);
        parser.action_with_context("@mixed", |rule, _context| {
            let text = rule.o1().map(|token| match &token.val {
                Value::String(text) => text.clone(),
                other => other.to_string(),
            });
            let value = format!("@{}", text.unwrap_or_default());
            std::rc::Rc::make_mut(&mut rule.o)[0].val = Value::String(value);
            Ok(())
        });
        let spec = GrammarSpec::from_value(serde_json::json!({
            "rule": {
                "val": { "open": [ { "s": "#T/ #TX", "a": "@mixed" } ] },
                "elem": { "close": [ { "s": "#T/ #TX", "r": "elem", "b": 2 } ] }
            }
        }))
        .expect("a valid document");
        parser.grammar(&spec).expect("installs");
        assert_eq!(
            parsed(&parser, &format!("[{marker}x{marker}y]")),
            r#"["@x","@y"]"#,
            "marker {marker:?}"
        );
    }
    // The untouched fixed token still reads as no value.
    assert_eq!(json(&parse("a:,b:").unwrap()), r#"{"a":null,"b":null}"#);
}

// --- The grammar's shape (go/alignment_test.go TestAlignmentGrammarGTags)

/// The group tags of every alternate, per rule and state, in order. The
/// source of truth is the TypeScript grammar after the json core and the
/// jsonic extension phases; the same table is asserted by the Go suite.
const GRAMMAR_TAGS: &[(&str, &[&str], &[&str])] = &[
    (
        "val",
        &[
            "map,json",
            "list,json",
            "pair,jsonic,top",
            "pair,jsonic",
            "val,json",
            "val,imp,null,jsonic",
            "list,imp,jsonic",
            "list,val,imp,null,jsonic",
            "jsonic",
        ],
        &[
            "end,json",
            "val,json,close",
            "list,val,imp,comma,jsonic",
            "list,val,imp,space,jsonic",
            "end,jsonic",
            "more,json",
        ],
    ),
    (
        "map",
        &[
            "end,jsonic",
            "map,json",
            "map,json,pair",
            "pair,list,val,imp,jsonic",
        ],
        &[
            "end,json",
            "path,close,jsonic",
            "end,path,jsonic",
            "end,jsonic",
        ],
    ),
    (
        "list",
        &[
            "",
            "list,json",
            "list,elem,json",
            "list,elem,val,imp,jsonic",
            "list,elem,jsonic",
        ],
        &["end,json", "end,jsonic"],
    ),
    (
        "pair",
        &["map,pair,key,json", "map,pair,comma,jsonic"],
        &[
            "map,pair,close,json",
            "map,pair,comma,jsonic",
            "end,jsonic",
            "map,pair,sync,json",
            "map,pair,sync,jsonic",
            "map,pair,imp,sync,jsonic",
            "map,pair,imp,path,close,jsonic",
            "end,jsonic",
            "map,pair,end,json",
            "map,pair,imp,jsonic",
        ],
    ),
    (
        "elem",
        &[
            "list,elem,imp,null,jsonic",
            "list,elem,imp,null,jsonic",
            "elem,pair,jsonic",
            "list,elem,val,json",
        ],
        &[
            "list,elem,comma,jsonic",
            "list,elem,sync,json",
            "list,elem,close,json",
            "list,elem,end,json",
            "end,jsonic",
            "list,elem,imp,jsonic",
        ],
    ),
];

fn tags(parser: &Tabnas, rule: &str) -> (Vec<String>, Vec<String>) {
    let spec = parser
        .rule_specs()
        .into_iter()
        .find(|spec| spec.name == rule)
        .unwrap_or_else(|| panic!("rule {rule} not found"));
    (
        spec.open.iter().map(|alt| alt.g.clone()).collect(),
        spec.close.iter().map(|alt| alt.g.clone()).collect(),
    )
}

#[test]
fn the_alternates_are_the_typescript_ones_in_the_typescript_order() {
    let parser = make();
    assert_eq!(parser.rule_names(), ["val", "map", "list", "pair", "elem"]);
    for (rule, open, close) in GRAMMAR_TAGS {
        let (got_open, got_close) = tags(&parser, rule);
        assert_eq!(got_open, *open, "{rule}.open");
        assert_eq!(got_close, *close, "{rule}.close");
    }
}

#[test]
fn the_option_conditional_alternates_are_decided_by_the_options() {
    let parser = make_with(|o| {
        o.map.child = true;
        o.list.child = true;
    });
    assert_eq!(
        tags(&parser, "pair").0,
        [
            "map,pair,key,json",
            "map,pair,comma,jsonic",
            "map,pair,child,jsonic"
        ]
    );
    assert_eq!(
        tags(&parser, "elem").0,
        [
            "list,elem,imp,null,jsonic",
            "list,elem,imp,null,jsonic",
            "elem,pair,jsonic",
            "elem,child,jsonic",
            "list,elem,val,json",
        ]
    );
}

#[test]
fn the_relaxed_profile_is_the_engine_default_profile() {
    // jsonic layers on the strict-JSON core, which arrives with its strict
    // option profile; every field that profile changes must be back at
    // the engine's relaxed default once jsonic is installed, and the two
    // must not have drifted apart. Only the error identity and hints
    // differ.
    let bare = Tabnas::new().config();
    let relaxed = make().config();
    assert_eq!(relaxed.text.lex, bare.text.lex);
    assert_eq!(relaxed.number.hex, bare.number.hex);
    assert_eq!(relaxed.number.oct, bare.number.oct);
    assert_eq!(relaxed.number.bin, bare.number.bin);
    assert_eq!(relaxed.number.sep, bare.number.sep);
    assert!(relaxed.number.check.is_none());
    assert_eq!(relaxed.string.chars, bare.string.chars);
    assert_eq!(relaxed.string.multi_chars, bare.string.multi_chars);
    assert_eq!(relaxed.string.allow_unknown, bare.string.allow_unknown);
    assert_eq!(relaxed.string.escape_strict, bare.string.escape_strict);
    assert_eq!(relaxed.string.escape, bare.string.escape);
    assert_eq!(relaxed.comment.lex, bare.comment.lex);
    assert_eq!(
        relaxed.comment.definitions.keys().collect::<Vec<_>>(),
        bare.comment.definitions.keys().collect::<Vec<_>>()
    );
    assert_eq!(relaxed.map.extend, bare.map.extend);
    assert_eq!(relaxed.list.property, bare.list.property);
    assert_eq!(relaxed.lex.empty, bare.lex.empty);
    assert_eq!(relaxed.rule.finish, bare.rule.finish);
    assert_eq!(relaxed.rule.include, bare.rule.include);
    assert_eq!(relaxed.rule.exclude, bare.rule.exclude);
    assert_eq!(relaxed.token_set.get("KEY"), bare.token_set.get("KEY"));
    assert_eq!(relaxed.token_set.get("VAL"), bare.token_set.get("VAL"));
    // json's budget is gone with the rest of its profile, and jsonic's
    // own depth budget (see `nesting_is_bounded_by_the_depth_budget`) is
    // in its place.
    assert!(relaxed.parse.budget.on_check.is_some());
    assert_eq!(relaxed.parse.budget.check_every_n, 1);
    assert_eq!(relaxed.safe.key, bare.safe.key);
    assert_eq!(relaxed.errmsg.name, "jsonic");
    assert_eq!(relaxed.errmsg.link, "https://github.com/tabnas/jsonic");
    assert_eq!(relaxed.error, bare.error);
    assert_ne!(relaxed.hint, bare.hint);
}

// --- Key order (go/ordered_test.go, ts/test/ordered.test.js) -----------

/// KEEP IN SYNC with go/ordered_test.go and ts/test/ordered.test.js.
const ORDERED_CASES: &[(&str, &[&str])] = &[
    ("{2:9, 1:8}", &["2", "1"]),
    ("{10:a, 2:b, x:c}", &["10", "2", "x"]),
    ("{a:1, 2:b, a:3}", &["a", "2"]), // repeated key keeps first position
    ("{zz:1, 0:2, aa:3}", &["zz", "0", "aa"]),
];

fn keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(map) => map.keys().cloned().collect(),
        Value::MapRef(map) => map.value.keys().cloned().collect(),
        other => panic!("not a map: {other:?}"),
    }
}

#[test]
fn key_order_is_source_order() {
    // An IndexMap keeps every key in insertion order by construction, as
    // the Go port's OrderedMap does and TypeScript's `map.ordered` mode
    // records; the same table is asserted in both.
    for (src, order) in ORDERED_CASES {
        assert_eq!(keys(&parse(src).unwrap()), *order, "{src:?}");
    }
    let ordered = make_with(|o| o.map.ordered = true);
    for (src, order) in ORDERED_CASES {
        assert_eq!(keys(&ordered.parse(src).unwrap()), *order, "{src:?}");
    }
}

#[test]
fn a_merged_nested_key_keeps_its_arrival_order() {
    let value = parse("a:b:1,a:c:2").unwrap();
    let Value::Object(map) = &value else {
        panic!("a map")
    };
    assert_eq!(keys(&map["a"]), ["b", "c"]);
    let value = parse("{9:{z:1}, 2:x}").unwrap();
    let Value::Object(map) = &value else {
        panic!("a map")
    };
    assert_eq!(keys(&map["9"]), ["z"]);
}

// --- info.map: MapRef (go/mapref_test.go) ---------------------------------

fn map_ref(value: &Value) -> &tabnas::MapRef {
    match value {
        Value::MapRef(map) => map,
        other => panic!("expected MapRef, got {other:?}"),
    }
}

fn with_map_info(src: &str) -> Value {
    make_with(|o| o.info.map = true)
        .parse(src)
        .unwrap_or_else(|error| panic!("{src:?}: {error}"))
}

#[test]
fn map_info_off_gives_plain_objects() {
    assert!(matches!(parse("{a:1}").unwrap(), Value::Object(_)));
    let parser = make_with(|o| o.info.map = false);
    assert!(matches!(parser.parse("{a:1}").unwrap(), Value::Object(_)));
}

#[test]
fn map_info_marks_explicit_and_implicit_maps() {
    let explicit = with_map_info("{a:1,b:2}");
    assert!(!map_ref(&explicit).implicit);
    assert_eq!(json(&explicit), r#"{"a":1,"b":2}"#);

    assert!(!map_ref(&with_map_info("{}")).implicit);
    assert!(!map_ref(&with_map_info("{a:1}")).implicit);

    for src in ["a:1", "a:1,b:2", "a:1 b:2"] {
        let implicit = with_map_info(src);
        assert!(map_ref(&implicit).implicit, "{src:?}");
    }

    // Nested explicit maps, and an implicit map nested inside an explicit
    // one through a pair dive.
    let nested = with_map_info("{a:{b:1}}");
    assert!(!map_ref(&nested).implicit);
    assert!(!map_ref(&map_ref(&nested).value["a"]).implicit);
    let dive = with_map_info("{a:b:1}");
    assert!(!map_ref(&dive).implicit);
    assert!(map_ref(&map_ref(&dive).value["a"]).implicit);
    assert_eq!(json(&dive), r#"{"a":{"b":1}}"#);
}

#[test]
fn map_info_leaves_lists_and_scalars_alone() {
    assert!(matches!(with_map_info("[1,2]"), Value::Array(_)));
    assert_eq!(with_map_info("42"), Value::Number(42.0));
    assert_eq!(with_map_info("true"), Value::Bool(true));
    assert_eq!(
        json(&with_map_info(r#"{a:"hello",b:"world"}"#)),
        r#"{"a":"hello","b":"world"}"#
    );
}

#[test]
fn map_info_survives_a_deep_merge_and_a_list() {
    let merged = with_map_info("a:{b:1},a:{c:2}");
    assert!(map_ref(&merged).implicit);
    let inner = map_ref(&map_ref(&merged).value["a"]);
    assert!(!inner.implicit);
    assert_eq!(json(&merged), r#"{"a":{"b":1,"c":2}}"#);

    let Value::Array(items) = with_map_info("[{a:1},{b:2}]") else {
        panic!("a list")
    };
    assert_eq!(items.len(), 2);
    assert!(!map_ref(&items[0]).implicit);
    assert!(!map_ref(&items[1]).implicit);
    assert_eq!(map_ref(&items[1]).value["b"], Value::Number(2.0));
}

#[test]
fn map_info_combines_with_list_and_text_info() {
    let parser = make_with(|o| {
        o.info.map = true;
        o.info.list = true;
        o.info.text = true;
    });
    let value = parser.parse(r#"{a:[1,2],b:"x"}"#).unwrap();
    let map = map_ref(&value);
    assert!(!map.implicit);
    let Value::ListRef(list) = &map.value["a"] else {
        panic!("a ListRef")
    };
    assert!(!list.implicit);
    assert_eq!(list.value.len(), 2);
    assert_eq!(
        map.value["b"],
        Value::Text(Text {
            quote: "\"".into(),
            string: "x".into()
        })
    );
    assert_eq!(json(&value), r#"{"a":[1,2],"b":"x"}"#);
}

// --- info.list: ListRef (go/listref_test.go) -----------------------------

fn list_ref(value: &Value) -> &tabnas::ListRef {
    match value {
        Value::ListRef(list) => list,
        other => panic!("expected ListRef, got {other:?}"),
    }
}

fn with_list_info(src: &str) -> Value {
    make_with(|o| o.info.list = true)
        .parse(src)
        .unwrap_or_else(|error| panic!("{src:?}: {error}"))
}

#[test]
fn list_info_off_gives_plain_arrays() {
    assert!(matches!(parse("[1,2]").unwrap(), Value::Array(_)));
    let parser = make_with(|o| o.info.list = false);
    assert!(matches!(parser.parse("[1,2]").unwrap(), Value::Array(_)));
}

#[test]
fn list_info_marks_explicit_and_implicit_lists() {
    for (src, implicit, want) in [
        ("[1,2,3]", false, "[1,2,3]"),
        ("[]", false, "[]"),
        ("[a]", false, r#"["a"]"#),
        ("a,b", true, r#"["a","b"]"#),
        ("a,", true, r#"["a"]"#),
        ("a b c", true, r#"["a","b","c"]"#),
        (",a", true, r#"[null,"a"]"#),
        (",", true, "[null]"),
        ("1,2,3", true, "[1,2,3]"),
        ("1,,", true, "[1,null]"),
        ("1,,,", true, "[1,null,null]"),
    ] {
        let value = with_list_info(src);
        assert_eq!(list_ref(&value).implicit, implicit, "{src:?}");
        assert_eq!(json(&value), want, "{src:?}");
    }
}

#[test]
fn list_info_nests_and_combines() {
    let nested = with_list_info("[[1],[2]]");
    assert!(!list_ref(&nested).implicit);
    for item in &list_ref(&nested).value {
        assert!(!list_ref(item).implicit);
    }

    let mixed = with_list_info("[a],[b]");
    assert!(list_ref(&mixed).implicit);
    for item in &list_ref(&mixed).value {
        assert!(!list_ref(item).implicit);
    }
    let spaced = with_list_info("[a] [b]");
    assert!(list_ref(&spaced).implicit);
    assert_eq!(json(&spaced), r#"[["a"],["b"]]"#);

    let in_map = with_list_info("a:[1,2]");
    let Value::Object(map) = &in_map else {
        panic!("a map")
    };
    assert!(!list_ref(&map["a"]).implicit);

    let maps = with_list_info("{a:1} {b:2}");
    assert!(list_ref(&maps).implicit);
    assert_eq!(json(&maps), r#"[{"a":1},{"b":2}]"#);

    // Extension (deep merge) with ListRef enabled merges the arrays.
    assert_eq!(
        json(&with_list_info("a:[{b:1}],a:[{b:2}]")),
        r#"{"a":[{"b":2}]}"#
    );
}

#[test]
fn list_info_leaves_maps_and_scalars_alone() {
    assert!(matches!(with_list_info("a:1"), Value::Object(_)));
    assert_eq!(with_list_info("42"), Value::Number(42.0));
    assert_eq!(with_list_info("true"), Value::Bool(true));
}

#[test]
fn list_info_combines_with_text_info() {
    let parser = make_with(|o| {
        o.info.list = true;
        o.info.text = true;
    });
    let value = parser.parse(r#"["a",'b',c]"#).unwrap();
    let list = list_ref(&value);
    assert!(!list.implicit);
    let want = [("\"", "a"), ("'", "b"), ("", "c")];
    for (item, (quote, string)) in list.value.iter().zip(want) {
        assert_eq!(
            item,
            &Value::Text(Text {
                quote: quote.into(),
                string: string.into()
            })
        );
    }
}

#[test]
fn list_child_lands_on_the_list_ref_child() {
    // With list.child on, a bare `:value` element becomes the list's
    // child, the wrapper `info.list` produces, and serializes as the plain
    // elements.
    let parser = make_with(|o| o.list.child = true);
    let value = parser.parse("[1,:2,3]").unwrap();
    let list = list_ref(&value);
    assert_eq!(json(&Value::array(list.value.clone())), "[1,3]");
    assert_eq!(list.child.as_deref(), Some(&Value::Number(2.0)));
    assert!(!list.implicit);
    assert_eq!(json(&value), "[1,3]");
    // A list without a child stays a plain array.
    assert!(matches!(parser.parse("[1,2]").unwrap(), Value::Array(_)));
}

// --- info.text: Text (go/textinfo_test.go) ---------------------------------

fn with_text_info(src: &str) -> Value {
    make_with(|o| o.info.text = true)
        .parse(src)
        .unwrap_or_else(|error| panic!("{src:?}: {error}"))
}

fn text(quote: &str, string: &str) -> Value {
    Value::Text(Text {
        quote: quote.into(),
        string: string.into(),
    })
}

#[test]
fn text_info_off_gives_plain_strings() {
    assert_eq!(parse("\"hello\"").unwrap(), Value::String("hello".into()));
    assert_eq!(parse("hello").unwrap(), Value::String("hello".into()));
    let parser = make_with(|o| o.info.text = false);
    assert_eq!(
        parser.parse("\"hello\"").unwrap(),
        Value::String("hello".into())
    );
}

#[test]
fn text_info_keeps_the_quote() {
    for (src, quote, string) in [
        ("\"hello\"", "\"", "hello"),
        ("'hello'", "'", "hello"),
        ("`hello`", "`", "hello"),
        ("hello", "", "hello"),
        ("\"\"", "\"", ""),
        ("''", "'", ""),
        ("``", "`", ""),
        ("\"a\\tb\"", "\"", "a\tb"),
        ("'a\\nb'", "'", "a\nb"),
    ] {
        assert_eq!(with_text_info(src), text(quote, string), "{src:?}");
    }
}

#[test]
fn text_info_wraps_values_and_not_keys() {
    let value = with_text_info(r#"a:"x",b:'y',c:z"#);
    let Value::Object(map) = &value else {
        panic!("a map")
    };
    assert_eq!(map.keys().collect::<Vec<_>>(), ["a", "b", "c"]);
    assert_eq!(map["a"], text("\"", "x"));
    assert_eq!(map["b"], text("'", "y"));
    assert_eq!(map["c"], text("", "z"));
    assert_eq!(json(&value), r#"{"a":"x","b":"y","c":"z"}"#);

    let value = with_text_info(r#""k":"v""#);
    let Value::Object(map) = &value else {
        panic!("a map")
    };
    assert_eq!(map["k"], text("\"", "v"));

    let Value::Array(items) = with_text_info(r#"["hello",1,true,null]"#) else {
        panic!("a list")
    };
    assert_eq!(
        *items,
        vec![
            text("\"", "hello"),
            Value::Number(1.0),
            Value::Bool(true),
            Value::Null
        ]
    );

    let Value::Array(items) = with_text_info("a b c") else {
        panic!("a list")
    };
    assert_eq!(*items, vec![text("", "a"), text("", "b"), text("", "c")]);

    let dive = with_text_info(r#"a:b:"c""#);
    assert_eq!(json(&dive), r#"{"a":{"b":"c"}}"#);
    let Value::Object(outer) = &dive else {
        panic!("a map")
    };
    let Value::Object(inner) = &outer["a"] else {
        panic!("a map")
    };
    assert_eq!(inner["b"], text("\"", "c"));
}

// --- safe.key (go/safe_test.go) --------------------------------------------

#[test]
fn unsafe_keys_are_dropped_from_list_properties_and_kept_on_objects() {
    let parser = make_with(|o| o.list.property = true);
    assert_eq!(parsed(&parser, "[1,2,constructor:fail]"), "[1,2]");
    assert_eq!(parsed(&parser, "[1,2,__proto__:3]"), "[1,2]");
    assert_eq!(parsed(&parser, "[1,2,__proto__:x:1]"), "[1,2]");

    // Objects have no prototype chain, so the key is just a key.
    assert_eq!(
        json(&parse("{constructor:1,a:2}").unwrap()),
        r#"{"constructor":1,"a":2}"#
    );
    assert_eq!(
        json(&parse("{__proto__:1,a:2}").unwrap()),
        r#"{"__proto__":1,"a":2}"#
    );
}

#[test]
fn safe_key_off_still_parses_list_properties() {
    // A JSON array carries no properties, so the pair leaves no trace in
    // the value either way; the guarantee is that the parse succeeds and
    // the elements survive.
    let parser = make_with(|o| {
        o.safe.key = false;
        o.list.property = true;
    });
    assert_eq!(parsed(&parser, "[1,2,__proto__:fail]"), "[1,2]");
    assert_eq!(parsed(&parser, "[1,2,constructor:fail]"), "[1,2]");
    assert_eq!(
        parsed(&parser, "{__proto__:1,a:2}"),
        r#"{"__proto__":1,"a":2}"#
    );
}

// --- Comment suffixes (go/comment_suffix_test.go) -------------------------

fn with_hash_suffixes(suffixes: &[&str]) -> Tabnas {
    let suffixes: Vec<String> = suffixes.iter().map(|s| s.to_string()).collect();
    make_with(move |o| {
        o.comment
            .definitions
            .get_mut("hash")
            .expect("hash def")
            .suffixes = suffixes;
    })
}

#[test]
fn a_line_comment_suffix_terminates_and_is_consumed() {
    assert_eq!(
        parsed(&with_hash_suffixes(&["@@"]), "a:1,# mid @@b:2"),
        r#"{"a":1,"b":2}"#
    );
    // Any of several suffixes terminates.
    assert_eq!(
        parsed(&with_hash_suffixes(&["END", "STOP"]), "a:1,# noise STOPb:2"),
        r#"{"a":1,"b":2}"#
    );
    // The longer of two overlapping suffixes wins and is fully consumed.
    assert_eq!(
        parsed(&with_hash_suffixes(&["@", "@@"]), "a:1,# stop@@b:2"),
        r#"{"a":1,"b":2}"#
    );
    // The suffix is eaten, so a bare `a:` is left with its implicit null.
    assert_eq!(
        parsed(&with_hash_suffixes(&["@@"]), "a:# noise @@"),
        r#"{"a":null}"#
    );
    // Without the marker the line still terminates the comment.
    assert_eq!(
        parsed(&with_hash_suffixes(&["END"]), "a:1\n# no-marker\nb:2"),
        r#"{"a":1,"b":2}"#
    );
}

#[test]
fn a_line_comment_suffix_beats_eatline() {
    // EatLine only runs when termination came from a line char; a suffix
    // terminated comment leaves the newline for the next matcher.
    let parser = make_with(|o| {
        let hash = o.comment.definitions.get_mut("hash").expect("hash def");
        hash.eat_line = true;
        hash.suffixes = vec!["@@".into()];
    });
    assert_eq!(parsed(&parser, "a:1,# note @@\nb:2"), r#"{"a":1,"b":2}"#);
}

#[test]
fn a_block_comment_suffix_terminates_early_and_the_end_marker_still_works() {
    let parser = make_with(|o| {
        o.comment
            .definitions
            .get_mut("multi")
            .expect("multi def")
            .suffixes = vec!["!!".into()];
    });
    // No `*/` at all: `!!` ends the comment and is consumed.
    assert_eq!(parsed(&parser, "a:/* note !!1,b:2"), r#"{"a":1,"b":2}"#);
    // The end marker arriving first wins.
    assert_eq!(parsed(&parser, "a:/* note */1,b:2"), r#"{"a":1,"b":2}"#);
    // Neither: still unterminated.
    assert_eq!(
        parser.parse("a:/* never ends").unwrap_err().code,
        "unterminated_comment"
    );
}

#[test]
fn comment_defs_are_option_space() {
    // Adding a def extends the default markers; removing one removes only
    // that marker; a partial change keeps the rest of the def.
    let added = make_with(|o| {
        o.comment.definitions.insert(
            "semi".into(),
            tabnas::CommentDef {
                line: true,
                start: ";".into(),
                end: String::new(),
                lex: true,
                suffixes: Vec::new(),
                suffix_matcher: None,
                eat_line: false,
            },
        );
    });
    assert_eq!(
        parsed(&added, "a:1 ;c\nb:2 #d\nc:3"),
        r#"{"a":1,"b":2,"c":3}"#
    );

    let removed = make_with(|o| {
        o.comment.definitions.shift_remove("hash");
    });
    assert_eq!(parsed(&removed, "a: #b"), r##"{"a":"#b"}"##);
    assert_eq!(parsed(&removed, "a:1 //c\nb:2"), r#"{"a":1,"b":2}"#);

    let tweaked = make_with(|o| {
        o.comment
            .definitions
            .get_mut("hash")
            .expect("hash def")
            .eat_line = true;
    });
    assert_eq!(parsed(&tweaked, "a:1 #c\nb:2"), r#"{"a":1,"b":2}"#);
}

// --- rule.include / rule.exclude (go/rule_include_test.go) --------------

#[test]
fn exclude_removes_the_relaxed_alternates() {
    let strict = make_with(|o| o.rule.exclude = "jsonic".into());
    assert_eq!(parsed(&strict, r#"{"a":[1,2]}"#), r#"{"a":[1,2]}"#);
    assert!(strict.parse("a:1").is_err());
    assert!(strict.parse("[1,2,]").is_err());

    // The same selector set BEFORE the grammar is honoured too.
    let mut parser = Tabnas::new();
    parser
        .set_options(|o| o.rule.exclude = "jsonic".into())
        .expect("options");
    jsonic(&mut parser).expect("installs");
    assert!(parser.parse("a:1").is_err());
    assert_eq!(parsed(&parser, r#"{"a":1}"#), r#"{"a":1}"#);
}

#[test]
fn include_keeps_only_the_tagged_alternates() {
    let json_only = make_with(|o| o.rule.include = "json".into());
    assert_eq!(
        parsed(&json_only, r#"{"a":1,"b":[2,3]}"#),
        r#"{"a":1,"b":[2,3]}"#
    );
    assert!(json_only.parse("a:1").is_err());

    // Nothing in the grammar carries this tag, so every alternate is
    // filtered and the simplest input fails.
    let none = make_with(|o| o.rule.include = "doesnotexist".into());
    assert!(none.parse(r#"{"a":1}"#).is_err());

    // An empty include is a no-op.
    let all = make_with(|o| o.rule.include = String::new());
    assert_eq!(parsed(&all, "a:1"), r#"{"a":1}"#);

    // Include first, then exclude: an alternate tagged both is removed.
    let both = make_with(|o| {
        o.rule.include = "json".into();
        o.rule.exclude = "map".into();
    });
    assert_eq!(parsed(&both, "[1,2]"), "[1,2]");
    assert!(both.parse(r#"{"a":1}"#).is_err());
}

#[test]
fn a_selector_set_after_construction_is_honoured_at_parse_time() {
    let mut parser = make();
    assert_eq!(parsed(&parser, "a:1"), r#"{"a":1}"#);
    parser
        .set_options(|o| o.rule.exclude = "jsonic".into())
        .expect("options");
    assert!(parser.parse("a:1").is_err());
    parser
        .set_options(|o| o.rule.exclude = String::new())
        .expect("options");
    assert_eq!(parsed(&parser, "a:1"), r#"{"a":1}"#);
}

// --- Map merging (go/alignment_test.go) ----------------------------------

#[test]
fn map_extend_off_replaces_instead_of_merging() {
    let parser = make_with(|o| o.map.extend = false);
    assert_eq!(parsed(&parser, "{a:{b:1},a:{c:2}}"), r#"{"a":{"c":2}}"#);
    assert_eq!(parsed(&parser, "{a:1,a:2}"), r#"{"a":2}"#);
}

#[test]
fn a_custom_merge_function_decides_repeated_keys() {
    // Always keep the previous value.
    let parser =
        make_with(|o| o.map.merge = Some(Arc::new(|previous, _value, _rule, _context| previous)));
    assert_eq!(parsed(&parser, "{a:1,a:2}"), r#"{"a":1}"#);
    assert_eq!(parsed(&parser, "{a:{b:1},a:{c:2}}"), r#"{"a":{"b":1}}"#);
}

#[test]
fn deep_merge_follows_the_canonical_utility() {
    let object = |pairs: &[(&str, Value)]| {
        Value::object(
            pairs
                .iter()
                .map(|(key, value)| (key.to_string(), value.clone()))
                .collect::<IndexMap<_, _>>(),
        )
    };
    let merged = deep_merge(
        object(&[
            ("a", Value::Number(1.0)),
            ("b", object(&[("c", Value::Number(2.0))])),
        ]),
        object(&[
            ("b", object(&[("d", Value::Number(3.0))])),
            ("e", Value::Null),
        ]),
    );
    assert_eq!(json(&merged), r#"{"a":1,"b":{"c":2,"d":3},"e":null}"#);
    // Arrays merge by index.
    assert_eq!(
        json(&deep_merge(
            Value::array(vec![Value::Number(1.0), Value::Number(3.0)]),
            Value::array(vec![Value::Number(2.0)])
        )),
        "[2,3]"
    );
    // A scalar replaces, an undefined overlay leaves the base alone, and
    // the prototype pollution guard skips the dangerous keys.
    assert_eq!(
        deep_merge(Value::Number(1.0), Value::Number(2.0)),
        Value::Number(2.0)
    );
    assert_eq!(
        deep_merge(Value::Number(1.0), Value::Undefined),
        Value::Number(1.0)
    );
    assert_eq!(
        json(&deep_merge(
            object(&[("a", Value::Number(1.0))]),
            object(&[("__proto__", Value::Number(2.0)), ("b", Value::Number(3.0))])
        )),
        r#"{"a":1,"b":3}"#
    );
}

// --- Option-dependent alignment (go/alignment_test.go) --------------------

#[test]
fn list_property_off_and_pair_off_reject_a_pair_in_a_list() {
    let parser = make_with(|o| {
        o.list.property = false;
        o.list.pair = false;
    });
    assert!(parser.parse("[a:1]").is_err());
    // Either option on accepts it again.
    let pairs = make_with(|o| {
        o.list.property = false;
        o.list.pair = true;
    });
    assert_eq!(parsed(&pairs, "[a:1]"), r#"[{"a":1}]"#);
}

#[test]
fn map_child_stores_bare_colon_values() {
    let parser = make_with(|o| o.map.child = true);
    assert_eq!(parsed(&parser, "{:1,a:2}"), r#"{"child$":1,"a":2}"#);
    assert_eq!(
        parsed(&parser, "{:{a:1},:{b:2}}"),
        r#"{"child$":{"a":1,"b":2}}"#
    );
}

#[test]
fn finish_off_names_the_end_of_source() {
    let parser = make_with(|o| o.rule.finish = false);
    for src in ["{a:1", "[1,", "{", "a:{b:"] {
        assert_eq!(
            parser.parse(src).unwrap_err().code,
            "end_of_source",
            "{src:?}"
        );
    }
    assert_eq!(parsed(&parser, "{a:1}"), r#"{"a":1}"#);
}

#[test]
fn nesting_is_bounded_by_the_depth_budget() {
    // 127 containers parse, the 128th is refused with the engine's
    // `cancel` code, whatever the containers are: lists, maps, or the
    // implicit maps of a pair dive. Neither other runtime limits depth
    // (DIVERGENCE.md records this), but the engine's display, JSON
    // conversion and drop of a value walk it with the call stack, and a
    // few thousand levels ended the process with a stack overflow.
    const LIMIT: usize = 127;
    type Nest = fn(usize) -> String;
    let shapes: [(&str, Nest); 3] = [
        ("lists", |n| format!("{}{}", "[".repeat(n), "]".repeat(n))),
        ("maps", |n| format!("{}1{}", "{a:".repeat(n), "}".repeat(n))),
        ("dive", |n| format!("{}1", "a:".repeat(n))),
    ];
    let parser = make();
    for (name, nest) in shapes {
        for depth in [1, 2, 64, LIMIT] {
            let value = parser
                .parse(&nest(depth))
                .unwrap_or_else(|error| panic!("{name} at depth {depth}: {error}"));
            // The value is usable: rendered, converted and dropped.
            assert!(!value.to_string().is_empty());
            assert!(!value.to_json().is_null());
        }
        for depth in [LIMIT + 1, LIMIT + 2, 500, 100_000] {
            let error = parser
                .parse(&nest(depth))
                .err()
                .unwrap_or_else(|| panic!("{name} at depth {depth} must be refused"));
            assert_eq!(error.code, "cancel", "{name} at depth {depth}");
        }
    }
    // An unclosed run is refused the same way, and so is the shared
    // default parser.
    assert_eq!(
        parser.parse(&"[".repeat(100_000)).unwrap_err().code,
        "cancel"
    );
    assert_eq!(parse(&"{a:".repeat(100_000)).unwrap_err().code, "cancel");
    // Width is not depth: ten thousand siblings are fine.
    let wide: Vec<String> = (0..10_000).map(|i| format!("k{i}:[{i}]")).collect();
    assert!(parser.parse(&wide.join(",")).is_ok());
}

#[test]
fn the_depth_budget_reaches_every_constructor_and_yields_to_a_caller_budget() {
    let deep = format!("{}{}", "[".repeat(200), "]".repeat(200));
    assert_eq!(make_json().parse(&deep).unwrap_err().code, "cancel");
    let mut bare = Tabnas::new();
    jsonic(&mut bare).expect("installs");
    assert_eq!(bare.parse(&deep).unwrap_err().code, "cancel");
    let mut used = Tabnas::new();
    used.use_plugin(plugin(), None).expect("installs");
    assert_eq!(used.parse(&deep).unwrap_err().code, "cancel");
    let derived = make().derive(|_| {}).expect("derives");
    assert_eq!(derived.parse(&deep).unwrap_err().code, "cancel");

    // A budget the caller set before the grammar is kept, not replaced.
    let counted = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let seen = counted.clone();
    let parser = make_with(move |o| {
        o.parse.budget.check_every_n = 1;
        o.parse.budget.on_check = Some(Arc::new(move |_context| {
            seen.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            true
        }));
    });
    assert!(parser.parse(&deep).is_ok());
    assert!(counted.load(std::sync::atomic::Ordering::Relaxed) > 0);
}

#[test]
fn the_error_codes_are_the_contract() {
    for (src, code) in [
        ("}", "unexpected"),
        ("]", "unexpected"),
        (":", "unexpected"),
        ("a:1,2", "unexpected"),
        ("\"unterminated", "unterminated_string"),
        ("/*", "unterminated_comment"),
        ("\"x\ty\"", "unprintable"),
    ] {
        let error = parse(src).unwrap_err();
        assert_eq!(error.code, code, "{src:?}");
        assert_eq!(error.row, 1);
    }
    let control = make_with(|o| o.string.allow_control = true);
    assert_eq!(parsed(&control, "\"x\ty\""), r#""x\ty""#);
}

#[test]
fn empty_input_is_the_engine_empty_result() {
    assert_eq!(parse("").unwrap(), Value::Undefined);
    // A source that lexes to nothing but end-of-source is a parse with no
    // value, which the engine reports as null.
    assert_eq!(parse("  # only a comment").unwrap(), Value::Null);
    let parser = make_with(|o| o.lex.empty = false);
    assert!(parser.parse("").is_err());
    let custom = make_with(|o| {
        o.lex.empty = true;
        o.lex.empty_result = Value::Null;
    });
    assert_eq!(custom.parse("").unwrap(), Value::Null);
}

// --- Strict JSON (go/variant_test.go) ------------------------------------

#[test]
fn strict_json_rejects_every_relaxation() {
    let strict = make_json();
    for src in [
        "{a:1}",
        "[1,2,]",
        "{\"a\":1,}",
        "1 // note",
        "'x'",
        "`x`",
        "0xFF",
        "1_000",
        "a:1",
        "[01]",
        "[+1]",
        "[1.]",
        "[.5]",
        "{1:1}",
        "{null:null}",
        "[\"\\x41\"]",
        "[\"\\v\"]",
        "",
        "{\"a\":1",
    ] {
        assert!(strict.parse(src).is_err(), "should have rejected: {src:?}");
    }
    for (src, want) in [
        (
            r#"{"a":[1,2.5,-3e2,true,null,"x"]}"#,
            r#"{"a":[1,2.5,-300,true,null,"x"]}"#,
        ),
        ("[\"\\u0041\"]", r#"["A"]"#),
        ("0", "0"),
        ("\"\"", r#""""#),
    ] {
        assert_eq!(parsed(&strict, src), want, "{src:?}");
    }
    // And the relaxed parser still takes what strict refuses.
    assert_eq!(json(&parse("{a:1}").unwrap()), r#"{"a":1}"#);
}

// --- Shape of the API ----------------------------------------------------

#[test]
fn make_and_parse_agree() {
    let value = make().parse("a:[1,2]").expect("parses");
    let direct = parse("a:[1,2]").expect("parses");
    assert_eq!(json(&value), json(&direct));
}

#[test]
fn an_instance_is_reusable_after_a_failure() {
    let parser = make();
    assert!(parser.parse("1").is_ok());
    assert!(parser.parse("{a:1").is_ok());
    assert!(parser.parse("}").is_err());
    assert_eq!(parsed(&parser, "3"), "3");
}

#[test]
fn empty_keeps_the_configuration_and_drops_the_rules() {
    let bare = empty();
    assert_eq!(bare.rule_names(), ["val", "map", "list", "pair", "elem"]);
    for spec in bare.rule_specs() {
        assert!(
            spec.open.is_empty() && spec.close.is_empty(),
            "{}",
            spec.name
        );
    }
    assert_eq!(bare.config().errmsg.name, "jsonic");
    assert!(bare.config().comment.lex);
}

#[test]
fn the_shared_default_parser_takes_concurrent_callers() {
    // `parse` builds its engine once and hands every caller the same one,
    // as `sync.Once` does in the Go port. Failing parses are interleaved
    // with succeeding ones on purpose: a lexer or rule-stack leak across
    // calls would surface as a wrong value or a spurious error here.
    let threads: Vec<_> = (0..8)
        .map(|n| {
            std::thread::spawn(move || {
                let src = format!("n:{n}, xs:[1,2,3], s:'a b'");
                for _ in 0..50 {
                    let value = parse(&src).expect("parses");
                    let Value::Object(fields) = &value else {
                        panic!("an object, got {value:?}")
                    };
                    assert_eq!(fields["n"], Value::Number(f64::from(n)));
                    assert_eq!(json(&fields["xs"]), "[1,2,3]");
                    assert!(parse("}").is_err());
                }
            })
        })
        .collect();
    for thread in threads {
        thread.join().expect("no thread panicked");
    }
}

#[test]
fn parse_reuses_the_default_instance() {
    // Guards against `parse` rebuilding the grammar on every call:
    // building the grammar dominates a small parse, so a rebuild-per-call
    // `parse` is many times slower than instance reuse. The check is
    // machine-independent: both sides run on the same machine in the same
    // process, with no wall-clock budget. Mirrors go/perf_test.go.
    const SRC: &str = "a:1,b:2,c:3";
    const N: usize = 3000;
    for _ in 0..100 {
        parse(SRC).expect("warm");
    }
    let parser = make();
    for _ in 0..100 {
        parser.parse(SRC).expect("warm");
    }

    let started = Instant::now();
    for _ in 0..N {
        parse(SRC).expect("parses");
    }
    let convenience = started.elapsed();

    let started = Instant::now();
    for _ in 0..N {
        parser.parse(SRC).expect("parses");
    }
    let reuse = started.elapsed();

    assert!(
        convenience <= reuse * 4,
        "parse() appears to rebuild the grammar on every call: {N} calls took {convenience:?} \
         against {reuse:?} reusing one instance"
    );
}
