# Tutorial — your first jsonic parse

This walks you from nothing to a working parse, then through one
customization and one error. Follow it in order; each step builds on
the last. When you finish you will have parsed a string, configured a
parser instance, and handled a parse error.

For a recipe-style index of individual tasks, see the
[how-to guide](guide.md). For exhaustive signatures, see the
[API reference](api.md).

## 1. Install

```bash
npm install jsonic
```

## 2. Parse a string

`Jsonic` is a function. Call it with a string:

```js
const { Jsonic } = require('jsonic')

Jsonic('a:1, b:2')   // { a: 1, b: 2 }
```

You wrote `a:1, b:2` — no braces, no quotes around the key — and got
back an object. That is the point: jsonic parses what you meant. It
still accepts ordinary JSON, so `Jsonic('{"a":1}')` works too.

In TypeScript the import is the same:

```ts
import { Jsonic } from 'jsonic'

Jsonic('x, y, z')    // ['x', 'y', 'z']
```

Comma-, newline-, or space-separated values with no key become an
array; key-value pairs become an object.

## 3. Make a configured instance

`Jsonic` uses sensible defaults, but you do not have to accept them.
`Jsonic.make(options)` returns a *new* parser instance — itself
callable — with the behavior you choose:

```js
const noNumbers = Jsonic.make({
  number: { lex: false },   // do not interpret numeric literals
})

noNumbers('a:1, b:2')       // { a: '1', b: '2' }   (strings, not numbers)
```

The instance is reusable; call it as many times as you like. Options
compose — turn things off, turn things on — and the original `Jsonic`
is unaffected.

## 4. Add a keyword

Suppose you want `yes` and `no` to parse as booleans. Define them with
the `value.def` option:

```js
const j = Jsonic.make({
  value: { def: { yes: { val: true }, no: { val: false } } },
})

j('active: yes')            // { active: true }
```

The built-in keywords are `true`, `false`, and `null`; you have just
added two more.

## 5. Catch an error

When the input cannot be parsed, jsonic throws a `JsonicError`. Catch
it and read its fields:

```js
const { Jsonic, JsonicError } = require('jsonic')

try {
  Jsonic('"abc')             // a string that is never closed
} catch (err) {
  if (err instanceof JsonicError) {
    err.code                 // 'unterminated_string'
    err.lineNumber           // 1
    err.columnNumber         // 1
    err.message              // formatted, with a caret under the source
  }
}
```

`err.message` is a multi-line, human-readable report with a source
extract and a hint — useful to show a user. The structured fields
(`code`, `lineNumber`, `columnNumber`) are for your code to branch on.

## Where to go next

- [How-to guide](guide.md) — focused recipes for individual tasks.
- [Syntax reference](syntax.md) — everything jsonic accepts.
- [Options reference](options.md) — every configuration field.
- [Writing plugins](plugins.md) — extend the grammar itself.
- [Concepts](concepts.md) — how the parser is put together, and why.
