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

## The Rust port

The Rust port (`rs/`) reproduces the **TypeScript** column of every row
in the register today: the Rust engine's string lexer consults the
`string.replace` map before the control-character class, exactly as the
canonical lexer does, so `"a\nc"` under `{"string":{"replace":{"\n":"X"}}}`
is `"aXc"` there too. `rs/tests/divergent_test.rs` asserts that column
and fails, naming the row, the day Rust stops agreeing.

The register has no `rust` column because `ts/test/divergent.test.js`
asserts exactly six columns; adding one is a change to the TypeScript
runner first. Until then the rule for a Rust-only split is the same as
for a Go one: repair it, or record it here AND add the column with the
Rust runner switched to the support crate's `Register`.

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
no caller can catch. A parse budget in `rs/src/lib.rs` refuses the 128th
container with the engine's `cancel` code, whether it is a list, a map
or one of the implicit maps a pair dive opens. The number is the one
`tabnas-json` and `serde_json` use. Pinned by
`nesting_is_bounded_by_the_depth_budget` in `rs/tests/jsonic_test.rs`;
it must never reach a shared fixture, and it belongs in the register
under a `rust` column the day the TypeScript runner can take one.

Measured against the TypeScript suite's own assertions
(`ts/test/feature.test.js`, `custom.test.js`, `comment.test.js` and
`error.test.js`: 399 inputs, 294 of them in no fixture), three further
engine-level splits stand. None is Rust-only, and none can be registered
here until the register takes a `rust` column:

- **A number separator that is also whitespace, at the end of a
  number.** Under `number.sep: ' '`, TypeScript reads
  `a:1 0, b : 2 000 ` as `{"a":10,"b":2000}`: its regexp backtracks off
  the trailing space, which is an ender. The Go and Rust scanners consume
  the trailing separator and decline the whole run, so both report
  `unexpected` at 1:14. The default separator `_` is not an ender in any
  port, which is why the shared `alignment-number-prefix-separator.tsv`
  rows agree everywhere; only a whitespace separator splits. This is the
  engine's number scanner (`parser/rs/src/lexer.rs`,
  `scan_number_digits`, and its Go counterpart), and TypeScript against
  both ports, so it belongs in the engine's register.
- **Error columns inside a string.** Only the code is contractual, and
  the codes agree; two positions do not. An unknown escape under
  `string.allowUnknown: false` is reported on the backslash in Rust
  (`"\w"` at 1:2) and on the escaped character in TypeScript and Go
  (1:3). A control character replaced through `string.replace` does not
  advance the row in TypeScript or Go, so the `\r` in `x:\n "ac\n\r"`
  under `{"\n":"X"}` is reported at 2:6 there and at 3:1 in Rust, where
  the replaced newline counts as a line.
- **A source that is only comments or whitespace.** TypeScript returns
  `undefined`; the Rust engine folds every `undefined` in a finished
  parse to `null`, so `#`, `//`, `/**/` and a lone space parse to `Null`
  (the empty string alone is `Undefined`, the `lex.empty` result).
  `alignment-empty.tsv` already writes `null` for these rows, and Go has
  one `nil` for both, so no serialized value changes.

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
