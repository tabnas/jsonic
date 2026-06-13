# Concepts (Go)

Background reading for the Go package: how it is built, the guarantees
it makes, and the mechanics specific to Go. For steps see the
[tutorial](tutorial.md) and [how-to guide](guide.md); for exact
signatures see the [API reference](api.md). For the behavioral
TypeScript ↔ Go comparison, see [differences](differences.md).

## One self-contained package

The TypeScript version of jsonic is a grammar layered on the separate
`tabnas` parsing engine. The Go port is **not** split that way: this
module bundles a port of the engine *and* the relaxed-JSON grammar into
a single package with no external dependencies. You `go get` one thing
and call `jsonic.Parse`.

That difference is an implementation detail of packaging, not of
behavior — the parse results match the canonical TypeScript ones,
verified by the shared `../ts/test/spec/*.tsv` fixtures that both test
suites run.

## Two stages: lexer, then parser

A parse runs in two cooperating stages.

The **lexer** turns source text into a stream of **tokens**, built from
independent **matchers** — one per token kind (fixed punctuation, space,
line endings, strings, comments, numbers, text, and custom matchers).
At each position the matchers run in a fixed priority order and the
first to produce a token wins. Matchers are configured, not hard-coded:
disabling comments or adding a quote character is an option change and
the lexer rebuilds from the resolved `LexConfig`.

The **parser** consumes tokens according to named **rules** (`val`,
`map`, `list`, `pair`, `elem`). Each rule has an **open** and a
**close** phase, each holding a list of **alternates** (`AltSpec`). An
alternate matches a short token pattern — at most two tokens of
lookahead — and when it matches can run an action that builds the
result node, push a child rule, replace the current rule, or backtrack a
token. There is no backtracking search, only two-token lookahead, which
keeps parsing linear and predictable.

## Predictable Go types

A successful parse returns `any` over a small, predictable set of
concrete types: `map[string]any`, `[]any`, `string`, `float64` (all
numbers — there is no integer type), `bool`, and `nil`. With the `Info`
options enabled, strings, lists, and maps are instead wrapped in the
typed `Text`, `ListRef`, and `MapRef` structs that carry quote and
implicit-bracket metadata. These wrappers have no TypeScript equivalent;
they exist so typed Go client code can recover information the plain
value would lose.

## Errors are returned, not panicked

The Go API delivers every failure as a returned `error` — a
`*JsonicError` carrying `Code`, `Row`, `Col`, `Pos`, `Src`, and `Hint`.
Malformed input never panics. The error codes and message templates
mirror the canonical TypeScript ones; the small set of codes that still
differ is listed in [differences](differences.md#error-codes).

## Instances and derivation

A parser instance bundles a resolved configuration, a token table, the
rule set, and its plugins. `Derive(options)` forks an instance: it
inherits the parent's resolved options, deep-merges your overrides on
top, and re-applies the parent's plugins and subscriptions to the child
— so the child sees the parent's configuration, and the parent is left
untouched. This mirrors TypeScript `make()`.

## Where this sits relative to TypeScript

TypeScript is canonical. When the two disagree on a successful parse,
the TypeScript result wins and the Go port is the thing that changes.
The accepted, documented differences — host-language `nil` vs
`undefined`, a few error codes, and the Go-only `Info` wrappers — are
collected in [differences](differences.md).
