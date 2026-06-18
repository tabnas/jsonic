# Concepts

Background on how jsonic is put together, and why. This is
understanding-oriented reading — for steps see the
[tutorial](tutorial.md) and [how-to guide](guide.md), and for exact
signatures see the [API](api.md) and [options](options.md) references.

## jsonic is a grammar on an engine

jsonic is two things wearing one coat:

- a **parsing engine** — a rule-based parser over a configurable,
  matcher-based lexer — which lives in the separate
  [`tabnas`](https://github.com/tabnas/parser) package, and
- the **relaxed-JSON grammar** — the rules that turn `a:1,b:2` into an
  object — which lives here, in `src/grammar.ts`.

`src/jsonic.ts` constructs a `tabnas` engine instance, installs the
grammar on it, and dresses it in the historic `Jsonic` shape: a parse
function with the management methods (`make`, `use`, `rule`, `token`,
`sub`, …) attached as properties. Everything in `src/error.ts` and
`src/utility.ts` is a thin re-export of the engine's equivalents
(`JsonicError` is the engine's `TabnasError` under its historic name).

The payoff of this split is that the grammar is just data and rules
fed to a general engine. The same engine drives the strict-JSON variant
(`Jsonic.make('json')`) and any plugin you write.

## Two stages: lexer, then parser

A parse runs in two cooperating stages.

The **lexer** turns source text into a stream of **tokens**. It is built
from independent **matchers**, one per token kind (fixed punctuation,
space, line endings, strings, comments, numbers, text, and your custom
matchers). At each position the matchers run in a fixed priority order
and the first to produce a token wins. Matchers are configured, not
hard-coded — disabling comments or adding a quote character is an
option change, and the lexer rebuilds itself from the resolved
configuration.

The **parser** consumes tokens according to named **rules** (`val`,
`map`, `list`, `pair`, `elem`). Each rule has an **open** and a
**close** phase, and each phase holds a list of **alternates**. An
alternate matches a short token pattern — at most two tokens of
lookahead — and when it matches it can run an **action** that builds the
result, **push** a child rule, **replace** the current rule, or
**backtrack** a token so another rule sees it. Four state-action hooks
(before/after open, before/after close) let a rule run code at each
phase boundary.

This open/close, push/replace model is deliberately small and strictly
deterministic: there is no backtracking search, only two-token
lookahead. That constraint keeps parsing linear and predictable, and it
is the main thing a grammar author designs around.

## Instances and derivation

A parser **instance** bundles a resolved configuration, a token table,
the rule set, and the plugins applied to it. `Jsonic.make(options)`
derives a new instance: it merges the parent's options with your
overrides and **re-runs every plugin** the parent registered against the
merged options. Re-running matters because grammar can be
option-conditional — an alternate that only exists when a flag is set
must be re-evaluated for the child, not copied stale. This is also why
plugins must be idempotent.

## Errors

A failed parse throws a `JsonicError` (which extends the platform
`SyntaxError`). It carries an error code, the source location (row,
column, position), the offending fragment, a formatted message with a
source-context extract, and an optional hint. Messages and hints are
templates with `{key}` placeholders, so they can be customised or
localised through the `error` and `hint` options.

## Design notes

Longer-form explorations live alongside this document:

- [LSP feasibility](lsp-feasibility.md) — language-server angles on the
  parser.

For how the Go port differs from this canonical behavior, see
[../../go/doc/differences.md](../../go/doc/differences.md).
