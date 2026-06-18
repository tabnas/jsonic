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
  extensions on the `val`/`map`/`list`/`pair`/`elem` rules. `@tabnas/json`
  builds its values on the engine's native-value `$`-builtins
  (`@reset$`/`@object$`/`@array$`/`@key$`/`@setval$`/`@push$`/`@value$`);
  jsonic reuses them and owns only the *relaxed* behaviour. **Maintenance
  hazards** (see the root `AGENTS.md` for the full list): implicit
  (brace-less) containers must allocate via `@object$`/`@array$` with
  `k.object$={implicit:true}`; every value-producing relaxed `val` open
  alt needs `a:'@reset$'` (else `{a:b:1}`/`[a:]` go circular); the `val`
  close coalescing is jsonic's own `@val-bc/replace` before-close hook
  (child > primitive plugin value > token > container > implicit null)
  plus a `val` **after-close** hook that restores a primitive plugin value
  over json's `@value$` close action; the `elem` close action
  (`@elem-bc/replace`) owns the guarded push + pair/child handling; the
  `pair`/`val` key alts use a `clear` alt-mod to bind jsonic's `@pairkey`
  (number/keyword keys from the token source). `/replace` takes ownership
  of a phase so the builtin is not re-installed on later
  `fnref()`/`make()`/derive. Also provides the
  strict-JSON variant selected by `Jsonic.make('json')` and exports the
  idiomatic `tabnas` plugin
  `jsonic` (apply jsonic option defaults + register grammar) and the
  `registerJsonicGrammar` helper; the legacy `make()` path installs the
  same grammar. The package is **a normal `tabnas` grammar plugin**
  (`new Tabnas().use(jsonic)`); the callable `Jsonic` API is kept only
  for backward compatibility.
- `src/defaults.ts` — jsonic-specific option/error/hint defaults layered
  on the engine defaults.

## Commands

```bash
npm install          # resolves `tabnas` via the file: path in package.json
npm run build        # tsc --build src test (emits dist/ and dist-test/)
npm test             # node --test test/**/*.test.js
TEST_PATTERN=name npm run test-some
node --test --experimental-test-coverage test/**/*.test.js
```

The `tabnas` (`file:../../parser/ts`), `@tabnas/json`
(`file:../../json/ts`) and `@tabnas/debug` (`file:../../debug/ts`)
dependencies are sibling checkouts of `tabnas/parser`, `tabnas/json` and
`tabnas/debug` whose `ts/` packages have been built (`@tabnas/json` and
`@tabnas/debug` themselves depend on the engine as a sibling; point
`@tabnas/debug`'s `tabnas` devDependency at `../../parser/ts`). Tests run
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
- `doc/lsp-feasibility.md` — design-note explanation.

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
