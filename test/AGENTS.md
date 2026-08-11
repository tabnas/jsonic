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

The files are read by
[`@tabnas/support`](https://github.com/tabnas/support) and its Go half —
**one loader, in two languages written to behave identically**. Both
`loadTSV`s are now thin shims over it, returning the shapes their callers
already expect.

**Escaping applies to the `input` column only, in both runtimes**: `\n`,
`\r`, `\t` and `\\` are decoded there. Every other column is taken as
written, so the `expected` column is raw JSON and JSON's own escape rules
apply to it — write `"a\nb"` for a string holding a newline, exactly as
you would in a `.json` file.

That rule is why the loader is now shared. The two used to disagree about
it, twice: TypeScript decoded *every* column and did not handle `\t`, so
an `expected` value containing a JSON escape became a raw control
character and failed `JSON.parse` while a `\t` in `input` reached the
parser as a literal backslash-t; and TypeScript kept `#`-leading comment
lines that the Go runners dropped, so a documented fixture ran clean in Go
and crashed here. Both were fixed by hand. A row that means two different
things in two runtimes cannot pin agreement on anything else, so there is
one loader instead of two that have to be kept in step.

`\\` is newly decodable, so a fixture can carry a literal backslash. No
existing cell changes meaning — every input cell was compared under both
rules.

## Who runs what

- TypeScript: `ts/test/utility.js` (`loadTSV`, over the shared loader);
  `ts/test/alignment.test.js` runs the `alignment-*` fixtures, and the other
  suites take the families named after them.
- Go: `go/jsonic_test.go` (`loadTSV` and `specDir`, over the shared loader,
  plus `parserTSVFiles`) and `go/feature_tsv_test.go`.

`specDir` finds `test/spec` by walking up, rather than counting `..` hops,
and a failure names the fixture's own physical line number.

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

## Never author an expected value — probe for it

`scripts/parity-probe.sh <file-of-sources>` runs each source through BOTH
ports and prints one line per candidate:

```
AGREE   <src>\t<value>       -> paste straight into a fixture
DIFFER  <src>\t<go>\t<ts>    -> adjudicate; if deliberate, add to divergent.tsv
```

A row is only trustworthy if both engines were asked. Hand-written rows pin
what the author believed, and that has gone wrong here more than once: a
base-prefix overflow row was authored as 2^72 when the literal is 2^76 (the
parser was right, the arithmetic was not), and a fixture read as "failing"
because its author compared JSON strings where the runners compare parsed
values. Downstream adopted this same workflow after finding 15 of 254 rows
in a supposed parity corpus encoded one port's behaviour only.

The two probes share no rendering code — only the source list and the
output convention (`ERROR:<code>`, or the value as JSON, with non-finite
numbers as `"@@Infinity"` / `"@@-Infinity"` / `"@@NaN"`). A shared renderer
could hide a divergence inside itself. The non-finite markers exist because
the JSON encoders disagree about how to fail on ±Inf — without them `1e400`
reports DIFFER while both parsers agree, which is the one thing a probe
must never do.

