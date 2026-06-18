# Tutorial — your first jsonic parse (Go)

This walks you from nothing to a working parse, then through one
customization and one error. Follow it in order; each step builds on
the last.

For a recipe-style index of individual tasks, see the
[how-to guide](guide.md). For exhaustive signatures, see the
[API reference](api.md).

## 1. Install

```bash
go get github.com/tabnas/jsonic/go@latest
```

jsonic is a grammar plugin for the `tabnas` engine
(`github.com/tabnas/parser/go`), which it pulls in as a dependency.
(While building from a source checkout before `tabnas/parser` is
published, see the sibling-checkout note in the [README](../README.md).)

## 2. Parse a string

Create `main.go`:

```go
package main

import (
	"fmt"

	"github.com/tabnas/jsonic/go"
)

func main() {
	result, err := tabnasjsonic.Parse("a:1, b:2")
	if err != nil {
		panic(err)
	}
	fmt.Println(result) // map[a:1 b:2]
}
```

Run it with `go run .`. You wrote `a:1, b:2` — no braces, no quotes
around the keys — and got back an object. `tabnasjsonic.Parse` is the
zero-config convenience function; it builds a fresh parser each call.
It still accepts ordinary JSON, so `tabnasjsonic.Parse(`{"a":1}`)` works too.

## 3. Inspect the result

`tabnasjsonic.Parse` returns `any`. For relaxed-JSON input the concrete types
are predictable:

- objects → `map[string]any`
- arrays → `[]any`
- numbers → `float64`
- strings → `string`
- booleans → `bool`
- `null` / empty input → `nil`

So type-assert and read fields directly:

```go
result, _ := tabnasjsonic.Parse("a:1, b:2")
m := result.(map[string]any)
fmt.Println(m["a"]) // 1   (a float64)
```

Numbers come back as `float64`, matching `encoding/json`. The full list
is in the [syntax reference](syntax.md#return-types).

## 4. Make a configured instance

The defaults are not the only option. `tabnasjsonic.Make` returns a
configured parser instance you can reuse across many parses. It takes an
`Options` value whose fields are pointers, so `nil` means "use the
default". Define a tiny helper to take the address of a literal:

```go
func boolp(b bool) *bool { return &b }
```

Now turn number lexing off so numeric-looking values stay strings:

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{
	Number: &tabnasjsonic.NumberOptions{Lex: boolp(false)},
})

result, _ := j.Parse("a:1, b:2")
m := result.(map[string]any)
fmt.Println(m["a"]) // 1   (now a string, not a float64)
```

The same instance parses as many strings as you like. Every option is
documented in the [options reference](options.md).

## 5. Catch an error

When the input is malformed, `Parse` returns an `error` — it never
panics. Inspect the structured detail with `errors.As`:

```go
import (
	"errors"
	"fmt"

	"github.com/tabnas/jsonic/go"
)

_, err := tabnasjsonic.Parse(`"abc`)
var je *tabnasjsonic.JsonicError
if errors.As(err, &je) {
	fmt.Println(je.Code)        // unterminated_string
	fmt.Println(je.Row, je.Col) // 1 1
}
```

`err.Error()` renders a formatted message with a caret pointing at the
source location — useful to show a user. The `*tabnasjsonic.JsonicError`
fields (`Code`, `Row`, `Col`, `Hint`, …) are for your code to branch on.

## Where to go next

- [How-to guide](guide.md) — focused recipes for individual tasks.
- [Syntax reference](syntax.md) — supported syntax and return types.
- [Options reference](options.md) — every configuration field.
- [Concepts](concepts.md) — how the package is built, and the
  guarantees it makes.
- [Differences from TypeScript](differences.md) — if you also use the
  TS version.
