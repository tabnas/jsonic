# jsonic (Go)

Version: 0.1.22

A Go port of [jsonic](https://github.com/jsonicjs/jsonic), the lenient
JSON parser. Same architecture, same syntax, same results. If you
already use jsonic in TypeScript, you know what this does. If you don't,
read on.

jsonic accepts all standard JSON -- and then goes further. Unquoted
keys, implicit objects, comments, trailing commas, single-quoted
strings, multiline strings, path diving, and more. It parses what you
meant, not just what you typed.

## Install

```bash
go get github.com/jsonicjs/jsonic/go@latest
```

## Quick Example

```go
package main

import (
    "fmt"
    "github.com/jsonicjs/jsonic/go"
)

func main() {
    result, err := jsonic.Parse("a:1, b:2")
    if err != nil {
        panic(err)
    }
    fmt.Println(result) // map[a:1 b:2]
}
```

That's it. No schema, no struct tags, no ceremony.

## jsonic is a tabnas plugin

jsonic is the relaxed-JSON **grammar plugin** for the
[`tabnas`](https://github.com/tabnas/parser) engine
(`github.com/tabnas/parser/go`). The engine ships no grammar; jsonic
supplies it. Install it on an engine instance the idiomatic way:

```go
import (
    tabnas "github.com/tabnas/parser/go"
    jsonic "github.com/jsonicjs/jsonic/go"
)

j := tabnas.Make()
j.Use(jsonic.Grammar)
out, _ := j.Parse("a:1, b:[x,y,z]")   // map[a:1 b:[x y z]]
```

Because it is a normal plugin, other grammar plugins can depend on it and
layer their own syntax on top of jsonic's value/map/list rules — register
jsonic first:

```go
j.Use(jsonic.Grammar)   // dependency: provides the cell-value grammar
j.Use(csv)              // builds on what jsonic registered
```

The `jsonic.Make` / `jsonic.Parse` helpers shown above are a **legacy
compatibility layer** that installs this same plugin. Reach for them when
porting existing code; reach for `Use(jsonic.Grammar)` when composing
grammars.

> **Building from source.** Until `tabnas/parser` publishes a tagged Go
> module, this module depends on a sibling checkout via a `replace`
> directive in `go.mod` (the same development model the TypeScript package
> uses). Clone `https://github.com/tabnas/parser.git` next to this repo so
> the engine resolves at `../../parser/go`.

## Configured Instance

You don't have to accept the defaults. `Make` gives you a configured
parser instance with whatever behavior you need:

```go
func boolp(b bool) *bool { return &b }

j := jsonic.Make(jsonic.Options{
    Number: &jsonic.NumberOptions{Lex: boolp(false)},
})

result, err := j.Parse("a:1, b:2")
// {"a": "1", "b": "2"} — numbers are kept as strings
```

Options compose. Turn things off, turn things on. You can always change
it later.

## Syntax

jsonic accepts all standard JSON plus the relaxations listed in the
[syntax reference](doc/syntax.md). Here are the highlights:

- **Unquoted keys**: `a:1` &rarr; `{"a": 1}`
- **Implicit objects**: `a:1,b:2` &rarr; `{"a": 1, "b": 2}`
- **Implicit arrays**: `a,b,c` &rarr; `["a", "b", "c"]`
- **Comments**: `#`, `//`, `/* */`
- **Single/backtick quotes**: `'hello'`, `` `hello` ``
- **Path diving**: `a:b:1` &rarr; `{"a": {"b": 1}}`
- **Trailing commas**: `{a:1,}` &rarr; `{"a": 1}`
- **All number formats**: hex, octal, binary, separators

## Documentation

The docs are organized by what you are trying to do:

- **Learning** — [Tutorial](doc/tutorial.md): from install to your first
  parse, step by step.
- **Tasks** — [How-to guide](doc/guide.md) and [Plugin guide](doc/plugins.md):
  focused recipes.
- **Reference** — [API](doc/api.md), [Options](doc/options.md), and
  [Syntax](doc/syntax.md): complete field, method, and syntax lists.
- **Understanding** — [Concepts](doc/concepts.md) and, if you also use
  the TypeScript version, [Differences from TypeScript](doc/differences.md).

## License

MIT. Copyright (c) Richard Rodger.
