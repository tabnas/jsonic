# Divergences

TypeScript is the canonical implementation; the Go port tracks it. This
file records where the two ports produce a **different result for the same
input**.

## The live register is executable

[`test/spec/divergent.tsv`](test/spec/divergent.tsv) is the authority, not
this page. Unlike a prose list, it is **run by both suites**: every row
states what each port actually produces today, the Go runner must reproduce
the `go` column and the TS runner the `ts` column, on every run.

That means a divergence which gets **fixed** fails the suite as loudly as
one that regresses, and the row must then be deleted. Prose cannot do that,
and did not: `go/doc/differences.md` claimed `2.e3` and `1e999` still
diverged for some time after they had been aligned, and claimed
base-prefixed overflow was aligned before it was. A ledger nothing executes
is documentation, not a gate.

So: **read the .tsv for what diverges today.** This page explains the
shape of the disagreements and the ones that are deliberate.

## Currently divergent

One row, at the time of writing — see the .tsv for the authoritative list.

### `string.replace` of a control character

With `{"string":{"replace":{"\n":"X"}}}`, the input `"a\nc"`:

| | result |
|---|---|
| TypeScript | `"aXc"` |
| Go | `ERROR: unprintable` |

TS consults the replacement map and a mapped control character becomes
legal string body. The Go string matcher rejects the raw control character
before replacement is consulted.

Replacement of **printable** characters is aligned, and the .tsv keeps a
control row adjacent to that one precisely so a fix to the divergent case
cannot be mistaken for a regression in the aligned case.

## Not divergences

These differ between the ports but never change a successful parse value,
and are covered in [`go/doc/differences.md`](go/doc/differences.md):

- **Error message text.** Only the error `code` is contractual.
- **Host type representation.** Go returns `int64`; TS a `number` or
  `bigint` by magnitude. The serialised bytes agree.
- **Empty / whitespace input**, token consumption details, and the
  `Info.*` option shapes — API surface, not parse results.

## Upstream

Most of what reaches a jsonic user comes from
[`@tabnas/parser`](https://github.com/tabnas/parser), whose own
`DIVERGENCE.md` records engine-level non-parity — notably lone surrogates
in quoted strings. A divergence seen here is more often inherited than
introduced; check upstream first.
