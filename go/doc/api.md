# API Reference (Go)

```go
import "github.com/tabnas/jsonic/go"
```

## Parsing

### `Parse(src string) (any, error)`

Parse a string using default settings. Convenience function that creates a
fresh parser for each call.

```go
result, err := tabnasjsonic.Parse("a:1, b:2")
// result: map[string]any{"a": float64(1), "b": float64(2)}
```

### `(*Jsonic) Parse(src string) (any, error)`

Parse using an instance's configuration.

```go
j := tabnasjsonic.Make()
result, err := j.Parse("a:1")
```

### `(*Jsonic) ParseMeta(src string, meta map[string]any) (any, error)`

Parse with metadata accessible in rule actions via `ctx.Meta`.

```go
result, err := j.ParseMeta("a:1", map[string]any{"filename": "config.jsonic"})
```

## Instance Management

### `Make(opts ...Options) *Jsonic`

Create a new parser instance. Unset option fields use defaults.

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{
    Comment: &tabnasjsonic.CommentOptions{Lex: boolp(false)},
    Number:  &tabnasjsonic.NumberOptions{Hex: boolp(false)},
})
```

### `(*Jsonic) Derive(opts ...Options) *Jsonic`

Create a child instance inheriting the parent's configuration, plugins, custom
tokens, and subscriptions. Changes to the child do not affect the parent.

```go
child := j.Derive(tabnasjsonic.Options{
    Comment: &tabnasjsonic.CommentOptions{Lex: boolp(false)},
})
```

### `(*Jsonic) SetOptions(opts Options) *Jsonic`

Deep-merge new options into the instance and rebuild the configuration,
grammar, and plugins. Nil/zero fields in opts do not overwrite existing values,
matching the TypeScript `options()` setter behavior. Returns the instance for
chaining.

### `(*Jsonic) Options() Options`

Returns a copy of the instance's current options.

### `(*Jsonic) Decorate(name string, value any) *Jsonic`

Set a named value on the instance. This is the Go equivalent of the
TypeScript pattern where plugins add properties dynamically
(`tabnasjsonic.foo = () => 'FOO'`). Decorations are inherited by `Derive`.

```go
j.Use(func(j *tabnasjsonic.Jsonic, opts map[string]any) {
    j.Decorate("greet", func(name string) string {
        return "hello " + name
    })
})
```

### `(*Jsonic) Decoration(name string) any`

Returns a named value previously set by `Decorate`, or nil.

```go
fn := j.Decoration("greet").(func(string) string)
fmt.Println(fn("world")) // "hello world"
```

## Grammar

### `(*Jsonic) Rule(name string, definer RuleDefiner) *Jsonic`

Modify or create a grammar rule. The definer callback receives the
`*RuleSpec` and the owning `*Parser`, and can modify the rule's
`Open`/`Close` alternate lists and state actions (`BO`, `BC`, `AO`, `AC`).
The parser is available for inspecting or referencing other rules.

```go
j.Rule("val", func(rs *tabnasjsonic.RuleSpec, p *tabnasjsonic.Parser) {
    rs.Open = append([]*tabnasjsonic.AltSpec{{
        S: [][]tabnasjsonic.Tin{{myToken}},
        A: func(r *tabnasjsonic.Rule, ctx *tabnasjsonic.Context) {
            r.Node = "custom"
        },
    }}, rs.Open...)
})
```

### `(*Jsonic) RSM() map[string]*RuleSpec`

Returns the rule spec map for direct inspection.

### `(*Jsonic) Token(name string, src ...string) Tin`

Register a new token type or look up an existing one. With `src`, registers
a fixed token mapping.

```go
TL := j.Token("#TL", "~")  // register ~ as #TL token
OB := j.Token("#OB", "")   // look up existing #OB token
```

### `(*Jsonic) TokenSet(name string) []Tin`

Get a named token set:
- `"IGNORE"` -- space, line, comment tokens
- `"VAL"` -- text, number, string, value tokens
- `"KEY"` -- text, number, string, value tokens

### `(*Jsonic) TinName(tin Tin) string`

Returns the name for a token identification number.

## Plugins

### `(*Jsonic) Use(plugin Plugin, opts ...map[string]any) error`

Register and execute a plugin. Returns an error if the plugin fails (a
plugin that returns `nil` cannot fail).

```go
if err := j.Use(myPlugin, map[string]any{"key": "value"}); err != nil {
    // plugin failed
}
```

### `Plugin` type

A plugin receives the instance and its options and returns an error
(`nil` on success):

```go
type Plugin func(j *Jsonic, opts map[string]any) error
```

### `(*Jsonic) Plugins() []Plugin`

Returns the list of installed plugins.

## Custom Matchers

Register custom lexer matchers via `options.lex.match`, keyed by name.
This mirrors the TypeScript `tabnasjsonic.options({ lex: { match: ... } })` API.
Matchers are tried in priority order (lower first). Built-in priorities:

| Matcher | Priority |
|---|---|
| fixed | 2,000,000 |
| space | 3,000,000 |
| line | 4,000,000 |
| string | 5,000,000 |
| comment | 6,000,000 |
| number | 7,000,000 |
| text | 8,000,000 |

Use an `Order` below 2,000,000 to run before all built-ins.

```go
j := tabnasjsonic.Make()
j.SetOptions(tabnasjsonic.Options{Lex: &tabnasjsonic.LexOptions{
    Match: map[string]*tabnasjsonic.MatchSpec{
        "date": {Order: 1_000_000, Make: func(_ *tabnasjsonic.LexConfig, _ *tabnasjsonic.Options) tabnasjsonic.LexMatcher {
            return func(lex *tabnasjsonic.Lex, rule *tabnasjsonic.Rule) *tabnasjsonic.Token {
                // ... read from lex.Cursor(), advance on match, return a Token
                return nil
            }
        }},
    },
}})
```

Setting a spec under an existing name replaces it.

### `LexMatcher` and `MakeLexMatcher` types

```go
type LexMatcher     func(lex *Lex, rule *Rule) *Token
type MakeLexMatcher func(cfg *LexConfig, opts *Options) LexMatcher

type MatchSpec struct {
    Order int            // lower runs first
    Make  MakeLexMatcher // factory invoked when options are applied
}
```

The matcher reads the current position via `lex.Cursor()` and must advance
the cursor if it produces a token.

## Events

### `(*Jsonic) Sub(lexSub LexSub, ruleSub RuleSub) *Jsonic`

Subscribe to lex and/or rule events. Pass `nil` for either to skip.

```go
j.Sub(func(tkn *tabnasjsonic.Token, rule *tabnasjsonic.Rule, ctx *tabnasjsonic.Context) {
    fmt.Println("token:", tkn)
}, nil)
```

### Subscriber types

```go
type LexSub func(tkn *Token, rule *Rule, ctx *Context)
type RuleSub func(rule *Rule, ctx *Context)
```

## Configuration

### `(*Jsonic) Config() *LexConfig`

Returns the parser's internal configuration for direct inspection or
modification. Prefer `Token()`, `Rule()`, and `options.lex.match` for most work.

### Filtering the grammar by group tag

Every grammar alternate carries group tags (e.g. `json`, `jsonic`, `map`).
Filter them through the `Rule` options rather than a method:

- `Rule.Include` — keep only alternates tagged with these (comma-separated)
  groups; applied first.
- `Rule.Exclude` — drop alternates tagged with these groups; applied after
  `Include`.

```go
// Keep only JSON-tagged rules (one piece of strict-JSON configuration).
j := tabnasjsonic.Make(tabnasjsonic.Options{Rule: &tabnasjsonic.RuleOptions{Include: "json"}})
```

### `MakeJSON() *Jsonic`

Returns an instance configured to accept strict JSON only. It applies
`Rule.Include: "json"` along with the option set that rejects every jsonic
relaxation (unquoted keys/values, comments, hex/octal/binary numbers,
trailing commas, leading-zero numbers, single-quoted or backtick strings,
and empty input). Mirrors TypeScript `Jsonic.make('json')`.

```go
j := tabnasjsonic.MakeJSON()
j.Parse(`{"a":1}`) // ok
j.Parse("a:1")      // *JsonicError — unquoted key rejected
```

## Error Handling

Parse errors are returned as `*JsonicError`:

```go
type JsonicError struct {
    Code   string // "unexpected", "unterminated_string", "unterminated_comment", "unprintable", ...
    Detail string // Human-readable message
    Pos    int    // 0-based character position
    Row    int    // 1-based line number
    Col    int    // 1-based column number
    Src    string // Source fragment at error
    Hint   string // Additional context (if configured)
}
```

```go
result, err := tabnasjsonic.Parse("{a:")
if err != nil {
    if je, ok := err.(*tabnasjsonic.JsonicError); ok {
        fmt.Println(je.Code, "at line", je.Row)
    }
}
```

## Helper Pattern

Go requires a pointer to pass `*bool` option fields. A common pattern:

```go
func boolp(b bool) *bool { return &b }

tabnasjsonic.Options{
    Comment: &tabnasjsonic.CommentOptions{Lex: boolp(false)},
}
```

## Constants

### `Version`

```go
const Version = "0.1.22"
```

The current version of the jsonic Go module.
