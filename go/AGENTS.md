# Agents Guide — jsonic (Go)

A Go port of jsonic, the relaxed-JSON parser: `jsonic.Parse("a:1, b:2")`
just works. Like the TypeScript package, this is a **grammar plugin for
the `tabnas` engine** (`github.com/tabnas/parser/go`), not a standalone
parser. The engine ships no grammar; this module supplies the
relaxed-JSON one (`jsonic.Grammar`, a `tabnas.Plugin`) plus a legacy
`jsonic.Make`/`jsonic.Parse` API on top of it.

**Dependency / build.** `go.mod` requires `github.com/tabnas/parser/go`
with a `replace` directive pointing at a sibling checkout
(`../../parser/go`) — the same development model the TS package uses for
`tabnas` (`file:../../parser/ts`). Clone
`https://github.com/tabnas/parser.git` next to this repo before building.
There is no `go.sum` entry for the engine while the dependency is a local
`replace`.

## Authority

The TypeScript implementation (`../ts/`) is canonical for parse
behavior. When porting or fixing, read the TS/engine source first and
mirror it:

- option, error, and hint defaults: `../ts/src/defaults.ts` (and the
  `tabnas` engine `defaults.ts`);
- error formatting: the engine `error.ts`;
- matcher semantics and the grammar: `../ts/src/grammar.ts`.

Accepted differences are documented in `doc/differences.md` — keep that
file current with every behavior change.

## Parity contract

Both runtimes run the shared fixtures in `../ts/test/spec/*.tsv`. The Go
suite resolves them via `specDir()` in `jsonic_test.go` (`../ts/test/
spec`) and must keep them all green. A successful parse must produce the
same value as TypeScript; only the documented differences (host-language
`nil` vs `undefined`, a few error *codes*, Go type representation) are
allowed.

## Go-only client features (keep, and keep tested)

- The typed metadata wrappers `Text{Quote, Str}`, `ListRef{Val,
  Implicit, ...}`, `MapRef{Val, Implicit}` now live in the engine and are
  re-exported here; tests in `textinfo_test.go`, `listref_test.go`,
  `mapref_test.go`.
- `MakeJSON()` strict-JSON constructor; tests in `variant_test.go`.
- Introspection API (`RSM()`, `Plugins()`, `Decorate()`, `TinName()`, …)
  — promoted from the engine.

## Layout

The engine (lexer, parser, rule machinery, options, error formatting,
utilities) lives in `github.com/tabnas/parser/go`. This module is just
three files plus tests:

- `jsonic.go` — the legacy API and the plugin: `Make`/`Empty`/`MakeJSON`/
  `Parse`, the idiomatic `Grammar` plugin (branding + grammar) and the
  internal `grammarPlugin` (grammar only), `jsonicOptions` (the jsonic
  error/identity branding), and `Version`.
- `grammar.go` — the relaxed-JSON grammar: `buildGrammar` populates the
  rule-spec map using the engine's exported `ResolveGrammarAltStatic`, and
  the `node*` helpers operate on plain/`MapRef`/`ListRef` nodes.
- `engine.go` — re-exports the engine's public surface (types, funcs,
  consts, vars) under the historic jsonic names; `Jsonic = tabnas.Tabnas`,
  `JsonicError = tabnas.TabnasError`.
- `*_test.go` — behavior and API tests, including `helpers_test.go`
  (test-only helpers: `boolPtr`/`intPtr`, `preprocessEscapes`,
  `splitGroupTags`) and the external `tabnas_plugin_test.go`
  (`package jsonic_test`) that pins the `tabnas.Make().Use(jsonic.Grammar)`
  contract.

Engine internals (the lexer cursor, `normalizeCommentSuffix`,
`resolveToken*Static`, …) are tested in the engine repo, not here.

## Commands

```bash
go build ./... && go vet ./...
go test ./...                  # includes the shared ../ts/test/spec fixtures
go test -coverpkg=./... -cover ./...
go test -run TestName -v ./...
```

## Testing conventions

- Shared behavior: add a fixture under `../ts/test/spec/` and run it via
  the TSV helpers (`alignment_test.go`, `feature_tsv_test.go`,
  `jsonic_test.go`). When you add one, the TS suite must also exercise it.
- Go-specific API: plain `_test.go` files; mirror the TS test name in a
  comment when porting a TS test. `options_parity_test.go` tracks the
  option surface against TS.
- Pointer option fields: most options are pointer types so `nil` means
  "use default"; tests use the `boolPtr`/`intPtr` helpers in
  `helpers_test.go`.
- Errors are returned, never panicked; `JsonicError` carries `Code`,
  `Row`, `Col`, `Pos`, `Src`, `Hint`. Branch on `Code` — note the
  documented code differences from TS in `doc/differences.md`.

## Documentation

`README.md` is an orientation hub: what the module is, `go get`, one
taste example, and links out. The `doc/` files are organized by a single
purpose each — keep them unmixed:

- `doc/tutorial.md` — learning-oriented, one sequential happy path.
- `doc/guide.md`, `doc/plugins.md` — task-oriented how-to recipes.
- `doc/api.md`, `doc/options.md`, `doc/syntax.md` — reference (dry,
  complete; do not teach). `syntax.md` links to the canonical TS spec.
- `doc/concepts.md`, `doc/differences.md` — explanation and the TS↔Go
  comparison.

Verify every signature against the source and keep examples compiling.
The `Plugin` type and `Use` both return `error`; option filtering is via
`Rule.Include`/`Rule.Exclude` (there is no `Exclude` method).
