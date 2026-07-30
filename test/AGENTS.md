# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes run
the same files, so a change here affects TypeScript and Go together — edit
with that in mind.

These moved up from `ts/test/spec/` so the fixtures sit above both runtimes
rather than inside one of them, matching @tabnas/parser.

## Format

Tab-separated, one case per line, with a header row naming the columns
(`input` `expected`, and a third column for the list-child fixtures).
The loaders unescape `\n`, `\r` and `\r\n` in every column. The `expected`
column is either a JSON value (the parse result) or `ERROR:<code>` for input
that must be rejected with that code.

## Who runs what

- TypeScript: `ts/test/utility.js` (`loadTSV`, resolving `../../test/spec`);
  `ts/test/alignment.test.js` runs the `alignment-*` fixtures, and the other
  suites take the families named after them.
- Go: `go/jsonic_test.go` (`specDir` → `../test/spec`, `parserTSVFiles`) and
  `go/feature_tsv_test.go`.

Every file in this directory is named by BOTH runners; adding a fixture means
wiring it into both, and a fixture that only one runtime runs proves nothing.

## Naming families

| Prefix | Purpose |
|---|---|
| `alignment-*` | behaviours pinned identical across TS and Go |
| `jsonic-*` | the relaxed-JSON surface |
| `feature-*` | individual relaxed-JSON features |
| `exclude-*` | option-restricted grammars (comma/strict-json off) |
| `include-json*` | the strict-JSON subset |
| `utility-*` | utility-function fixtures |

## Rules

- Prefer adding a fixture here over a one-off in-language assertion when a
  case is expressible as input → output. That is what keeps the two runtimes
  honest against each other.
- TypeScript is canonical. If the two runtimes disagree, the TS behaviour is
  the expected value — unless Go has exposed a genuine TS defect, in which
  case fix TS first and pin the corrected behaviour here.
- A new fixture must pass in BOTH runtimes: run `go test ./...` (from `go/`)
  and `npm test` (from `ts/`) before considering it done.
