# Options Reference

Options are passed to `Jsonic.make()` to create a configured parser instance.
All fields are optional -- unset fields use defaults.

```js
const j = Jsonic.make({
  comment: { lex: false },
  number: { hex: false }
})
```

## `fixed`

Controls recognition of fixed structural tokens (`{`, `}`, `[`, `]`, `:`, `,`).

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable fixed token recognition |
| `token` | object | (built-in) | Map of token name to source character |

## `space`

Controls whitespace handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable space recognition |
| `chars` | string | `" \t"` | Characters treated as space |

## `line`

Controls line ending handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable line recognition |
| `chars` | string | `"\r\n"` | Characters treated as line endings |
| `rowChars` | string | `"\n"` | Characters that increment the row counter |
| `single` | boolean | `false` | Generate a separate token per newline |

## `text`

Controls unquoted text lexing.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable text matching |
| `modify` | function[] | `[]` | Pipeline of value transformers applied after matching |

## `number`

Controls numeric literal parsing.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable number matching |
| `hex` | boolean | `true` | Support `0x` hexadecimal |
| `oct` | boolean | `true` | Support `0o` octal |
| `bin` | boolean | `true` | Support `0b` binary |
| `sep` | string\|null | `"_"` | Separator character (null to disable) |
| `exclude` | RegExp | -- | Pattern to exclude from number matching |

## `comment`

Controls comment handling.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable all comment lexing |
| `def` | object | (see below) | Comment type definitions |

Default comment definitions:

```js
{
  hash:  { line: true, start: '#', lex: true },
  slash: { line: true, start: '//', lex: true },
  multi: { line: false, start: '/*', end: '*/', lex: true }
}
```

`def` entries merge with the defaults: a new name adds a marker alongside
them, a partial entry for a default name inherits the fields it leaves
unset (e.g. `{ hash: { eatline: true } }` keeps the `#` start), and a
`null` (or `false`) entry removes just that marker.

Each definition has:

| Field | Type | Description |
|---|---|---|
| `line` | boolean | `true` for line comments, `false` for block |
| `start` | string | Start marker |
| `end` | string | End marker (block comments only) |
| `lex` | boolean | Enable this definition (a new def is inactive without `lex: true`) |
| `eatline` | boolean | Consume trailing newline after comment |
| `suffix` | string \| string[] \| LexMatcher | Extra terminator(s), consumed as the comment tail |

## `string`

Controls quoted string parsing.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable string matching |
| `chars` | string | `"'\"\`` | Quote characters |
| `multiChars` | string | `` "`" `` | Characters that allow multiline strings |
| `escapeChar` | string | `"\\"` | Escape character |
| `escape` | object | (standard) | Escape sequence mappings |
| `allowUnknown` | boolean | `true` | Allow unknown escape sequences |
| `abandon` | boolean | `false` | On error, let next matcher try |
| `replace` | object | -- | Character replacement map during scanning |

## `map`

Controls object/map behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `extend` | boolean | `true` | Deep-merge duplicate keys |
| `merge` | function | -- | Custom merge function: `(prev, curr) => result` |
| `child` | boolean | `false` | Parse bare colon as `child$` key |
| `ordered` | boolean | `false` | Record key insertion order; read it with `keyOrder(result)` |

With `ordered: true` the result is still a plain object, but the order every
key first appeared in the source is recorded on the side (non-enumerable, so
`Object.keys`, spread and `JSON.stringify` are unaffected) and read back with
the exported `keyOrder` function. A repeated key keeps its first position,
exactly as the Go port's `OrderedMap.Set` does:

```js
const { Jsonic, keyOrder } = require('@tabnas/jsonic')
const j = Jsonic.make({ map: { ordered: true } })
keyOrder(j.parse('{2:9, 1:8}'))        // => ['2', '1']
keyOrder(j.parse('{10:a, 2:b, x:c}'))  // => ['10', '2', 'x']
```

Without the option, plain-object mode **loses integer-like key order**:
JavaScript enumerates integer-like keys in ascending numeric order no matter
the order they were written, so `{2:9, 1:8}` enumerates as `['1', '2']`. That
is a language semantic, not a fixable bug in the plain representation — the
Go port's `*jsonic.OrderedMap` preserves insertion order for every key, and
`ordered: true` is the TS mirror of that information.

## `list`

Controls array/list behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `property` | boolean | `true` | Allow key-value pairs in arrays |
| `pair` | boolean | `false` | Push pairs as object elements |
| `child` | boolean | `false` | Parse bare colon as child value |

## `value`

Controls keyword recognition.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `true` | Enable value matching |
| `def` | object | (see below) | Keyword definitions |

Default value definitions:

```js
{
  true:  { val: true },
  false: { val: false },
  null:  { val: null }
}
```

Add custom keywords:

```js
Jsonic.make({
  value: {
    def: {
      yes: { val: true },
      no:  { val: false }
    }
  }
})
```

## `match`

Controls custom matcher tokens and values.

| Field | Type | Default | Description |
|---|---|---|---|
| `lex` | boolean | `false` | Enable custom matchers |
| `token` | object | -- | Map of token name to RegExp or matcher function |
| `value` | object | -- | Map of value name to `{match, val?}` |

## `rule`

Controls parser rule behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `start` | string | `"val"` | Name of the starting rule |
| `finish` | boolean | `true` | Auto-close unclosed structures at EOF |
| `maxmul` | number | `3` | Rule occurrence multiplier limit |
| `include` | string | -- | Include only rules with these group tags |
| `exclude` | string | -- | Exclude rules with these group tags |

## `lex`

Controls global lexer behavior.

| Field | Type | Default | Description |
|---|---|---|---|
| `empty` | boolean | `true` | Allow empty source input |
| `emptyResult` | any | `undefined` | Value returned for empty input |

## `safe`

Controls security features.

| Field | Type | Default | Description |
|---|---|---|---|
| `key` | boolean | `true` | Block `__proto__` and `constructor` keys |

## `error`

Custom error message templates, keyed by error code.

```js
Jsonic.make({
  error: { unexpected: 'bad character: ' }
})
```

## `hint`

Additional explanatory text per error code, appended to error messages.

## `debug`

Controls debug output.

| Field | Type | Default | Description |
|---|---|---|---|
| `get_console` | function | -- | Returns the console object for logging |
| `maxlen` | number | -- | Max output length for debug strings |
| `print` | object | -- | `{config?, src?}` debug print options |
