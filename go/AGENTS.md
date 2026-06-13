# Agents Guide — jsonic (Go)

A Go port of jsonic, the relaxed-JSON parser: `jsonic.Parse("a:1, b:2")`
just works. Unlike the TypeScript package (which sits on the separate
`tabnas` engine), this is **one self-contained module** —
`github.com/jsonicjs/jsonic/go` — that bundles a port of the engine
*and* the grammar, with no external dependencies.

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

- `TextInfo` → `Text{Quote, Str}`, `ListRef` → `ListRef{Val, Implicit,
  ...}`, `MapRef` → `MapRef{Val, Implicit}` (typed metadata for Go
  callers); tests in `textinfo_test.go`, `listref_test.go`,
  `mapref_test.go`.
- `MakeJSON()` strict-JSON constructor; tests in `variant_test.go`.
- Introspection API (`RSM()`, `Plugins()`, `Decorate()`, `TinName()`, …).

## Layout

- `jsonic.go` — public `Parse`/`ParseMeta` and `JsonicError` (the error
  templates mirror the engine defaults), `Version`.
- `lexer.go` — matchers and `LexConfig` (the resolved option tree).
- `parser.go`, `rule.go` — rule machinery.
- `grammar.go` — the relaxed-JSON grammar; `grammarspec.go` — the
  declarative grammar-spec machinery.
- `options.go` — the `Options` tree, `Make`/`MakeJSON`, and
  `buildConfig` (Options → LexConfig, merging defaults).
- `plugin.go` — `Use`, `Rule`, `Token`, `SetOptions`, match registration.
- `debug.go` — `Describe`: render the resolved grammar/config while
  debugging.
- `utility.go`, `util.go`, `text.go`, `token.go` — `Deep`, `StrInject`,
  text-form option parsing, and small helpers.

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
  "use default"; tests use a `boolp`/`intp` helper.
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
