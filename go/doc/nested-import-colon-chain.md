# Design Note: Colon-Chain Nested `@"file"` Import — Root Cause and Fix

## Status

Investigated. **jsonic Go requires no change** — its colon-chain handling
already matches canonical TypeScript. The defect lives in the
[`@tabnas/multisource`](https://github.com/tabnas/multisource) Go port; the
fix is a one-clause condition there (see [Proposed Fix](#proposed-fix)).

This note exists because the upstream report
([`aontu/docs/design/nested-import-colon-chain.md`](https://github.com/rjrodger/aontu/blob/main/docs/design/nested-import-colon-chain.md))
attributed the defect to "the jsonic/Go parser." That attribution is
incorrect; the analysis below records why, so the fix is not mistakenly
applied here.

## Symptom

With the multisource import plugin loaded, a *colon-chain* (path-dive) key
whose value is a bare `@"file"` import silently drops the import. The same
import nested with explicit braces works:

```
struct: minor: @"minor.aon"     =>  {struct:{minor:null}}    ❌ import lost
struct: {minor: @"minor.aon"}   =>  {struct:{minor:{…}}}     ✅ braced works
struct: @"minor.aon"            =>  {struct:{…}}              ✅ direct works
a: b: c: @"minor.aon"           =>  {a:{b:{c:null}}}          ❌ lost at any depth
```

| Configuration style | TypeScript | Go (before fix) |
|---|---|---|
| Direct assignment (`a: @"f"`)        | ✅ | ✅ |
| Braced nesting (`a: {b: @"f"}`)      | ✅ | ✅ |
| Colon-chain nesting (`a: b: @"f"`)   | ✅ | ❌ |

## Reproduction

The bug needs the real plugin stack — `@tabnas/directive` (which binds the
`@` open token to a value-position rule) plus `@tabnas/multisource` (which
layers the path/`pk` handling on top). jsonic Go on its own, and a plain
directive on its own, both parse colon-chains correctly; the failure only
appears once multisource's `custom` grammar is installed.

```go
fsys := fstest.MapFS{"minor.aon": {Data: []byte(`{x:1}`)}}
j := ms.MakeJsonic(ms.MultiSourceOptions{Resolver: ms.MakeFileResolver(), FS: fsys})
got, _ := j.Parse(`struct: minor: @"minor.aon"`)
// got == {struct:{minor:<nil>}}   (import dropped)
```

## Why jsonic Go is **not** the cause

1. **The grammar is identical to TS.** The `val`, `map`, and `pair` rules in
   [`go/grammar.go`](../grammar.go) match
   [`ts/src/grammar.ts`](../../ts/src/grammar.ts) alt-for-alt, including the
   path-dive alternates:

   ```
   // val open — pair dive (d > 0): increment the pair-key depth counter
   {S: "#KEY #CL", P: "map", B: 2, N: map[string]int{"pk": 1}, A: "@reset$", …}
   ```

2. **The `n.pk` counter is correct and matches the documented contract.**
   `pk` records pair-key (path-dive) depth at the value position:
   `a:9 → pk=0`, `a:b:9 → pk=1`, `a:b:c:9 → pk=2`. A rule-step trace of the
   failing parse confirms Go reaches the innermost value rule for `minor`
   with `pk=1` and `parent.name == "pair"` — exactly the state TypeScript
   reaches. The colon-chain is *nested*, not flattened.

3. **Pure-jsonic colon-chains already round-trip identically** in both
   runtimes (shared `ts/test/spec/*.tsv` fixtures), so there is no jsonic-side
   parity gap to close.

In short: the design note's "Go flattens colon-chains, returning to depth 0
before lexing `@`" describes the *observed* behavior of the whole stack, but
the cause is not jsonic flattening the dive — it is a downstream plugin
mis-reading the (correct) dive state.

## Actual root cause: `@tabnas/multisource` Go port

multisource's `custom` callback adds grammar that decides whether an `@` in
value position is handled *here* or deferred *up* the path dive. The
canonical TypeScript uses **two distinct** conditions
(`ts/src/multisource.ts`):

```ts
// val open — back-track the @ ONLY when inside a dive AND not a pair value
{ s: [OPEN], c: (r) => 0 < r.n.pk && 'pair' != r.parent.name, b: 1 },
// map close / pair close — back-track on pk alone
close: [{ s: [OPEN], c: (r) => 0 < r.n.pk, b: 1 }],
```

The Go port collapsed these into a **single** shared condition and reused it
for the `val`-open alt (`go/plugin.go`):

```go
"@pk-pos": jsonic.AltCond(func(r *jsonic.Rule, ctx *jsonic.Context) bool {
    return r.N["pk"] > 0          // missing: && parent is not a pair
}),
```

The dropped `&& 'pair' != r.parent.name` clause is the whole bug. In a
colon-chain the value-position `val` rule has `pk > 0` **and** its parent is
the `pair` for that key. TypeScript's condition is therefore **false**, so it
falls through to the directive's `val`-open alt and resolves the import
*nested under the key*. Go's condition is **true**, so it back-tracks the `@`
and unwinds the path; by the time the directive finally re-fires (at
`pk == 0`, the wrong level) the key has already been finalised to `null` and
the import argument is gone.

This is precisely the "keep the parser in value-position" behavior the
upstream note credits to TypeScript — it just lives in multisource, not
jsonic.

## Proposed Fix

Give the multisource `val`-open back-track its own condition that mirrors TS,
leaving the `map`/`pair` close conditions on plain `pk > 0`
(`@tabnas/multisource`, `go/plugin.go`):

```go
"@pk-pos-val": jsonic.AltCond(func(r *jsonic.Rule, ctx *jsonic.Context) bool {
    return r.N["pk"] > 0 &&
        (r.Parent == nil || r.Parent == jsonic.NoRule || r.Parent.Name != "pair")
}),
```

and reference it from the `val`-open alt:

```go
"val": {
    Open: []*jsonic.GrammarAltSpec{
        {S: openToken, C: "@pk-pos-val", B: 1},   // was C: "@pk-pos"
        {S: openToken, C: "@d-zero", P: "map", B: 1, N: map[string]int{topCounter: 1}},
    },
},
```

(The `nil`/`NoRule` guards are the Go-idiomatic equivalent of reading
`r.parent.name` in TS, where the root rule's parent is the `NORULE`
sentinel.)

### Verification

With this change applied to a local `@tabnas/multisource` checkout, every
colon-chain depth resolves the import nested correctly:

```
struct: @"minor.aon"          =>  {struct:"{x:1}"}
struct: {minor: @"minor.aon"} =>  {struct:{minor:"{x:1}"}}
struct: minor: @"minor.aon"   =>  {struct:{minor:"{x:1}"}}   ✅ fixed
a: b: c: @"minor.aon"         =>  {a:{b:{c:"{x:1}"}}}         ✅ fixed
```

The existing `@tabnas/multisource` Go test suite (`go test ./...`) stays
green, so the added clause narrows the back-track without regressing the
leading-`@`, sibling, or nested-relative-load cases.

## Recommended interim workaround

Until the multisource fix ships, use explicit bracing instead of a
colon-chain for an imported value — it works on both runtimes at any depth:

```
struct: { minor: @"minor.aon" }
```
