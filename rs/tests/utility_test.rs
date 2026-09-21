// The utility fixtures: `util.deep`, `util.modlist`, `util.str` and
// `util.strinject`, which are not parses.
//
// The utilities live in the engine crate (`tabnas::utility`), which is
// where jsonic's TypeScript gets them too (`@tabnas/parser/utility`); the
// engine runs its own copy of these four files, and jsonic's copies are
// byte-identical to them. They are run again here because they are in
// this repository's `test/spec`, and a fixture only one runtime runs
// proves nothing.

mod common;

use serde_json::Value;
use tabnas::utility::{deep, modlist, str_inject, str_value, ListMods};
use tabnas_support::{load_spec, Row, SpecFile, SpecOptions};

use common::spec_dir;

fn rows(file: &str) -> SpecFile {
    load_spec(spec_dir().join(file), &SpecOptions::default())
        .unwrap_or_else(|error| panic!("{error}"))
}

fn json(row: &Row, name: &str) -> Value {
    serde_json::from_str(row.named(name))
        .unwrap_or_else(|error| panic!("{}: column {name}: {error}", row.location()))
}

#[test]
fn shared_str_fixture() {
    for row in &rows("utility-str.tsv").rows {
        let value = json(row, "input");
        let max_len = if row.named("maxlen").is_empty() {
            44
        } else {
            row.named("maxlen").parse().expect("an integer maxlen")
        };
        assert_eq!(
            str_value(&value, max_len),
            row.named("expected"),
            "{}",
            row.location()
        );
    }
}

#[test]
fn shared_deep_fixture() {
    for row in &rows("utility-deep.tsv").rows {
        // Columns: arg1..arg4, expected. Empty columns mean the argument
        // is not provided.
        let args: Vec<Value> = ["arg1", "arg2", "arg3", "arg4"]
            .iter()
            .map(|name| row.named(name))
            .take_while(|cell| !cell.is_empty())
            .map(|cell| {
                serde_json::from_str(cell)
                    .unwrap_or_else(|error| panic!("{}: {error}", row.location()))
            })
            .collect();
        let expected = json(row, "expected");
        assert_eq!(
            deep(args[0].clone(), args[1..].iter().cloned()),
            expected,
            "{}",
            row.location()
        );
    }
}

#[test]
fn shared_modlist_fixture() {
    for row in &rows("utility-modlist.tsv").rows {
        let list: Vec<Value> = serde_json::from_str(row.named("input"))
            .unwrap_or_else(|error| panic!("{}: {error}", row.location()));
        let raw = row.named("opts");
        let mods = if raw.is_empty() {
            None
        } else {
            let value: Value = serde_json::from_str(raw)
                .unwrap_or_else(|error| panic!("{}: {error}", row.location()));
            let indexes = |name: &str| -> Vec<isize> {
                value[name]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_i64)
                    .map(|index| index as isize)
                    .collect()
            };
            Some(ListMods {
                delete: indexes("delete"),
                move_items: indexes("move"),
                custom: None,
            })
        };
        let expected: Vec<Value> = serde_json::from_str(row.named("expected"))
            .unwrap_or_else(|error| panic!("{}: {error}", row.location()));
        assert_eq!(modlist(list, mods.as_ref()), expected, "{}", row.location());
    }
}

#[test]
fn shared_strinject_fixture() {
    for row in &rows("utility-strinject.tsv").rows {
        let values = (!row.named("values").is_empty()).then(|| json(row, "values"));
        assert_eq!(
            str_inject(row.named("template"), values.as_ref()),
            row.named("expected"),
            "{}",
            row.location()
        );
    }
}
