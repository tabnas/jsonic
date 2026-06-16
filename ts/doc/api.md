# API Reference

## Parsing

### `Jsonic(src, meta?, parent_ctx?)`

Parse a string using default settings. `Jsonic` is both a callable function
and a namespace for the API below.

```js
const { Jsonic } = require('@tabnas/jsonic')

Jsonic('a:1')  // => { a: 1 }
```

The optional `meta` parameter passes arbitrary data through to plugins and
rule actions.

### `instance(src, meta?)`

A configured instance is also directly callable.

```js
const { Jsonic } = require('@tabnas/jsonic')
const j = Jsonic.make({ comment: { lex: false } })
j('a:1')  // => { a: 1 }
```

## Instance Management

### `Jsonic.make(options?)`

Create a new parser instance with the given options. Unset options fall back
to defaults. Returns a callable instance with the full API.

```js
const strict = Jsonic.make({
  comment: { lex: false },
  text: { lex: false }
})
```

The `options` parameter can also be a string shortcut:
- `'json'` -- strict JSON parser (only JSON-tagged grammar rules)
- `'jsonic'` -- minimal jsonic parser

### `Jsonic.empty(options?)`

Create an empty parser with no grammar or defaults. Used as a base for
building custom parsers from scratch.

## Configuration

### `instance.options`

Direct access to the current options object.

### `instance.options(changes?)`

When called as a function, deep-merges `changes` into the options and returns
the result. Does not modify the instance in-place (use `make()` for that).

### `instance.config()`

Returns a deep copy of the internal configuration. This is the resolved,
compiled form of the options -- useful for debugging.

## Grammar

### `instance.rule(name?, definer?)`

Access or modify grammar rules.

- `rule()` -- returns the full rule spec map
- `rule(name)` -- returns the rule spec for `name`
- `rule(name, definer)` -- calls `definer(ruleSpec)` to modify the rule

Each rule spec has `open` and `close` alternate lists, plus state actions
(`bo`, `bc`, `ao`, `ac`) for before/after open/close phases.

`rs.open(alts)` and `rs.close(alts)` add alternates; `rs.bo`/`rs.ao`/
`rs.bc`/`rs.ac` each register a state-action hook.

```js
jsonic.rule('val', (rs) => {
  rs.open([{
    s: [myToken],
    a: (rule) => { rule.node = 'custom' }
  }])
})
```

### `instance.token(ref)`

Get or create a token type. `ref` is a string name (e.g., `'#OB'` for open
brace); the call returns the token's Tin number, minting one if the name is
new.

```js
const T_OB = jsonic.token('#OB')   // look up a built-in token's Tin
```

To register a new *fixed* token (an exact source string), use the
`fixed.token` option, then look up its Tin:

```js
jsonic.options({ fixed: { token: { '#TL': '~' } } })
const T_TILDE = jsonic.token('#TL')
```

### `instance.tokenSet(ref)`

Get a named token set. Built-in sets: `'IGNORE'` (space, line, comment),
`'VAL'` (text, number, string, value), `'KEY'`.

### `instance.fixed(ref)`

Get the fixed token mapping (source characters to token types).

## Plugins

### `instance.use(plugin, options?)`

The shortcut for adding plugins. Registers and immediately executes a
plugin; the plugin function receives the jsonic instance and the resolved
options (its `plugin.defaults` deep-merged with `options`).

```js
const j = Jsonic.make().use(myPlugin, { key: 'value' })
```

`use()` **returns the instance, so registrations chain**:

```js
const j = Jsonic.make().use(grammarPlugin).use(myPlugin, { opt: 1 })
```

A plugin may return a replacement instance (for example a wrapper); in
that case `use()` returns whatever the plugin returns, otherwise the same
instance. Plugins are **re-applied when deriving a child instance with
`make()`**, so `.use()` decorations propagate to children.

Plugins may also be supplied at construction time — equivalent to calling
`.use()` for each, in order:

```js
const j = Jsonic.make({ plugins: [grammarPlugin, myPlugin] })
```

The same shortcut works on the underlying engine when composing grammars:
`new Tabnas().use(jsonic).use(myPlugin)` (see [Writing Plugins](plugins.md)).

## Events

### `instance.sub({ lex?, rule? })`

Subscribe to lex and/or rule events.

- `lex(token, rule, ctx)` -- fires after each token is lexed
- `rule(rule, ctx)` -- fires before each rule processing step

```js
jsonic.sub({
  lex: (token, rule, ctx) => {
    console.log('token:', token)
  }
})
```

## Utilities

### `instance.util`

A collection of helper functions for plugin authors:

- `tokenize` -- convert token names to Tin numbers
- `deep` -- deep merge objects
- `clone` -- deep clone
- `regexp` -- safe regex construction
- `srcfmt` -- format source strings for display
- `charset` -- build character sets
- `errmsg`, `strinject` -- error message helpers
- `prop`, `keys`, `values`, `entries`, `omap` -- object utilities
- `trimstk`, `makelog`, `clean`, `str`, `mesc`, `escre` -- misc helpers

## Constants

### `OPEN`, `CLOSE`, `BEFORE`, `AFTER`, `EMPTY`

Step constants used in rule definitions and state actions.

## Error Handling

Parsing errors throw a `JsonicError` with:

| Property | Description |
|---|---|
| `code` | Error code (`'unexpected'`, `'unterminated_string'`, etc.) |
| `detail` | Human-readable message |
| `pos` | 0-based character position |
| `row` | 1-based line number |
| `col` | 1-based column number |
| `src` | Source fragment at the error |

## Exports

The main module exports:

| Export | Description |
|---|---|
| `Jsonic` | Main parser (callable + API) |
| `JsonicError` | Error class |
| `Parser` | Parser class |
| `makeLex`, `makeParser` | Factory functions |
| `makeToken`, `makePoint`, `makeRule`, `makeRuleSpec` | Internal constructors |
| `makeFixedMatcher`, `makeSpaceMatcher`, `makeLineMatcher` | Lexer matchers |
| `makeStringMatcher`, `makeCommentMatcher`, `makeNumberMatcher`, `makeTextMatcher` | Lexer matchers |
| `OPEN`, `CLOSE`, `BEFORE`, `AFTER`, `EMPTY` | Step constants |
| `util` | Utility functions |
| `make` | Alias for `Jsonic.make` |
