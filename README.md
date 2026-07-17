# jsonic

<!-- tabnas-badges -->
[![npm](https://tabnas.github.io/status/badges/jsonic-npm.svg)](https://www.npmjs.com/package/@tabnas/jsonic)
[![CI](https://github.com/tabnas/jsonic/actions/workflows/ci.yml/badge.svg)](https://github.com/tabnas/jsonic/actions/workflows/ci.yml)
[![go](https://tabnas.github.io/status/badges/jsonic-go.svg)](https://pkg.go.dev/github.com/tabnas/jsonic/go)
[![tabnas standard](https://tabnas.github.io/status/badges/jsonic-standard.svg)](https://tabnas.github.io/status/)
<!-- /tabnas-badges -->

A dynamic JSON parser that isn't strict and can be customized.

```
a:1,foo:bar  →  {"a": 1, "foo": "bar"}
```

jsonic accepts all standard JSON and then relaxes it for humans: you can
skip the quotes, the braces, the commas — and jsonic will still parse
what you meant. Every relaxation below is verified against the shared
conformance fixtures, works identically in TypeScript and Go, and can be
switched off individually if you want less magic.

```js
const { Jsonic } = require('@tabnas/jsonic')

Jsonic('a:1, b:2')       // => { a: 1, b: 2 }
Jsonic('x, y, z')        // => ['x', 'y', 'z']
Jsonic('a:b:c:1')        // => { a: { b: { c: 1 } } }
```

## The relaxations

### Write less punctuation

Quotes on keys, quotes on plain string values, top-level braces, and
commas between elements are all optional. Newlines and spaces separate
just as well:

```
a:1, b:2             →  {"a": 1, "b": 2}        # no top-level braces
a:1 b:2              →  {"a": 1, "b": 2}        # no commas
first-name: Sam      →  {"first-name": "Sam"}   # unquoted key and value
1, 2, 3              →  [1, 2, 3]               # implicit array
[x y z]              →  ["x", "y", "z"]         # unquoted, space-separated
```

Trailing commas are fine, dangling structures close themselves at the end
of input, and an empty value means `null`:

```
{a:1, b:2,}          →  {"a": 1, "b": 2}
[1, 2                →  [1, 2]
{a:{b:1              →  {"a": {"b": 1}}
a:                   →  {"a": null}
```

### Path diving

A chain of colons dives into nested objects, and repeated keys
deep-merge instead of clobbering — handy for config files:

```
a:b:c:1              →  {"a": {"b": {"c": 1}}}
a:b:1, a:c:2         →  {"a": {"b": 1, "c": 2}}
x:{a:1}, x:{b:2}     →  {"x": {"a": 1, "b": 2}}
```

Scalars and arrays replace rather than merge (`a:1,a:2` → `{"a":2}`),
and you can supply your own merge function via the `map.merge` option.

### Comments

All three common styles:

```
a:1   # hash comment
b:2   // slash comment
c:3   /* block
         comment */
```

### More strings

Single quotes and backticks work alongside double quotes, and backtick
strings can span multiple lines with newlines preserved. The usual JSON
escapes apply, plus `\xXX` ASCII escapes:

```
'hello'              →  "hello"
`line one
line two`            →  "line one\nline two"
{'b': `\x42`}        →  {"b": "B"}
```

### More numbers

Hex, octal, binary, scientific notation, and underscore digit
separators:

```
0xFF                 →  255
0o17                 →  15
0b1010               →  10
1e3                  →  1000
1_000_000            →  1000000
```

Anything that looks almost like a number but isn't (`1a`, `1.2.3`)
falls back to being a plain string rather than an error.

## The conveniences

### Call it like a function

`Jsonic` is directly callable, and non-string input passes through
unchanged (so you can feed it values that might already be parsed):

```js
Jsonic('a:1')                  // => { a: 1 }
Jsonic({ already: 'parsed' })  // => { already: 'parsed' }
```

### Configure without global state

`Jsonic.make(options)` returns a fresh, independently configured
instance; `instance.make(options)` forks a child that inherits config
and plugins. The original is never mutated:

```js
const relaxed = Jsonic.make({ number: { sep: ' ' } })
relaxed('a: 1 000 000')     // => { a: 1000000 }

const strict = Jsonic.make('json')   // strict-JSON rules only
strict('{"a":1}')           // => { a: 1 }
strict('a:1,')              // throws: not valid strict JSON
```

### Teach it your vocabulary

Custom keywords and custom value matchers turn domain notation into
real values at parse time:

```js
const j = Jsonic.make({
  value: { def: { yes: { val: true }, no: { val: false } } },
  match: {
    value: { date: { match: /^\d{4}-\d{2}-\d{2}/, val: (res) => new Date(res[0]) } },
  },
})

j('active: yes, backup: no')   // => { active: true, backup: false }
j('start: 2026-07-17')         // => { start: 2026-07-17T00:00:00.000Z (a Date) }
```

### Extend it with plugins

jsonic is itself a grammar plugin on the
[tabnas](https://github.com/tabnas/parser) engine, so the same mechanism
that builds it extends it — the `csv`, `toml`, `yaml`, and `ini`
grammars are all plugins layered on jsonic:

```js
const { Tabnas } = require('@tabnas/parser')
const { jsonic } = require('@tabnas/jsonic')

const parser = new Tabnas().use(jsonic)   // add .use(csv), .use(toml), ...
parser.parse('a:1, b:[x,y,z]')            // => { a: 1, b: ['x','y','z'] }
```

(The callable `Jsonic` façade is the classic API; `new Tabnas().use(jsonic)`
is the idiomatic engine form. Both parse identically.)

### Errors that explain themselves

Parse errors are `JsonicError` instances (a `SyntaxError` subclass) with
`code`, `row`, `col`, and a message that shows the offending source with
a caret:

```
[jsonic/unterminated_string]: unterminated string: "oops
  --> <no-file>:1:3
  1 | a:"oops
        ^^^^^ unterminated string: "oops
```

### Safe by default

Prototype pollution is blocked by default (`safe.key`): parsed maps are
built without a prototype chain, so `__proto__:{x:1}` becomes an
ordinary own property and never touches `Object.prototype`.

## Every relaxation is optional

Each feature above sits behind an option group — `comment.lex`,
`number.hex`, `string.multiChars`, `map.extend`, `rule.finish`, and so
on — so you can dial jsonic anywhere between "strict JSON"
(`Jsonic.make('json')`) and fully relaxed. See the options reference
([TS](ts/doc/options.md), [Go](go/doc/options.md)) for the full list.

## Choose your runtime

| Runtime | Start here |
|---|---|
| **TypeScript / JavaScript** (canonical, `jsonic` on npm) | [`ts/README.md`](ts/README.md) |
| **Go** (`github.com/tabnas/jsonic/go`) | [`go/README.md`](go/README.md) |

Both packages are grammar plugins built on the
[`tabnas`](https://github.com/tabnas/parser) parsing engine, layering
jsonic's relaxed syntax on the standard-JSON core supplied by the
[`@tabnas/json`](https://github.com/tabnas/json) plugin (TypeScript uses
the npm packages, Go uses `github.com/tabnas/parser/go` and
`github.com/tabnas/json/go`). TypeScript is canonical — both runtimes
share the conformance fixtures in [`ts/test/spec/`](ts/test/spec/) and
produce the same parse results:

```go
import jsonic "github.com/tabnas/jsonic/go"

result, err := jsonic.Parse("a:1, b:2")   // map[a:1 b:2]
```

## Documentation

Organized by what you are trying to do:

- **Learning** — tutorials: [TypeScript](ts/doc/tutorial.md), [Go](go/doc/tutorial.md).
- **Tasks** — how-to guides ([TS](ts/doc/guide.md), [Go](go/doc/guide.md))
  and plugin guides ([TS](ts/doc/plugins.md), [Go](go/doc/plugins.md)).
- **Reference** — syntax, API, and options per runtime
  ([TS](ts/doc/syntax.md) / [Go](go/doc/syntax.md), and the api/options
  docs alongside them).
- **Understanding** — concepts ([TS](ts/doc/concepts.md), [Go](go/doc/concepts.md))
  and the Go [differences from TypeScript](go/doc/differences.md).

Working on the codebase? Each directory has an `AGENTS.md` with build,
layout, and contribution notes; start with [`AGENTS.md`](AGENTS.md).

## Grammar diagram

The grammar as a railroad/syntax diagram, generated from the live grammar
with [`@tabnas/railroad`](https://github.com/tabnas/railroad):

![jsonic grammar railroad diagram](ts/doc/grammar.svg)

ASCII version: [`ts/doc/grammar.txt`](ts/doc/grammar.txt).

## License

MIT. Copyright (c) Richard Rodger.
