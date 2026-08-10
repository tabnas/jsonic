# Agents Guide — shared spec fixtures

`spec/*.tsv` holds the cross-runtime conformance fixtures. Both runtimes run
the same files, so a change here affects TypeScript and Go together — edit
with that in mind.

These moved up from `ts/test/spec/` so the fixtures sit above both runtimes
rather than inside one of them, matching @tabnas/parser.

## Format

Tab-separated, one case per line, with a header row naming the columns
(`input` `expected`, and a third column for the list-child fixtures). The
`expected` column is either a JSON value (the parse result) or `ERROR:<code>`
for input that must be rejected with that code.

**Escaping applies to the `input` column only, in both runtimes.** Each
runner decodes `\n`, `\r`, `\r\n` and `\t` there: the TypeScript `loadTSV`
unescapes `cols[0]`, and the Go `loadTSV` returns columns verbatim with its
callers applying `preprocessEscapes` to the parser-input column. Every other
column is taken as written, so the `expected` column is raw JSON and JSON's
own escape rules apply to it — write `"a\nb"` for a string holding a newline,
exactly as you would in a `.json` file.

The two loaders used to disagree here: TypeScript decoded *every* column and
did not handle `\t`, so an `expected` value containing a JSON escape became a
raw control character and failed `JSON.parse`, while a `\t` in `input` reached
the parser as a literal backslash-t. Both are fixed; the note that once told
you to keep escapes out of non-input columns no longer applies.

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
