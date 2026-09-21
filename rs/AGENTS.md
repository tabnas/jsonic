# Agents Guide: rs/

The Rust port of the canonical TypeScript in [`../ts`](../ts). Read
[`../AGENTS.md`](../AGENTS.md) first: it holds the cross-runtime rules,
and this file only covers what is specific to this crate.

## Layout

| Path | |
|---|---|
| `src/lib.rs` | the whole port: the option branding, the two grammar documents, every closure the grammar names, `jsonic`, `register_jsonic_grammar`, `plugin`, `make`, `make_with`, `make_json`, `empty`, `parse` |
| `tests/parity_test.rs` | every standard-shaped `../test/spec/*.tsv` fixture through `tabnas_support::Runner`, with the per-file parser the Go runners build; plus the three-column list-child files |
| `tests/lex_test.rs` | the token-stream corpus `lex.tsv` |
| `tests/utility_test.rs` | the four `utility-*.tsv` files, against `tabnas::utility` |
| `tests/divergent_test.rs` | the divergence register |
| `tests/registration_test.rs` | the tripwire: every fixture is run by one of the above, and every exemption is still bespoke-shaped |
| `tests/jsonic_test.rs` | in-language behaviour: README examples, plugin layering, `MapRef`/`ListRef`/`Text`, key order, safe keys, comment defs and suffixes, selectors, strict mode, threads, `parse` reuse |
| `tests/version_test.rs` | Cargo.toml == `VERSION` == ts/package.json |
| `tests/common/mod.rs` | shared helpers: spec dir, value and failure conversion, the JavaScript number renderer |
| `README.md` | the crate front page, prose-gated; its `rust` fences are doctests of this crate (see below) |

Crate `tabnas-jsonic`, library `tabnas_jsonic`. The engine (`tabnas`),
the JSON core (`tabnas-json`) and the fixture runner (`tabnas-support`,
dev only) are **path dependencies on sibling checkouts**
(`../../parser/rs`, `../../json/rs`, `../../support/rs`). None is
published, so there is no registry version to fall back on.

```bash
cargo build --all-targets
cargo test --all-targets && cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt
```

`make test-rs` from the repository root is the fast loop; `ci/rust/run.sh`
is the full gate and adds `fmt --check`, the lockfile check and the MSRV
pin.

## How the grammar is layered

`register_jsonic_grammar` does, in order:

1. Registers every closure the grammar names (see below).
2. Snapshots the options, calls `tabnas_json::json`, then restores every
   field json's strict profile changed. **This is load-bearing.** The
   TypeScript `registerJsonGrammar` installs rules alone; the Rust
   `tabnas_json::json` is the crate's one entry point and installs its
   strict OPTIONS too (text.lex off, `KEY` = `#ST`, the number check, a
   parse budget, ...). jsonic wants the rules only. `restore_relaxed`
   lists the fields; `the_relaxed_profile_is_the_engine_default_profile`
   in `jsonic_test.rs` fails if json starts changing one it does not
   list.
3. Installs `jsonic_document`, then `jsonic_document_append`.

Two documents, not one, because of an engine ordering difference: the
engine's `apply_alt_list` applies an `inject` object's `delete` and
`move` to the EXISTING alternates before inserting the new ones, where
TypeScript's `RuleSpec.add` inserts first and modifies after. Three of
jsonic's calls depend on TypeScript's order: the `move: [1, -1]` that
sends json's "more JSON" alternate to the end of `val.close`, and the
second `.open(..., {append: true})` on `map` and `list`. Phase two is
exactly those three calls. The result is asserted alternate for
alternate against the TypeScript table in
`the_alternates_are_the_typescript_ones_in_the_typescript_order`. Do not
fold the phases together without re-running that test.

The option-conditional alternates (`map.child`, `list.child`, and the
error on a pair inside a list when neither `list.property` nor
`list.pair` is on) are decided when the document is BUILT, from the
options in force, as TypeScript's `p.cfg.*` and Go's `cfg.*` decide them.
That is why `make_with` applies the caller's options before the grammar,
and why `jsonic()` on an instance whose options are set afterwards does
not grow them. `rule.include` / `rule.exclude` are the exception: the
engine filters alternates at parse time, so they may be set at any
point.

## The closures, and why their names carry suffixes

The serialized document names every closure and the closures are
registered before it is installed. The lifecycle names all carry a phase
suffix, `@map-bo/append`, `@val-bc/replace`, and so on, and the pair-key
action is `@jsonic-pairkey`. That is deliberate: the engine resolves a
bare `@map-bo`, `@list-bo`, `@pair-bc`, `@elem-bc`, `@val-bc` or
`@pairkey` to its OWN builtin first, so a registration under one of
those names is shadowed silently and the port behaves as json. The
suffixed names never collide.

`@val-bc/replace` and `@elem-bc/replace` take ownership of their phase,
exactly as the TypeScript `'@val-bc/replace'` and `'@elem-bc/replace'`
do; the rest append.

## Two engine facts the val coalescing depends on

- **Fixed tokens carry their source text as `val`** (`}` has
  `Value::String("}")`), where the canonical engine's carry `undefined`.
  jsonic reads a fixed token whose `val` is still its own source text as
  no value (`carries_no_value`), but json's `@value$` close alternate
  re-resolves the token after the before-close hook, so `a:,b:` came back
  as `{"a":","}` until `val_after_close` put the no-value back. Keep that
  branch. The test is on the VALUE, not the token kind, on purpose: a
  plugin action in `val` open may assign a fixed token a value
  (`r.o0.val = '@' + r.o1.val`, the `parser-mixed-token` shape in
  `ts/test/custom.test.js`), and that value must survive both hooks, as
  it does in TypeScript. Treating every fixed token as valueless silently
  dropped such elements (`[QxQy]` parsed as `[]`).
- **`#ZZ` carries `Undefined`**, and `rule.o` keeps backtracked tokens,
  so `rule.os()` counts them as TypeScript's `r.os` does.

## The shared node cell

A pushed or replaced rule SHARES its parent's `Rc<RefCell<Value>>`, so
`*rule.node.borrow_mut() = v` overwrites the parent's node too. An
assignment, `r.node = v` in TypeScript, installs a fresh cell here
(`set_node`). The cell is borrowed directly only to mutate a container
the rule genuinely shares: pushing onto the enclosing list, inserting
into the enclosing map, and ONE deliberate exception, the implicit-list
promotion in `list_before_open`, which writes the new array into the
cell it shares with the replaced `val` rule's snapshot. That is
`r.prev.node = r.node` in the canonical grammar.

## The depth budget is a crash fix

`register_jsonic_grammar` ends by installing a parse budget that refuses
nesting past `DEPTH_LIMIT` (127) containers with the engine's `cancel`
code, unless the instance already carries a budget (a caller's, set
through `make_with`, wins). The engine parses iteratively, but its
display, `to_json` and drop of a value walk the tree with the call
stack, and on a 2 MiB thread the overflow arrived past 6,000 levels in a
release build and 1,500 in a debug build: an abort, not an error.
TypeScript and Go have no limit, so the refusal is a recorded divergence
(`../DIVERGENCE.md`, "The Rust port") that must never reach a shared
fixture. `restore_relaxed` deliberately drops the budget `tabnas_json`
installs along with the rest of its profile; jsonic's replaces it, with
the same limit, so the two Rust crates bound nesting alike. Depth is
counted from the `map` and `list` rule names, not `rule_stack.len()`,
for the reason `../../json/rs/AGENTS.md` gives.

## What a fixture cannot hold

- `list.child` values ride on a `ListRef.child` (a `Vec` has no
  properties), so `feature-list-child*.tsv` is run by a bespoke runner
  that reads `value` and `child` separately.
- A pair inside a list under `list.property` is parsed and discarded, as
  in Go; the serialized value agrees with TypeScript.
- Key order is document order without `map.ordered`; the cross-port
  table from `go/ordered_test.go` is pinned in `jsonic_test.rs`.

## The divergence register

`tests/divergent_test.rs` runs `../test/spec/divergent.tsv` and asserts
the **`ts` column**: the Rust port reproduces TypeScript on every row
(the engine's string lexer consults `string.replace` before the
control-character class). It has no `rust` column of its own because
`ts/test/divergent.test.js` asserts exactly six columns and `ts/` is not
this port's to change. When that runner reads by header or accepts a
seventh column, add the column (rust = ts on both rows) and switch the
test to `tabnas_support::Register::new(runner, "rust", &["go", "ts",
"rust"])`. Note the register refuses a row whose cells all agree, so the
control row would need to go through a plain `Runner`.

## The unprintable shim is not needed

The Go port carries `unprintable.go`, a pre-scan matcher that reports a
raw control character in a string as `unprintable` where the Go engine
said `unterminated_string`. The Rust engine's string lexer already
reports `unprintable` at the character, honours `string.allow_control`
and consults `string.replace` first, so `alignment-errors.tsv`,
`string-allow-control.tsv` and the register pass with no shim. Do not
add one.

## The docs are gated

`README.md` is in the published set: no em dashes in prose, no first
person singular, no links to any `AGENTS.md`, no project history. This
file is internal and may be blunt.

## The README is doctested

`src/lib.rs` includes `README.md` as rustdoc under `#[cfg(doctest)]`, so
every `rust` fence in it runs on `cargo test --doc` (they show up as
`readme_examples (line N)`). rustdoc runs each fence as written, so a
fence must be a complete program: wrap it in
`fn main() -> Result<(), Box<dyn std::error::Error>> { ... Ok(()) }`
rather than using `?` at the top level, and never use hidden `# ` lines,
which render as garbage on GitHub. The `toml` and `bash` fences are not
run.
