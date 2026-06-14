# Agents Guide — jsonic

## What this project is

jsonic is a **lenient JSON parser**: it accepts standard JSON and then
relaxes it for humans — unquoted keys (`a:1`), implicit objects and
arrays (`a:1,b:2`, `x,y,z`), comments, trailing commas, single- and
backtick-quoted strings, multiline strings, and path diving
(`a:b:1` → `{a:{b:1}}`). Keep that use case in mind for every change;
the shared test fixtures encode exactly this behavior.

The parser is a rule-based parser over a configurable matcher-based
lexer. In both runtimes that engine is the separate `tabnas` package and
jsonic supplies the grammar as a plugin. The standard-JSON grammar core
comes from the separate [`@tabnas/json`](https://github.com/tabnas/json)
plugin; jsonic layers its relaxed extensions on top. So jsonic depends on
two plugins: the `tabnas` engine and `@tabnas/json` (npm packages in TS;
`github.com/tabnas/parser/go` and `github.com/tabnas/json/go` in Go).

## Repository map

| Path | What it is |
|---|---|
| [`ts/`](ts/) | **Canonical** TypeScript/JavaScript implementation — the `jsonic` npm package. Supplies the relaxed-JSON grammar, the BNF converter, the debug plugin, and the CLIs on top of the [`tabnas`](https://github.com/tabnas/parser) engine (a dependency). |
| [`go/`](go/) | Go port — a grammar plugin (`github.com/jsonicjs/jsonic/go`) for the Go `tabnas` engine (`github.com/tabnas/parser/go`), mirroring the TS split. Supplies `jsonic.Grammar` (a `tabnas.Plugin`) and the legacy `jsonic.Make`/`Parse` API. Depends on a sibling `tabnas/parser` checkout via a `replace` directive (the Go analogue of the TS `file:` dependency). |
| [`ts/test/spec/`](ts/test/spec/) | Shared `.tsv` conformance fixtures (`input → expected`, or `ERROR:<code>`). Run by both the TypeScript suite and the Go suite. |

## Authority and alignment rules

1. **TypeScript is canonical.** When TS and Go disagree on parse
   behavior, TS wins; change Go to match, and add or extend a shared
   fixture when the behavior is expressible as `input → output`.
2. The shared fixtures in `ts/test/spec/*.tsv` are the parity contract.
   Both suites run them and both must stay green. The Go suite resolves
   them at `../ts/test/spec` (see `go/jsonic_test.go` `specDir`).
3. **Go-only client features are intentional** and must be kept and
   tested: the `TextInfo`, `ListRef`, and `MapRef` wrappers (typed
   metadata for Go callers) and the introspection API. They have no TS
   equivalent by design.
4. Known, accepted differences (error codes, host-language `nil`/
   `undefined`, type representation) are documented in
   [`go/doc/differences.md`](go/doc/differences.md). Update that file
   whenever you change either side's behavior or feature surface.
5. When you add a TS feature, port it to Go in the same change when
   feasible, or record the gap in `go/doc/differences.md` if not.

## Build / test

The TypeScript build needs the `tabnas` engine resolvable at the
`file:` path in `ts/package.json` (`../../parser/ts`) — i.e. a sibling
checkout of the `tabnas/parser` repo, with its `ts/` package built
(`npm install && npm run build` inside it). `ts/Makefile` has combined
targets that drive both languages.

```bash
# TypeScript (from ts/)
npm install
npm run build        # tsc --build src test
npm test             # node --test, includes the shared fixtures

# Go (from go/)
go build ./... && go vet ./...
go test ./...        # includes the shared ts/test/spec fixtures
```

Tests run against compiled output — always `npm run build` after
editing `ts/src/` or `ts/test/*.ts`.

## Documentation

Each implementation's `doc/` is split by purpose; keep every file to
one job and do not mix them:

- **Learning** — `doc/tutorial.md`: one guided happy path, no options
  dumps.
- **Tasks** — `doc/guide.md` and `doc/plugins.md`: short, self-contained
  how-to recipes.
- **Reference** — `doc/api.md`, `doc/options.md`, `doc/syntax.md`: dry
  and complete, no teaching.
- **Explanation** — `doc/concepts.md`, `go/doc/differences.md`, and the
  `ts/doc/*-feasibility.md` design notes: background and rationale.

The `ts/doc/syntax.md` syntax reference is canonical; `go/doc/syntax.md`
links to it and only adds Go-specific notes. Each `README.md` is an
**orientation hub** — what the package is, install, one tiny example,
and links out to the four doc types. Do not let a README grow into a
manual. Ground every factual claim against the source and the fixtures
before writing it.

Working on the code itself? `ts/` and `go/` each have their own
`AGENTS.md` with build, layout, and contribution notes.
