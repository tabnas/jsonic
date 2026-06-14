# Agents Guide — jsonic (TypeScript)

This is the **canonical** implementation — the `jsonic` npm package.
jsonic is the relaxed-JSON parser: standard JSON plus unquoted keys,
implicit objects/arrays, comments, trailing commas, single/backtick
quotes, multiline strings, and path diving.

The lexer, parser, rule machinery, errors, and utilities are **not**
here — they live in the [`tabnas`](https://github.com/tabnas/parser)
engine package, a dependency. The standard-JSON grammar core
(`val`/`map`/`list`/`pair`/`elem`) is **also** not here: it comes from the
[`@tabnas/json`](https://github.com/tabnas/json) plugin (a dependency)
via its `registerJsonGrammar`. This package supplies jsonic's *relaxed*
extensions on top of that core, and the historic `Jsonic` API: a callable
parse function with the management methods attached as properties.

## Layout

- `src/jsonic.ts` — entry point and API. Constructs `tabnas` engine
  instances and dresses them in the callable-with-properties `Jsonic`
  shape (the legacy compatibility interface). Re-exports the engine
  types/constructors for plugin authors, plus the engine class (`Tabnas`)
  and the idiomatic grammar plugin (`jsonic`) / grammar-only helper
  (`registerJsonicGrammar`).
- `src/grammar.ts` — installs the standard-JSON core via
  `registerJsonGrammar` from `@tabnas/json`, then layers jsonic's relaxed
  extensions on the `val`/`map`/`list`/`pair`/`elem` rules. It overrides
  the `val` close action and the `pair` key alt with jsonic's fuller
  versions (the `@tabnas/json` ones are strict-only: they overwrite
  plugin-set value nodes and decode number/keyword keys to the wrong
  type), using the engine's plugin-override API: the `@val-bc/replace`
  funcref (takes ownership of the close phase so the strict action is not
  re-installed on later `fnref()`/`make()`/derive) and the `clear` alt-mod
  on the `pair` open list. Also provides the strict-JSON variant selected
  by `Jsonic.make('json')` and exports the idiomatic `tabnas` plugin
  `jsonic` (apply jsonic option defaults + register grammar) and the
  `registerJsonicGrammar` helper; the legacy `make()` path installs the
  same grammar. The package is **a normal `tabnas` grammar plugin**
  (`new Tabnas().use(jsonic)`); the callable `Jsonic` API is kept only
  for backward compatibility.
- `src/bnf.ts` + `src/jsonic-bnf-cli.ts` — the BNF→jsonic grammar
  converter and its CLI (`bin/jsonic-bnf`).
- `src/jsonic-cli.ts` — the `jsonic` CLI (`bin/jsonic`).
- `src/debug.ts` — the debug plugin (subpath export `jsonic/debug`):
  trace lexing/parsing while diagnosing grammar behavior.
- `src/defaults.ts` — jsonic-specific option/error/hint defaults layered
  on the engine defaults.
- `src/error.ts`, `src/utility.ts` — thin re-exports of the engine's
  error and utility modules (back-compat names: `JsonicError` aliases
  `TabnasError`).

## Commands

```bash
npm install          # resolves `tabnas` via the file: path in package.json
npm run build        # tsc --build src test (emits dist/ and dist-test/)
npm test             # node --test test/**/*.test.js
TEST_PATTERN=name npm run test-some
node --test --experimental-test-coverage test/**/*.test.js
```

The `tabnas` (`file:../../parser/ts`) and `@tabnas/json`
(`file:../../json/ts`) dependencies are sibling checkouts of
`tabnas/parser` and `tabnas/json` whose `ts/` packages have been built
(`@tabnas/json` itself depends on the engine as a sibling). Tests run
against compiled output, so always `npm run build` after editing
`src/` or `test/*.ts`.

## Documentation

The docs follow a strict four-purpose split — keep each file to ONE
purpose, never mix them:

- `doc/tutorial.md` — learning: one guided happy path, no options dumps.
- `doc/guide.md`, `doc/plugins.md` — task recipes ("how to X").
- `doc/api.md`, `doc/options.md`, `doc/syntax.md` — reference: dry,
  complete, no teaching. `syntax.md` is the canonical syntax spec the
  Go port links to.
- `doc/concepts.md` — explanation: the model and rationale.
- `doc/bnf-to-jsonic-feasibility.md`, `doc/lsp-feasibility.md` —
  design-note explanations.

`README.md` is an **orientation hub**: what the package is, install, one
tiny example, links out. New detail belongs in the relevant doc above.
Ground every factual claim against `src/` and the fixtures before
writing — the current grammar accepts less than older jsonic did in a
few corners (e.g. unquoted values do not span spaces; pairs in arrays
need `list.pair`), so verify examples by running them.

## Rules of the road

- Behavior changes here are changes to the spec: the Go port (`../go/`)
  must follow. Either port in the same change or record the gap in
  `../go/doc/differences.md`.
- Shared fixtures live in `test/spec/` (`input → expected`, or
  `ERROR:<code>`). The TS suite runs them through `test/utility.js`
  (`loadTSV`); the Go suite reads the same files. Prefer adding a shared
  fixture over a one-off assertion when the case is `input → output`.
- The plugin/rule API is the engine's: register fixed tokens with
  `jsonic.options({ fixed: { token: {...} } })`, add alternates with
  `rs.open([...])` / `rs.close([...])`, and register state-action hooks
  with `rs.bo`/`rs.ao`/`rs.bc`/`rs.ac` (method calls, not assignment).
