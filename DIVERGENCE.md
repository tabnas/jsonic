# Divergences

TypeScript is the canonical implementation; the Go and Rust ports track
it. This file records where the ports produce a **different result for the
same input**.

## The live register is executable

[`test/spec/divergent.tsv`](test/spec/divergent.tsv) is the authority, not
this page. Unlike a prose list, it is **run by every suite**: every row
states what each port actually produces today, and each runner must
reproduce its own column, on every run. The register carries one column
per runtime, `go`, `ts` and `rust`, read by header name, and an error cell
pins the reported position as well as the code.

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

## The Rust port

The register carries a `rust` column beside `go` and `ts`, and
`rs/tests/divergent_test.rs` asserts it. All three runners read columns
by header name, so each port records its own measured answer and a row
where one stops agreeing fails that port's suite, naming the row.

Two rows record a Rust answer that is not TypeScript's, and the register
is where to read them rather than this paragraph: `number-sep-space`,
where Go and Rust both decline a run TypeScript reads, and
`string-replace-control-row`, where Rust alone reports the row after the
replaced newline. Rust reproduces the TypeScript answer on every other
row.

Where it does, that is the engine agreeing rather than a coincidence: the
Rust string lexer consults the `string.replace` map before the
control-character class, exactly as the canonical lexer does, so
`"a\nc"` under `{"string":{"replace":{"\n":"X"}}}` is `"aXc"` there too,
and on that row only the reported ROW splits, not the value or the code.
The rule for a Rust-only split is the same as for a Go one: repair it, or
record what Rust produces in the `rust` cell and explain the shape here.

Rust inherits the engine-level splits recorded in `@tabnas/parser`'s own
`DIVERGENCE.md` (lone surrogates fold to U+FFFD; the regular expression
dialect is the `regex` crate's, with no lookaround).

### Nesting past 127 levels is refused in Rust

| input | TypeScript | Go | Rust |
|---|---|---|---|
| 128 nested `[` | parses | parses | `ERROR:cancel` |

**Deliberate, and Rust-only.** The engine parses iteratively, but the
value it returns is walked with the call stack to display, convert to
JSON or drop, one frame per level, and a source a few thousand levels
deep ended the process with a stack overflow (past 6,000 levels in a
release build and 1,500 in a debug build, on a 2 MiB thread): an abort
no caller can catch. A parse guard in `rs/src/lib.rs` refuses the 128th
container with the engine's `cancel` code, whether it is a list, a map
or one of the implicit maps a pair dive opens, and whatever budget the
caller sets. The number is the one `tabnas-json` and `serde_json` use.
Pinned by `nesting_is_bounded_by_the_depth_guard` in
`rs/tests/jsonic_test.rs`
rather than by a register row, because the input is 128 nested brackets
and a shared fixture must stay runnable in every port; it must never
reach one.

Measured against the TypeScript suite's own assertions
(`ts/test/feature.test.js`, `custom.test.js`, `comment.test.js` and
`error.test.js`: 399 inputs, 294 of them in no fixture), two further
engine-level splits stand. Both are now ROWS in the register rather than
paragraphs here, so each is executed by all three suites, and they do not
split the same way:

- `number-sep-space`. A number separator that is also whitespace, at the
  end of a number. Go and Rust both decline the run; TypeScript reads it.
  Not Rust-only.
- `string-replace-control-row`. The reported ROW of a control character
  mapped through `string.replace`. Go and TypeScript agree at 2:6 and
  Rust reports 3:1, so this one IS Rust-only. The code is the same in all
  three, and the code is the contract.

The register's cells pin a position as well as a code
(`ERROR:<code>@<row>:<col>`), which is what lets the second of those be
recorded: the three ports agree on `unprintable` and disagree on where
they say it happened. An unknown-escape COLUMN split stood beside it and
has closed: under `string.allowUnknown: false`, `"\w"` is reported at 1:3
in TypeScript, Go and Rust alike.

One further difference is deliberately not a register row:

- **A source that is only comments or whitespace.** TypeScript returns
  `undefined`; the Rust engine folds every `undefined` in a finished
  parse to `null`, so `#`, `//`, `/**/` and a lone space parse to `Null`
  (the empty string alone is `Undefined`, the `lex.empty` result).
  `alignment-empty.tsv` already writes `null` for these rows, and Go has
  one `nil` for both, so no serialized value changes and every runtime
  cell would read the same. The register refuses a row whose cells all
  agree, correctly: there is nothing there to diverge.

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
