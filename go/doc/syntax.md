# Syntax Reference (Go)

The Go version of jsonic supports the same core syntax as the TypeScript
version. See the [TypeScript syntax reference](../../ts/doc/syntax.md) for the
full specification — it is canonical and the Go port produces the same parse
results.

This page notes Go-specific behavior. For a complete list of differences, see
[differences.md](differences.md).

## Return Types

jsonic maps parsed values to Go types:

| JSON Type | Go Type |
|---|---|
| Object | `map[string]any` |
| Array | `[]any` |
| String | `string` |
| Number (integer) | `float64` |
| Number (float) | `float64` |
| Boolean | `bool` |
| Null | `nil` |

### Extended Return Types

With the `Info` options enabled, richer types are returned:

- **`Info.Text`** -- string values become `tabnasjsonic.Text{Quote, Str string}`,
  preserving which quote character was used (`""` for unquoted text).
- **`Info.List`** -- arrays become `tabnasjsonic.ListRef{Val []any, Implicit bool, Child any, Meta map[string]any}`,
  where `Implicit` reports whether the array had brackets.
- **`Info.Map`** -- objects become `tabnasjsonic.MapRef{Val map[string]any, Implicit bool, Meta map[string]any}`,
  where `Implicit` reports whether the object had braces.

See the [options reference](options.md#info) for how to enable them.

## Number Handling

All numbers are returned as `float64`, matching `encoding/json` conventions.
There is no separate integer type; `1` and `1.0` both parse to `float64(1)`.

A leading-digit token that is not a valid number lexes as text in both
runtimes: `123abc` parses to the string `"123abc"`, and `a:123abc` to
`{"a":"123abc"}`. The Go and TypeScript parse results are identical here.
