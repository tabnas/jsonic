# Differences from TypeScript

The TypeScript version is the authoritative implementation. The Go version is
a faithful port but has some differences in behavior, missing features, and
Go-specific additions.

## Behavioral Differences

The two runtimes produce identical parse results for the shared conformance
fixtures (`ts/test/spec/*.tsv`, run by both suites). The differences below do
**not** change a successful parse value; they concern empty input, error
codes, and host-language type representation.

### Empty / Whitespace Input

Both implementations short-circuit exact empty-string input (`""`). The one
observable difference is the host language's "no value": TypeScript returns
`undefined`, Go returns `nil`. Whitespace- or comment-only input is processed
through the normal parse flow in both and resolves to `undefined`/`nil` by
grammar behavior.

### Error Codes

A successful parse is identical across runtimes, but a few failing inputs map
to different error *codes*:

| Input | TypeScript code | Go code |
|---|---|---|
| Control char in a double-quoted string (e.g. a raw newline) | `unprintable` | `unterminated_string` |
| Unterminated triple-quote start (`'''...`) | `unprintable` | `unterminated_string` |

The `Code` field is the only thing that differs; both report the failure at the
same row/column. If you branch on `Code`, account for these.

### Number + Text

A leading-digit token that is not a valid number is treated as text in **both**
runtimes (this was previously listed as a divergence and is not one): `123abc`
parses to the string `"123abc"` on both sides.

### Token Consumption

When no grammar alternate matches, both implementations raise an immediate
parse error. Token consumption behavior is aligned.

## Missing Features

The following TypeScript features are not yet available in Go:

| Feature | TS Option | Notes |
|---|---|---|
| Custom match matchers | `match.token`, `match.value` | Use `options.lex.match` instead |

## Go-Specific Features

The `Info` options (Go-only) wrap output values in typed structs that carry
metadata, instead of plain Go values. See the
[options reference](options.md#info).

### `Info.Text`

Wraps string and text values in a `Text` struct that preserves the quote
character used (`""` for unquoted text):

```go
j := jsonic.Make(jsonic.Options{Info: &jsonic.InfoOptions{Text: boolp(true)}})
result, _ := j.Parse(`'hello'`)
// result: jsonic.Text{Quote: "'", Str: "hello"}
```

### `Info.List`

Wraps arrays in a `ListRef` struct with metadata:

```go
j := jsonic.Make(jsonic.Options{Info: &jsonic.InfoOptions{List: boolp(true)}})
result, _ := j.Parse("a, b, c")
// result: jsonic.ListRef{Val: []any{"a", "b", "c"}, Implicit: true, Meta: map[string]any{}}
```

### `Info.Map`

Wraps objects in a `MapRef` struct with metadata:

```go
j := jsonic.Make(jsonic.Options{Info: &jsonic.InfoOptions{Map: boolp(true)}})
result, _ := j.Parse("a:1")
// result: jsonic.MapRef{Val: map[string]any{"a": 1.0}, Implicit: true, Meta: map[string]any{}}
```

## Plugin Differences

| Area | TypeScript | Go |
|---|---|---|
| Plugin signature | `(jsonic, opts?) => void` | `func(j *Jsonic, opts map[string]any) error` |
| Rule definer | Receives `RuleSpec` (+ `Parser`) | Receives `*RuleSpec` + `*Parser` |
| State actions | Can return error tokens | No return value |
| Option namespacing | Plugin options merged by name | No namespacing |
| Custom matchers | Via `match` option | Via `options.lex.match` (keyed by name, same shape) |

## Error Handling Differences

| Area | TypeScript | Go |
|---|---|---|
| Parse errors | Thrown as exceptions | Returned as `error` (never panics) |
| Error messages | `{key}` template injection | Template prefix + appended source fragment |
| ANSI colors | On by default | On by default for `Make` instances; the `jsonic.Parse` convenience is plain. Toggle via the `Color` option |
| Error hints | Rich suffix with source context | `Hint` string field |

## Type System

TypeScript returns untyped `any`. Go returns `any` but the concrete types are
predictable:

| Value | Go Type |
|---|---|
| Objects | `map[string]any` (or `MapRef` with option) |
| Arrays | `[]any` (or `ListRef` with option) |
| Strings | `string` (or `Text` with option) |
| Numbers | `float64` |
| Booleans | `bool` |
| Null | `nil` |
