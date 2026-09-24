# tabnas-jsonic (Rust)

The jsonic relaxed-JSON grammar plugin for the
[`tabnas`](https://github.com/tabnas/parser) parsing engine, crate
`tabnas_jsonic`.

jsonic accepts standard JSON and then relaxes it for humans: unquoted
keys (`a:1`), implicit objects and arrays (`a:1,b:2`, `x,y,z`),
comments, trailing commas, single and backtick quoted strings, multiline
strings, and path diving (`a:b:1` is `{"a":{"b":1}}`). The standard-JSON
core (`val` / `map` / `list` / `pair` / `elem`) comes from the
[`tabnas-json`](https://github.com/tabnas/json) plugin; this crate
installs that core and weaves the relaxed alternates and lifecycle
actions around it.

This is the Rust port of the canonical TypeScript implementation in
[`../ts`](../ts); the TypeScript version is authoritative and this crate
tracks it. The Go port is in [`../go`](../go).

## Use

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let value = tabnas_jsonic::parse("a:1, b:[x,y,z]")?;
    assert_eq!(value.to_string(), r#"{"a":1,"b":["x","y","z"]}"#);
    Ok(())
}
```

Or build an instance and reuse it:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = tabnas_jsonic::make();
    let value = parser.parse("a:b:c:1")?;
    assert_eq!(value.to_string(), r#"{"a":{"b":{"c":1}}}"#);
    Ok(())
}
```

Configure one with a closure over the engine's typed options; the
grammar is installed against the finished options, so an option that
adds or removes an alternate (`map.child`, `list.child`, `list.pair`)
takes effect:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parser = tabnas_jsonic::make_with(|options| {
        options.number.lex = false;
        options.list.pair = true;
    });
    assert_eq!(parser.parse("[a:1,2]")?.to_string(), r#"[{"a":"1"},"2"]"#);
    Ok(())
}
```

The strict variant, `Jsonic.make('json')` in TypeScript, is
`make_json()`: standard JSON only, with the lexer tightened as well as
the grammar filtered.

To layer another grammar on the jsonic core, install the plugin on your
own instance, or through `use_plugin` so a derived instance rebuilds it:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut parser = tabnas::Tabnas::new();
    tabnas_jsonic::jsonic(&mut parser)?; // options and grammar
    assert_eq!(parser.parse("a:1")?.to_string(), r#"{"a":1}"#);

    let mut bare = tabnas::Tabnas::new();
    tabnas_jsonic::register_jsonic_grammar(&mut bare)?; // grammar only
    assert_eq!(bare.parse("x,y")?.to_string(), r#"["x","y"]"#);

    let mut derived = tabnas::Tabnas::new();
    derived.use_plugin(tabnas_jsonic::plugin(), None)?;
    assert_eq!(derived.parse("[1,2,]")?.to_string(), "[1,2]");
    Ok(())
}
```

Parse errors are the engine's `TabnasError`, re-exported as
`JsonicError`, with `code`, `row`, `col` and a report that shows the
offending source with a caret under the `[jsonic/<code>]` tag.

## Install

Neither the engine nor the JSON core is published to a registry, so both
are consumed as **sibling checkouts**, the standard tabnas development
model. Clone `https://github.com/tabnas/parser` and
`https://github.com/tabnas/json` next to this repository and point at
them:

```toml
[dependencies]
tabnas-jsonic = { path = "../jsonic/rs" }
tabnas = { path = "../parser/rs" }
```

Both entries are needed. A crate's dependencies are not passed on to its
dependents, so `tabnas-jsonic` alone does not put `tabnas` in your extern
prelude, and the examples above that name `tabnas::Tabnas` would not
resolve. Only `JsonicError` is re-exported. The test suite additionally
needs `https://github.com/tabnas/support` beside the repository, for the
shared fixture runner.

## Differences from the canonical TypeScript

Every relaxation and every parse result is the TypeScript one; the
shared fixtures in [`../test/spec`](../test/spec) hold all three
runtimes to it. What differs is the shape of the API and a few points
where the host language has no way to say what JavaScript says:

- **Configuration is a closure, not an options object.** `make_with`
  mutates the engine's typed `Options`; the callable `Jsonic` facade and
  its string-form options have no Rust counterpart, and `make_json()`
  stands in for `Jsonic.make('json')`.
- **Metadata is typed.** Under `info.map`, `info.list` and `info.text`
  the result carries `MapRef`, `ListRef` and `Text` wrappers, as the Go
  port does, in place of the non-enumerable marker property. They
  serialize as the plain value.
- **A list's `child$` rides on a `ListRef`.** A JavaScript array can take
  a property; a `Vec` cannot, so with `list.child` on, a list that
  received a bare `:value` becomes a `ListRef` whose `child` holds it.
- **A pair inside a list leaves no trace.** With `list.property`,
  TypeScript stores `[1,2,a:3]` as an array with a property `a`, which
  `JSON.stringify` drops. Here the pair is parsed and discarded, as in
  Go, so the serialized value agrees.
- **Key order is document order.** An `IndexMap` keeps every key where it
  arrived, the behaviour TypeScript reaches only with `map.ordered`. The
  option is accepted and changes nothing.
- **Strictness is a `check` hook.** The strict number grammar is a
  negative lookahead in TypeScript, and the `regex` crate has no
  lookaround, so `make_json` binds the positive pattern plus an
  inversion, the shape the Go port uses.
- **Nesting past 127 levels is rejected** with the error code `cancel`,
  counting lists, maps and the implicit maps of a pair dive alike. The
  engine walks a value with the call stack to display, convert, or drop
  it, and a source a few thousand levels deep ended the process; the
  TypeScript and Go ports have no limit. The number is the one
  `tabnas-json` and `serde_json` use.
- **`empty()` keeps the five rules with no alternates**, as the Go
  `Empty` does, so a parse on it fails with `unexpected` where
  `Jsonic.empty()` in TypeScript, which has no rules at all, returns
  `undefined`.
- **Lone surrogates fold to U+FFFD**, and the regular expression dialect
  is the `regex` crate's. Both come from the engine, and both are
  recorded there.

## Build and test

The engine, the JSON core and the fixture runner are path dependencies
on sibling checkouts, so there is nothing to fetch:

```bash
cargo test --all-targets
```

Or, from the repository root, `make test-rs`. For what CI would say,
including formatting and the lockfile check, run `ci/rust/run.sh`.

The suite runs every shared `../test/spec/*.tsv` fixture, the same files
the TypeScript and Go suites run: the standard-shaped ones through the
shared runner, the token-stream corpus, the utility fixtures, the
divergence register, and a tripwire that fails when a fixture is run by
nobody. Beside them are the in-language tests for what a fixture cannot
express: plugin layering, the typed metadata wrappers, key order, safe
keys, comment suffixes, rule selectors, strict mode, the shared default
parser under threads, and that `parse` reuses its instance.

## License

MIT.
