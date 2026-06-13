# How-to guide (Go)

Task-focused recipes. Each is self-contained. For a guided introduction
start with the [tutorial](tutorial.md); for complete field and signature
lists see the [API reference](api.md) and [options reference](options.md).

All recipes assume this import and the pointer helper:

```go
import "github.com/jsonicjs/jsonic/go"

func boolp(b bool) *bool { return &b }
```

## Parse strict JSON

`jsonic.MakeJSON()` returns an instance that rejects every relaxation
(unquoted keys, comments, trailing commas, hex/octal/binary numbers,
single/backtick quotes, empty input):

```go
j := jsonic.MakeJSON()

j.Parse(`{"a":1}`) // ok
j.Parse("a:1")      // *JsonicError — unquoted key rejected
```

Under the hood this filters the grammar to alternates tagged `json`. To
get the same filtering on a custom configuration, pass
`Rule: &jsonic.RuleOptions{Include: "json"}` to `jsonic.Make`.

## Keep numbers as strings

Turn the number matcher off so numeric-looking values lex as text:

```go
j := jsonic.Make(jsonic.Options{
	Number: &jsonic.NumberOptions{Lex: boolp(false)},
})

result, _ := j.Parse("a:1, b:2")
// map[string]any{"a": "1", "b": "2"}
```

To keep numbers but drop a single format, set `Hex`, `Oct`, or `Bin`
to `boolp(false)` instead.

## Handle errors

Every parse failure is a `*jsonic.JsonicError`. Use `errors.As` (or a
type assertion) to read its structured fields:

```go
_, err := jsonic.Parse(`"abc`)
var je *jsonic.JsonicError
if errors.As(err, &je) {
	fmt.Println(je.Code)            // "unterminated_string"
	fmt.Println(je.Row, je.Col, je.Pos)
	fmt.Println(je.Hint)            // human-readable explanation
}
```

`je.Error()` renders the full formatted message (header, source extract
with a caret, hint) for display. To turn off the ANSI colors, build the
instance with `Color: &jsonic.ColorOptions{Active: boolp(false)}`.

## Get quote / implicit metadata

The `Info` options wrap output values in typed structs carrying extra
metadata, instead of plain Go values:

```go
j := jsonic.Make(jsonic.Options{Info: &jsonic.InfoOptions{
	Text: boolp(true), // strings → jsonic.Text{Quote, Str}
	List: boolp(true), // arrays  → jsonic.ListRef{Val, Implicit, ...}
	Map:  boolp(true), // objects → jsonic.MapRef{Val, Implicit, ...}
}})

result, _ := j.Parse("a:'x'")
mr := result.(jsonic.MapRef)
fmt.Println(mr.Implicit)          // true (no braces in source)
tx := mr.Val["a"].(jsonic.Text)
fmt.Println(tx.Quote, tx.Str)     // ' x
```

`Text.Quote` is the quote character (`""` for unquoted text).
`ListRef.Implicit` / `MapRef.Implicit` report whether brackets/braces
were present. See the [options reference](options.md#info).

## Derive a configured child instance

`(*Jsonic).Derive(options)` forks an instance: the child inherits the
parent's configuration, plugins, tokens, and subscriptions, then merges
your overrides on top. The parent is left unchanged.

```go
base  := jsonic.Make(jsonic.Options{Number: &jsonic.NumberOptions{Hex: boolp(false)}})
child := base.Derive(jsonic.Options{Comment: &jsonic.CommentOptions{Lex: boolp(false)}})

child.Parse("0xa")  // "0xa"  — hex still off (inherited), comments off too
base.Parse("0xa")   // "0xa"  — parent unaffected
```

## Add a custom matcher

To recognize syntax beyond the built-in matchers, register one under
`Options.Lex.Match`, keyed by name. The factory is invoked when the
options are applied; the matcher it returns reads from `lex.Cursor()`
and must advance the cursor when it produces a token:

```go
j := jsonic.Make(jsonic.Options{Lex: &jsonic.LexOptions{
	Match: map[string]*jsonic.MatchSpec{
		"at": {
			Order: 1_000_000, // < 2_000_000 runs before all built-ins
			Make: func(_ *jsonic.LexConfig, _ *jsonic.Options) jsonic.LexMatcher {
				return func(lex *jsonic.Lex, rule *jsonic.Rule) *jsonic.Token {
					pnt := lex.Cursor()
					if pnt.SI < len(lex.Src) && lex.Src[pnt.SI] == '@' {
						tkn := lex.Token("#TX", jsonic.TinTX, "AT", "@")
						pnt.SI++
						pnt.CI++
						return tkn
					}
					return nil // pass to the next matcher
				}
			},
		},
	},
}})
```

`Order` controls priority (lower runs first); the built-in priorities
are listed in the [plugin guide](plugins.md#custom-matchers). For
grammar changes (new tokens and rules), write a plugin — see the
[plugin guide](plugins.md).
