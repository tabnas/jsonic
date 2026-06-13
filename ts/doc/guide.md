# How-to guide

Short, task-focused recipes. Each is self-contained and assumes you
have jsonic installed (see the [tutorial](tutorial.md) for the basics).
For full field lists and signatures, follow the links into the
[API reference](api.md) and [options reference](options.md).

```js
const { Jsonic, JsonicError } = require('jsonic')
```

## Parse strict JSON only

Pass the `'json'` string shortcut to `make()`. The result rejects every
relaxation (unquoted keys, comments, trailing commas, single/backtick
quotes, hex/octal/binary numbers, empty input):

```js
const json = Jsonic.make('json')

json('{"a":1}')   // { a: 1 }
json('a:1')        // throws JsonicError — unquoted key rejected
```

## Keep numbers as strings

Turn the number matcher off so numeric-looking values stay text:

```js
const j = Jsonic.make({ number: { lex: false } })

j('a:1, b:2')      // { a: '1', b: '2' }
```

To keep numbers but drop one format, set `number.hex`, `number.oct`, or
`number.bin` to `false` instead.

## Disable comments

```js
const j = Jsonic.make({ comment: { lex: false } })

j('a:1')           // { a: 1 }
j('a:1 # x')       // throws — `#` is no longer a comment
```

## Add a keyword

Map a source word to a fixed value with `value.def`:

```js
const j = Jsonic.make({
  value: { def: { yes: { val: true }, no: { val: false } } },
})

j('a: yes, b: no') // { a: true, b: false }
```

## Match a value with a pattern

For values that need a regex rather than an exact word, register one
under `match.value`. The `match` regex must be anchored with `^`; `val`
maps the match array to the parsed value:

```js
const j = Jsonic.make({
  match: {
    lex: true,
    value: {
      date: { match: /^\d{4}-\d{2}-\d{2}/, val: (m) => new Date(m[0]) },
    },
  },
})

j('d: 2024-01-15') // { d: Date('2024-01-15') }
```

See the [`match` option](options.md#match).

## Derive a configured child instance

`instance.make(options)` forks an instance: the child inherits the
parent's configuration and plugins, then merges your overrides on top.
The parent is left unchanged.

```js
const base  = Jsonic.make({ number: { hex: false } })
const child = base.make({ comment: { lex: false } })

child('0xa')       // '0xa'  — hex still off (inherited), comments off too
base('0xa')        // '0xa'  — parent unaffected
```

## Handle parse errors

A failed parse throws a `JsonicError`. Catch it and read its fields:

```js
try {
  Jsonic('"abc')   // unterminated string
} catch (err) {
  if (err instanceof JsonicError) {
    err.code         // 'unterminated_string'
    err.lineNumber   // 1   (1-based row of the offending token)
    err.columnNumber // 1   (1-based column)
    err.message      // formatted, multi-line, with source context
  } else {
    throw err
  }
}
```

Customise messages and hints with the `error` and `hint` options. See
the [error reference](api.md#error-handling).

## Watch lexing and parsing

`instance.sub({ lex, rule })` registers observers that fire as the parse
runs. They watch only — they cannot change the result:

```js
const j = Jsonic.make()
j.sub({
  lex:  (token, rule, ctx) => { /* a token was produced */ },
  rule: (rule, ctx)        => { /* a rule step was processed */ },
})
j('a:1')
```

For step-by-step tracing while developing a grammar, use the debug
plugin instead:

```js
const { Debug } = require('jsonic/debug')
const j = Jsonic.make().use(Debug)
```
