# Writing Plugins

Plugins extend jsonic by modifying the grammar, adding new token types,
registering custom matchers, or subscribing to parse events.

## jsonic is itself a plugin

jsonic is a grammar plugin for the
[`tabnas`](https://github.com/tabnas/parser) engine — the engine ships no
grammar, jsonic supplies the relaxed-JSON one. The idiomatic way to use
it, and to write plugins that build on it, is at the engine level:

```js
const { Tabnas } = require('tabnas')
const { jsonic } = require('jsonic')

const parser = new Tabnas().use(jsonic).use(myPlugin)
```

A `tabnas` plugin is a function `(tabnas, options) => void` that
configures the engine instance it is given. Register dependencies first:
a plugin that builds on jsonic's value/map/list rules must be `use`d
*after* `jsonic`. To install only the grammar rules (without re-applying
jsonic's option defaults), call the exported `registerJsonicGrammar(tabnas)`.

The rest of this guide uses the callable `Jsonic` API and its `.use()`
method, which is a legacy compatibility wrapper around the same engine.
Plugins written against it receive the callable `Jsonic` instance instead
of the bare engine, but the rule/token/option methods shown below are the
same on both.

## Plugin Structure

A plugin is a function that receives a jsonic instance and optional
configuration:

```js
function myPlugin(jsonic, options) {
  // Modify the parser here
}

const j = Jsonic.make()
j.use(myPlugin, { key: 'value' })
```

Plugins are re-applied when a child instance is derived with `make()`.

## Adding Tokens

Register a new fixed token (an exact source string) through the `fixed.token`
option, then look up its Tin with `token(name)`:

```js
function tildePlugin(jsonic) {
  jsonic.options({ fixed: { token: { '#TL': '~' } } })
  const T_TILDE = jsonic.token('#TL')  // the token's Tin number
}
```

Token names conventionally use `#XX` format. Built-in tokens:

| Name | Src | Description |
|---|---|---|
| `#OB` | `{` | Open brace |
| `#CB` | `}` | Close brace |
| `#OS` | `[` | Open square |
| `#CS` | `]` | Close square |
| `#CL` | `:` | Colon |
| `#CA` | `,` | Comma |
| `#NR` | -- | Number |
| `#ST` | -- | String |
| `#TX` | -- | Text |
| `#VL` | -- | Value (keyword) |
| `#SP` | -- | Space |
| `#LN` | -- | Line |
| `#CM` | -- | Comment |
| `#BD` | -- | Bad (error) |
| `#ZZ` | -- | End |

## Modifying Rules

The parser uses named rules, each with an `open` and a `close` phase.
`rs.open(alts)` and `rs.close(alts)` add a list of **alternates**; each
alternate matches a token pattern and fires actions. Pass the alternates you
want to add — they are appended to the rule's existing alternates.

```js
function myPlugin(jsonic) {
  jsonic.options({ fixed: { token: { '#TL': '~' } } })
  const T_TILDE = jsonic.token('#TL')

  jsonic.rule('val', (rs) => {
    rs.open([{
      // Match a tilde token
      s: [T_TILDE],
      // Action: set the node value
      a: (rule) => {
        rule.node = 42
      }
    }])
  })
}
```

### Alternate Spec Fields

| Field | Description |
|---|---|
| `s` | Token pattern to match: a token name (`'#TL'`) or an array of Tin |
| `a` | Action function: `(rule, ctx) => void` |
| `p` | Push a new rule onto the stack by name |
| `r` | Replace current rule with another |
| `b` | Backtrack: number of tokens to put back |
| `g` | Group tag string (e.g., `'json'`, `'jsonic,map'`) |
| `h` | Custom handler: `(alt, rule, ctx) => alt` |
| `e` | Error function: `(rule, ctx) => token` |

### State Actions

Each rule spec has four hook points:

| Hook | When |
|---|---|
| `bo` | Before open -- runs before open alternates are tried |
| `ao` | After open -- runs after an open alternate matches |
| `bc` | Before close -- runs before close alternates are tried |
| `ac` | After close -- runs after a close alternate matches |

Each is a method that registers a hook (hooks accumulate; they do not
replace each other):

```js
jsonic.rule('map', (rs) => {
  rs.ao((rule, ctx) => {
    console.log('opened a map at', rule.node)
  })
})
```

## Custom Matchers

For syntax that doesn't fit the built-in matchers, add a custom lexer matcher
via the `match` option:

```js
const j = Jsonic.make({
  match: {
    lex: true,
    value: {
      date: {
        match: /^\d{4}-\d{2}-\d{2}/,
        val: (res) => new Date(res[0])
      }
    }
  }
})

j('d: 2024-01-15')  // { d: Date('2024-01-15') }
```

## Subscribing to Events

Plugins can observe the parse process without modifying it:

```js
function loggingPlugin(jsonic) {
  jsonic.sub({
    lex: (token, rule, ctx) => {
      console.log('lexed:', token)
    },
    rule: (rule, ctx) => {
      console.log('rule:', rule.name, rule.state)
    }
  })
}
```

## Token Sets

Access groups of tokens for use in alternate patterns:

```js
const ignoreTokens = jsonic.tokenSet('IGNORE') // [#SP, #LN, #CM]
const valueTokens  = jsonic.tokenSet('VAL')    // [#TX, #NR, #ST, #VL]
const keyTokens    = jsonic.tokenSet('KEY')     // [#TX, #NR, #ST, #VL]
```

## Example: CSV Plugin

A simplified CSV plugin that treats commas as separators and newlines as
row boundaries:

```js
function csvPlugin(jsonic, options) {
  const sep = options?.sep ?? ','

  // Remove default comment handling
  jsonic.options({ comment: { lex: false } })

  // Modify grammar to treat each line as a row
  jsonic.rule('val', (rs) => {
    // ... add alternates for row/cell parsing
  })
}
```
