# Differences from TypeScript

The TypeScript version is the authoritative implementation. The Go version is
a faithful port but has some differences in behavior, missing features, and
Go-specific additions.

## Behavioral Differences

The two runtimes produce identical parse results for the shared conformance
fixtures (`test/spec/*.tsv`, run by both suites).

> **This file is prose, and prose rots.**
> [`test/spec/divergent.tsv`](../../test/spec/divergent.tsv) is the
> authority: it is EXECUTED by both suites, so a divergence that gets fixed
> fails as loudly as one that regresses. This file has been wrong in both
> directions — claiming `2.e3` and `1e999` still diverged after they were
> aligned, and claiming base-prefixed overflow was aligned before it was.
> Where the two disagree, the ledger wins.

### Divergences that DO change a parse result

The sections after this one concern empty input, error codes and
host-language type representation, and none of them changes a successful
parse value. These three do, and are recorded in the ledger with the engine
PR that closes each. They are inherited from `@tabnas/parser`, not
introduced here, and they close on adoption rather than by a change in this
repo.

| input | TypeScript | Go | closes with |
|---|---|---|---|
| `a"b` | `"a\"b"` | `ERROR:unterminated_string` | `parser#128` |
| `{a:1"}` | `{"a":"1\""}` — a **string** | `ERROR:unterminated_string` | `parser#128` |
| `{a:x<U+2028>y}` | `ERROR:unexpected` | `{"a":"x\u2028y"}` | `parser#125` |
| `{a:"p\u00st"}` | `{"a":"p\u0000"}` | `ERROR:invalid_unicode` | `parser#123` |

The last is a **defect in the canonical port**, not a defensible
difference: `parseInt` used as a validator stops at the first non-hex
character and returns what it read, so the `st` is consumed and discarded
and a character that was never in the input is emitted. It is recorded
rather than repaired only because the repair is in the engine.

`{a:1"}` is the one to notice: the divergence changes the parsed value's
TYPE, not merely whether the document parses.

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

Base-prefixed runs with a dot continuation (`0xFF.5`, `0b1.`) and
separator-at-run-edge forms (`+_1`, `1_`, `1.5_`, `1e_2`) are **aligned**:
both ports decline the whole run to lenient text as one string. Previously
the TS scanner claimed the prefix and emitted the trailing fixed token at
the wrong position — `[0xFF.5]` parsed as `[[],"xFF.5"]`, fabricating
elements and destroying characters — and the Go scanner silently swallowed
edge separators, parsing `+_1` as the number `1`. Numeric separators are
legal only between the digits of a run. Pinned cross-port by
`test/spec/alignment-number-prefix-separator.tsv`.

Key insertion order is **representable in both ports**: Go's `*OrderedMap`
always preserves it; TS opts in with `map: { ordered: true }` + `keyOrder`
(plain objects reorder integer-like keys — a JS semantic). The twin tables
in `go/ordered_test.go` and `ts/test/ordered.test.js` pin the parity.


A leading-digit token that is not a valid number is treated as text in **both**
runtimes (this was previously listed as a divergence and is not one): `123abc`
parses to the string `"123abc"` on both sides.

A trailing dot before an exponent — `2.e3`, `0.e1`, `2.e+3`, `2.e-3` — was
previously text in Go and a number in TS. **This is now aligned**: the engine's
`matchNumber` checked whether the character after the dot was trailing text
before it checked for an exponent, so the `e` was misread as text and the whole
token abandoned. Fixed in `github.com/tabnas/parser/go` (`isExponentStart` in
`go/lexer.go`); both runtimes now yield `2000`, `0`, `2000`, `0.002`. Without
exponent digits (`2.e`) the token stays text in both, matching TS's regexp,
whose exponent group requires a digit.

Base-prefixed integers beyond int64 (`0xFFFFFFFFFFFFFFFF`) are **aligned**:
both ports evaluate the exact value and round once to nearest float64, so
`0xFFFFFFFFFFFFFFFF` is 1.8446744073709552e19 in both (Go previously lexed
the run as text). Pinned, with the clamping trap called out, in
`test/spec/alignment-number-prefix-separator.tsv`.

Out-of-range exponents are **also now aligned**. TS coerces with unary `+`,
which saturates instead of failing, and Go's `parseNumericString` used to treat
`strconv.ParseFloat`'s `ErrRange` as a hard failure and drop the token to the
text matcher. It now keeps the saturated value ParseFloat already returns:

| Source | Both runtimes |
|---|---|
| `1e999`, `1e+999`, `1e309`, `2.e999` | `+Inf` / `Infinity` |
| `-1e999` | `-Inf` / `-Infinity` |
| `1e-999` | `0` |
| `-1e-999` | negative zero |
| `1e` (no exponent digits) | text (`"1e"`) |

Both fixes live in `github.com/tabnas/parser/go`, so they reach jsonic only once
the engine is republished; a `GOWORK=off` build (what the Makefile and CI use)
still resolves the last published `@tabnas/parser` until then. Non-range parse
failures still yield NaN and fall through to the text matcher, so `123abc`
behaviour above is unchanged.

The same applies to the strict-JSON key fix below: it is an engine-side change,
so `go test ./...` (workspace on, sibling `parser`) sees it, while
`GOWORK=off go test ./...` still resolves published `parser/go v0.6.1` and
fails the two non-string-key rows in
`test/spec/alignment-strict-json-mode-errors.tsv`. That failure is expected and
clears when `parser/go` publishes; the fixture pins the correct behaviour
rather than the stale one.

Strict-JSON keys are **aligned** and are no longer a difference.
`Jsonic.make('json')` and Go's `MakeJSON` both restrict map keys to quoted
strings via `tokenSet: { KEY: ['#ST', ...] }` / `TokenSet: {"KEY": {"#ST"}}`,
so both runtimes now reject `{1:1}` and `{null:null}`, matching
`encoding/json` and `JSON.parse`. Go previously accepted them (`{"1":1}`,
`{"null":null}`) because the engine ignored `Options.TokenSet` and resolved
`#KEY` in a declarative `GrammarSpec` against package-level builtin sets;
`Options.TokenSet` is now applied by `Make` and `#KEY`/`#VAL` resolve against
the per-instance sets. Pinned in
`test/spec/alignment-strict-json-mode-errors.tsv`, which runs in both
runtimes.

### Token Consumption

When no grammar alternate matches, both implementations raise an immediate
parse error. Token consumption behavior is aligned.

### Comment Definitions (`comment.def`)

`Make`-time comment defs follow the TS option merge (the defaults live in
option space, `jsonicOptions`, exactly as in TS `defaults.ts`):

- adding a def **extends** the default markers (`#`, `//`, `/* */`)
  instead of replacing them;
- a partial def for a default name (`hash` / `slash` / `multi`) inherits
  the fields it leaves unset (start, end, line, lex, eatline) — e.g.
  `{"hash": {EatLine: &t}}` keeps the `#` start;
- a `nil` def removes just that marker (TS `hash: null`);
- a def for a **new** name is inactive unless it sets `Lex` — mirroring TS
  `makeCommentMatcher`'s `lex: !!om.lex`. (The raw Go engine defaults an
  unset `Lex` to true; `jsonic.Make` normalizes to the TS behavior.)

Two engine-level edges are **not** aligned:

- After construction, `SetOptions` merges the def *map* per key but
  replaces each def value wholesale (the engine's `Deep` does not recurse
  into map values), so a post-construction partial def loses the fields it
  leaves unset. Adding or removing whole defs via `SetOptions` is aligned.
- Go bool fields cannot distinguish unset from `false`, so for a default
  name an explicit `Line: false` (TS `line: false`) is honored only when
  the def also sets `End` — the unambiguous block-conversion shape, since
  line comments never use `End`. A bare `{Line: false}` without `End`
  (degenerate in TS too: a block comment with no end marker) reads as
  unset and keeps the default `Line: true`. To express anything else,
  use a def under a new name (with `Lex`) and set the default name to
  `nil`.

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
