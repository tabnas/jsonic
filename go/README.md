# jsonic (Go)

Version: 0.1.22

A Go port of [jsonic](https://github.com/tabnas/jsonic), the lenient
JSON parser. Same architecture, same syntax, same results. If you
already use jsonic in TypeScript, you know what this does. If you don't,
read on.

jsonic accepts all standard JSON -- and then goes further. Unquoted
keys, implicit objects, comments, trailing commas, single-quoted
strings, multiline strings, path diving, and more. It parses what you
meant, not just what you typed.

## Install

```bash
go get github.com/tabnas/jsonic/go@latest
```

## Quick Example

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

That's it. No schema, no struct tags, no ceremony.

## jsonic is a tabnas plugin

jsonic is the relaxed-JSON **grammar plugin** for the
[`tabnas`](https://github.com/tabnas/parser) engine
(`github.com/tabnas/parser/go`). The engine ships no grammar; the
standard-JSON core comes from the
[`@tabnas/json`](https://github.com/tabnas/json) plugin
(`github.com/tabnas/json/go`), and jsonic layers its relaxed extensions on
top of that core. Install it on an engine instance the idiomatic way:

```go
import (
    tabnas "github.com/tabnas/parser/go"
    tabnasjsonic "github.com/tabnas/jsonic/go"
)

j := tabnas.Make()
j.Use(tabnasjsonic.Grammar)
out, _ := j.Parse("a:1, b:[x,y,z]")   // map[a:1 b:[x y z]]
```

Because it is a normal plugin, other grammar plugins can depend on it and
layer their own syntax on top of jsonic's value/map/list rules — register
jsonic first:

```go
j.Use(tabnasjsonic.Grammar)   // dependency: provides the cell-value grammar
j.Use(csv)              // builds on what jsonic registered
```

The `tabnasjsonic.Make` / `tabnasjsonic.Parse` helpers shown above are a **legacy
compatibility layer** that installs this same plugin. Reach for them when
porting existing code; reach for `Use(tabnasjsonic.Grammar)` when composing
grammars.

> **Building from source.** Until `tabnas/parser` and `tabnas/json`
> publish tagged Go modules, this module depends on sibling checkouts via
> `replace` directives in `go.mod` (the same development model the
> TypeScript package uses). Clone `https://github.com/tabnas/parser.git`
> and `https://github.com/tabnas/json.git` next to this repo so they
> resolve at `../../parser/go` and `../../json/go`.

## Configured Instance

You don't have to accept the defaults. `Make` gives you a configured
parser instance with whatever behavior you need:

```go
func boolp(b bool) *bool { return &b }

j := tabnasjsonic.Make(tabnasjsonic.Options{
    Number: &tabnasjsonic.NumberOptions{Lex: boolp(false)},
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
