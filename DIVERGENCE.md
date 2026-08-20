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

**No count here, deliberately.** This page used to say "one row, at the
time of writing", which was literally accurate about the file and wrong
about the world: three whole divergence classes were live and unrecorded
while it said so. A count in prose is a claim that rots the moment the
ledger changes and nothing checks it. Read
[`test/spec/divergent.tsv`](test/spec/divergent.tsv) for what diverges
today; what follows is the SHAPE of each class, not an inventory.

### Downstream of engine defects

Three classes, all rooted in `@tabnas/parser` and all closing when the
engine repair is adopted. Each ledger row names the PR that closes it, and
goes red when it lands.

**A quote ends a text run in Go and not here.** The text matcher's ender
set omits the string quote characters in Go, so `a"b` reaches the string
matcher there and reports `unterminated_string`, while TypeScript keeps the
quote inside the text value. `{a:1"}` is worth seeing: TypeScript's value
is the *string* `"1\""`, not the number `1` — the divergence changes the
type of the parsed value, not only whether the document parses. Roughly
two thirds of the 1,612 divergences in this repo's own 6,000-case fuzz run
were this shape. Closes with `tabnas/parser#128`.

**A text run crosses U+2028/U+2029 in Go and not here.** JavaScript's `.`
excludes four line terminators and RE2's excludes one. Closes with
`tabnas/parser#125`.

**A malformed `\u` escape is ACCEPTED here.** `parseInt` used as a
validator stops at the first non-hex character and returns what it read, so
`"p\u00st"` decodes and silently discards the `st`, emitting a character
that was never in the input. Go rejects it.

> This one is a **defect in the canonical port on its own terms**, not a
> defensible difference. `test/AGENTS.md` says that where Go has exposed a
> genuine TS defect, TS is fixed first and the corrected behaviour pinned —
> and that is the intended outcome here. The ledger row exists only because
> the repair is in the ENGINE (`tabnas/parser#123`), not in this repo,
> which pins a published parser. The row is an admission of parity debt, in
> the register's own words, and must not be read as the expected behaviour.
> When #123 is adopted the row goes red and is deleted, not updated.

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
