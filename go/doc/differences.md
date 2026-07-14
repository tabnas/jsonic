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

Error codes are aligned with TypeScript. In particular, a raw control
character (code point below 32) inside a quoted string reports
`unprintable`, positioned at the offending character — including a raw
newline in a non-multiline string — while a string that simply hits end of
source reports `unterminated_string`, exactly as in TS. (Earlier Go
versions reported `unterminated_string` for both; the alignment is
implemented as a jsonic-installed lex matcher, `jsonic$unprintable` in
`options.lex.match`, that pre-scans quoted strings just before the
engine's string matcher.)

One related edge is **not** aligned: TS `string.replace` can map a control
character to a replacement (e.g. `{'\n': 'X'}`), making it *legal* string
body — `j1('"aAc\n"') === 'aBcX'` in TS. The Go engine's string matcher
still rejects the raw control character (`unterminated_string`) even when
it has a replacement mapping. Replacement of printable characters is fully
supported, and the `unprintable` error scan honours replace mappings (a
replaced control char is skipped when locating the first offending one).

### Number + Text

A leading-digit token that is not a valid number is treated as text in **both**
runtimes (this was previously listed as a divergence and is not one): `123abc`
parses to the string `"123abc"` on both sides.

### Token Consumption

When no grammar alternate matches, both implementations raise an immediate
parse error. Token consumption behavior is aligned.

## Missing Features

Custom match matchers (`match.token`, `match.value`) are now fully ported:

- `Options.Match.Token` (`map[string]*regexp.Regexp`) and
  `Options.Match.TokenFn` (`map[string]LexMatcher`) are the two halves of
  the TS `match.token` union (RegExp | LexMatcher).
- `Options.Match.Value` (`map[string]*MatchValueSpec`) covers TS
  `match.value`: `Match` is the regexp, `Val` the submatch → value
  handler, and `Fn` the function-form alternative.
- Go regexps are used as-is (no dialect translation is applied to
  programmatically supplied `*regexp.Regexp` values); anchor them with
  `^` explicitly, as the lexer matches against the forward source.
  (Text-form grammars using `@/.../` regex literals are translated by the
  engine as usual.)

See `custom_test.go` for ported examples.

The following TypeScript features are not yet available in Go:

| Feature | TS Option | Notes |
|---|---|---|
| Token-set overrides reaching the built grammar | `tokenSet` | The Go jsonic grammar resolves `#KEY`/`#VAL` statically when its rules are built, so a custom `tokenSet` (e.g. adding an identifier token to `KEY`) does not change the existing alternates. Workaround: modify the rules directly via `j.Rule(...)`. |
| Alt `h` modifier action suppression | (Rule flags) | The TS `h` modifier can set `rule.ao/bc/ac = false` to suppress state actions; the Go `Rule` has no such flags. |
| Deep-copied option values | (all options) | TS copies options deeply, so mutating an option value (e.g. a `value.def` map) after `make()` has no effect; Go keeps the caller's reference. |

## Go-Specific Features

The `Info` options (Go-only) wrap output values in typed structs that carry
metadata, instead of plain Go values. See the
[options reference](options.md#info).

### `Info.Text`

Wraps string and text values in a `Text` struct that preserves the quote
character used (`""` for unquoted text):

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{Info: &tabnasjsonic.InfoOptions{Text: boolp(true)}})
result, _ := j.Parse(`'hello'`)
// result: tabnasjsonic.Text{Quote: "'", Str: "hello"}
```

### `Info.List`

Wraps arrays in a `ListRef` struct with metadata:

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{Info: &tabnasjsonic.InfoOptions{List: boolp(true)}})
result, _ := j.Parse("a, b, c")
// result: tabnasjsonic.ListRef{Val: []any{"a", "b", "c"}, Implicit: true, Meta: map[string]any{}}
```

### `Info.Map`

Wraps objects in a `MapRef` struct with metadata:

```go
j := tabnasjsonic.Make(tabnasjsonic.Options{Info: &tabnasjsonic.InfoOptions{Map: boolp(true)}})
result, _ := j.Parse("a:1")
// result: tabnasjsonic.MapRef{Val: map[string]any{"a": 1.0}, Implicit: true, Meta: map[string]any{}}
```

## Plugin Differences

| Area | TypeScript | Go |
|---|---|---|
| Plugin signature | `(jsonic, opts?) => void` | `func(j *Jsonic, opts map[string]any) error` |
| Rule definer | Receives `RuleSpec` (+ `Parser`) | Receives `*RuleSpec` + `*Parser` |
| State actions | Can return error tokens | No return value |
| Option namespacing | Plugin options merged by name | No namespacing |
| Custom matchers | Via `match` option or `lex.match` | Same: `Options.Match` (token/value matchers) or `Options.Lex.Match` (ordered raw matchers, keyed by name) |

## Error Handling Differences

| Area | TypeScript | Go |
|---|---|---|
| Parse errors | Thrown as exceptions | Returned as `error` (never panics) |
| Error messages | `{key}` template injection | Template prefix + appended source fragment |
| ANSI colors | On by default | On by default for `Make` instances; the `tabnasjsonic.Parse` convenience is plain. Toggle via the `Color` option |
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
