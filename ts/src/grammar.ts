/* Copyright (c) 2013-2024 Richard Rodger, MIT License */

/*  grammar.ts
 *  Grammar definition.
 *
 *  First, a pure JSON grammar is defined. Then it is extended to provide the
 *  Jsonic format.
 */

import type { Plugin } from '@tabnas/parser'

// The standard-JSON grammar core (val / map / list / pair / elem) is
// provided by the @tabnas/json plugin; jsonic layers its relaxed
// extensions on top of it instead of re-declaring the JSON grammar.
import { registerJsonGrammar } from '@tabnas/json'

import { Jsonic, Rule, RuleSpec, Context, Parser, FuncRef } from './jsonic'

import { defaults } from './defaults'

const defprop = Object.defineProperty

function mark(node: any, marker: string, data: any): void {
  if (node != null && typeof node === 'object') {
    defprop(node, marker, { value: data, writable: true })
  }
}

function grammar(jsonic: Jsonic) {
  const { deep } = jsonic.util

  const {
    // Fixed tokens
    // OB, // Open Brace `{`
    // CB, // Close Brace `}`
    // OS, // Open Square `[`
    // CS, // Close Square `]`
    // CL, // Colon `:`
    CA, // Comma `,`

    // Complex tokens
    TX, // Text (unquoted character sequence)
    ST, // String (quoted character sequence)

    // Control tokens
    ZZ, // End-of-source
  } = jsonic.token

  const {
    VAL, // All tokens that make up values
    // KEY, // All tokens that make up keys
  } = jsonic.tokenSet

  const fnm: Record<FuncRef, Function> = {
    '@finish': (_rule: Rule, ctx: Context) => {
      if (!ctx.cfg.rule.finish) {
        // TODO: pass missing end char for replacement in error message
        ctx.t0.err = 'end_of_source'
        return ctx.t0
      }
    },

    // TODO: define a way to "export" rule actions or other functions so that
    // other plugins can use them.
    '@pairkey': (r: Rule) => {
      // Get key string value from first matching token of `Open` state.
      const key_token = r.o0
      const key =
        ST === key_token.tin || TX === key_token.tin
          ? key_token.val // Was text
          : key_token.src // Was number, use original text

      r.u.key = key
    },
  }


  // Plain JSON
  // ----------
  //
  // The standard-JSON rule set (val / map / list / pair / elem) is now
  // supplied by the @tabnas/json grammar plugin. jsonic installs that
  // shared core and layers its relaxed extensions on top of it, rather
  // than re-declaring the JSON grammar here.
  registerJsonGrammar(jsonic as any)

  // @tabnas/json wires a strict @val-bc that overwrites the value node
  // from the matched token. jsonic's relaxed grammar instead preserves a
  // node a plugin set in a `val` open-alt action, and treats a value with
  // no matched token as undefined (implicit null). Replace the val close
  // action with jsonic's fuller version.
  jsonic.rule('val', (rs: RuleSpec) => {
    // The `@val-bc/replace` funcref takes ownership of the val close phase:
    // it clears @tabnas/json's strict @val-bc and installs jsonic's, and
    // because the phase is then "replaced" the strict one is not
    // re-installed by later fnref() calls or on Derive/make().
    rs.fnref({
      '@val-bc/replace': (r: Rule, ctx: Context) => {
        // Stash the value a plugin set in a val OPEN action (before any
        // coalescing). @tabnas/json's @value$ close ALT action still runs
        // after this and re-resolves the matched token, which would
        // overwrite a plugin value; the @val-ac after-close hook below
        // restores it. A normal value rule @reset$s its node in open, so
        // this is undefined except when a plugin deliberately set it.
        r.u.openval = r.node

        const resolveToken = () => {
          let val = r.o0.resolveVal(r, ctx)
          if (
            ctx.cfg.info.text &&
            typeof val === 'string' &&
            (r.o0.tin === ctx.cfg.t.ST || r.o0.tin === ctx.cfg.t.TX)
          ) {
            let quote =
              r.o0.tin === ctx.cfg.t.ST && r.o0.src.length > 0 ? r.o0.src[0] : ''
            let sv = new String(val)
            mark(sv, ctx.cfg.info.marker, { quote })
            val = sv as any
          }
          return val
        }

        r.node =
        // A child map/list node wins (the value was a container),
        undefined !== r.child.node
          ? r.child.node
          : // else a deliberate PRIMITIVE value a plugin set in a val open
            // action (a stale parent-seeded node is always a container, so
            // a non-object node here is an intentional scalar value),
            null != r.node && 'object' !== typeof r.node
              ? r.node
              : // else the matched scalar token — this beats a stale
                // parent-seeded container node,
                0 !== r.os
                  ? resolveToken()
                  : // else a deliberate container a plugin set (no token),
                    undefined !== r.node
                      ? r.node
                      : // else no value -> undefined (implicit null).
                        undefined
      },
    })
      // After-close: @tabnas/json's @value$ close alt re-resolves the
      // matched token and so overwrites a value a plugin set in a val open
      // action (e.g. `{ s:[NOT], a:r=>r.node='<not>' }`). Restore the
      // plugin's value, but only a PRIMITIVE one set with no child: a
      // parent-seeded stale node is always a container (object/array), so
      // restricting to non-objects restores deliberate scalar plugin
      // values without disturbing normal coalescing.
      .ac((r: Rule) => {
        const ov = r.u.openval
        if (
          null != ov &&
          'object' !== typeof ov &&
          undefined === r.child.node
        ) {
          r.node = ov
        }
      })
  })



  // Jsonic syntax extensions.
  // NOTE: undefined values are still removed, as JSON does not have "undefined", only null.

  // Counters.
  // * pk: depth of the pair-key path
  // * dmap: depth of maps

  function pairval(r: Rule, ctx: Context) {
    let key = r.u.key
    let val = r.child.node

    // Convert undefined to null when there was no pair value
    val = undefined === val ? null : val

    // Do not set unsafe keys on Arrays (Objects are created without a prototype)
    if (r.u.list && ctx.cfg.safe.key) {
      if ('__proto__' === key || 'constructor' === key) {
        return
      }
    }

    // Drop keys that match the info marker to preserve metadata.
    if (ctx.cfg.info.map && key === ctx.cfg.info.marker) {
      return
    }

    // The previous value at this key (the engine's @setval$ builtin no
    // longer threads it through r.u.prev): read it straight off the node
    // so a repeated key (`a:1,a:2`) or a deep object can merge/extend.
    const prev = r.node[key]

    val = null == prev
      ? val
      : ctx.cfg.map.merge
        ? ctx.cfg.map.merge(prev, val, r, ctx)
        : ctx.cfg.map.extend
          ? deep(prev, val)
          : val

    r.node[key] = val
  }



  jsonic.grammar({
    ref: {
      '@val-close-error': (r: Rule, c: Context) => (0 === r.d ? c.t0 : undefined),
    },

    rule: {
      val: {
        open: {
          alts: [
            // A pair key: `a: ...`
            // Implicit map at top level.
            // @reset$ clears the parent-seeded node (mirrors json's #OB/#OS
            // open alts) so val-close coalesces to the pushed map, not the
            // inherited parent container.
            {
              s: '#KEY #CL',
              c: { d: 0 },
              p: 'map',
              b: 2,
              a: '@reset$',
              g: 'pair,jsonic,top',
            },

            // A pair dive: `a:b: ...`
            // Increment counter n.pk to indicate pair-key depth (for extensions).
            // a:9 -> pk=undef, a:b:9 -> pk=1, a:b:c:9 -> pk=2, etc
            // @reset$ as above: without it a dive inside an explicit map
            // (`{a:b:1}`) coalesces a's value to the outer map (a circular
            // self-reference) instead of the nested `{b:1}`.
            {
              s: '#KEY #CL',
              p: 'map',
              b: 2,
              n: { pk: 1 },
              a: '@reset$',
              g: 'pair,jsonic',
            },

            // A plain value: `x` `"x"` `1` `true` ....
            // @reset$ clears the parent-seeded node so the scalar doesn't
            // inherit the parent container (mirrors @tabnas/json's #VAL
            // open alt that the `delete:[2]` below removes).
            { s: '#VAL', a: '@reset$', g: 'val,json' },

            // Implicit ends `{a:}` -> {"a":null}, `[a:]` -> [{"a":null}]
            // @reset$ so the empty value resolves to null (via @val-bc)
            // rather than keeping the inherited parent container.
            {
              s: ['#CB #CS'],
              b: 1,
              c: { d: { $gt: 0 } },
              a: '@reset$',
              g: 'val,imp,null,jsonic',
            },

            // Implicit list at top level starting with a comma: `,` -> [null].
            // Allocate the (implicit) array here — this path does not go
            // through @list-bo's implist promotion, and @tabnas/json's
            // @array$ only runs for `[`.
            {
              s: '#CA',
              c: { d: 0 },
              p: 'list',
              b: 1,
              a: '@array$',
              k: { array$: { implicit: true } },
              g: 'list,imp,jsonic',
            },

            // Value is implicitly null when empty before commas.
            { s: '#CA', b: 1, a: '@reset$', g: 'list,val,imp,null,jsonic' },

            { s: '#ZZ', g: 'jsonic' },

          ],
          inject: { append: true, delete: [2] },
        },

        close: {
          alts: [

            // Explicitly close map or list: `}`, `]`
            {
              s: ['#CB #CS'],
              b: 1,
              g: 'val,json,close',
              e: '@val-close-error', // (r, c) => (0 === r.d ? c.t0 : undefined),
            },

            // Implicit list (comma sep) only allowed at top level: `1,2`.
            {
              s: '#CA',
              c: { 'n.dlist': { $lte: 0 }, 'n.dmap': { $lte: 0 } },
              r: 'list',
              u: { implist: true },
              g: 'list,val,imp,comma,jsonic',
            },

            // Implicit list (space sep) only allowed at top level: `1 2`.
            {
              c: { 'n.dlist': { $lte: 0 }, 'n.dmap': { $lte: 0 } },
              r: 'list',
              u: { implist: true },
              g: 'list,val,imp,space,jsonic',
              b: 1,
            },

            { s: '#ZZ', g: 'jsonic' },

          ],
          inject: {
            append: true,

            // Move "There's more JSON" to end.
            move: [1, -1],
          }
        }
      }
    }
  })



  jsonic.rule('map', (rs: RuleSpec) => {
    rs
      .fnref({
        ...fnm
      })
      .bo((r: Rule) => {
        // Increment depth of maps.
        r.n.dmap = 1 + (r.n.dmap ? r.n.dmap : 0)
      })
      .open([
        // Auto-close; fail if rule.finish option is false. Allocate the
        // (empty) object so `{` -> `{}` when finish is allowed.
        { s: '#OB #ZZ', b: 1, a: '@object$', e: '@finish', g: 'end,jsonic' },
      ])
      .open(
        [
          // Pair from implicit map (no braces). @tabnas/json's map-open
          // alts only match `#OB`, so the brace-less entry must allocate
          // the container itself: @object$ with the static implicit:true
          // flag (the brace-less counterpart of json's implicit:false).
          {
            s: '#KEY #CL',
            p: 'pair',
            b: 2,
            a: '@object$',
            k: { object$: { implicit: true } },
            g: 'pair,list,val,imp,jsonic',
          },
        ],
        { append: true },
      )
      .close(
        [
          // Normal end of map, no path dive.
          {
            s: '#CB',
            c: { 'n.pk': { $lte: 0 } },
            g: 'end,json',
          },

          // Not yet at end of path dive, keep ascending.
          { s: '#CB', b: 1, g: 'path,jsonic' },

          // End of implicit path
          { s: ['#CA #CS #VAL'], b: 1, g: 'end,path,jsonic' },

          // Auto-close; fail if rule.finish option is false.
          { s: '#ZZ', e: '@finish', g: 'end,jsonic' },
        ],
        { append: true, delete: [0] },
      )
      .bc((r: Rule, ctx: Context) => {
        let m = ctx.cfg.info.marker
        if (ctx.cfg.info.map && r.node?.[m]) {
          r.node[m].implicit = !(r.o0 && r.o0.tin === ctx.cfg.t.OB)
        }
      })
  })

  jsonic.rule('list', (rs: RuleSpec) => {
    rs
      .fnref({
        ...fnm,
        '@list-bo': (r: Rule, ctx: Context) => {
          // Increment depth of lists.
          r.n.dlist = 1 + (r.n.dlist ? r.n.dlist : 0)

          // For an implicit (bracket-less) list, @tabnas/json's @array$
          // never runs (its list-open alts only match `#OS`), so allocate
          // the array here, mark it implicit when info.list is on, then
          // promote the already-parsed first value into it.
          if (r.prev.u.implist) {
            r.node = []
            if (ctx.cfg.info.list) {
              mark(r.node, ctx.cfg.info.marker, { implicit: true, meta: {} })
            }
            r.node.push(r.prev.node)
            r.prev.node = r.node
          }
        }
      })
      // .bo('@bo')
      .open({
        c: { 'prev.u.implist': { $eq: true } },
        p: 'elem',
      })
      .open(
        [
          // Initial comma [, will insert null as [null,
          { s: '#CA', p: 'elem', b: 1, g: 'list,elem,val,imp,jsonic' },

          // Another element.
          { p: 'elem', g: 'list,elem,jsonic' },
        ],
        { append: true },
      )
      .close(
        [
          // Fail if rule.finish option is false.
          { s: '#ZZ', e: '@finish', g: 'end,jsonic' },
        ],
        { append: true },
      )
      .bc((r: Rule, ctx: Context) => {
        let m = ctx.cfg.info.marker
        if (ctx.cfg.info.list && r.node?.[m]) {
          r.node[m].implicit = !(r.o0 && r.o0.tin === ctx.cfg.t.OS)
        }
      })
  })

  // sets key:val on node
  jsonic.rule('pair', (rs: RuleSpec, p: Parser) => {
    rs
      .fnref({
        ...fnm,
        '@pair-bc': (r: Rule, ctx: Context) => {
          if (r.u.pair) {
            pairval(r, ctx)
          }

          if (true === r.u.child) {
            let val = r.child.node
            val = undefined === val ? null : val
            let prev = r.node['child$']

            if (undefined === prev) {
              r.node['child$'] = val
            } else {
              r.node['child$'] =
                ctx.cfg.map.merge
                  ? ctx.cfg.map.merge(prev, val, r, ctx)
                  : ctx.cfg.map.extend
                    ? deep(prev, val)
                    : val
            }
          }
        }
      })

      .open(
        [
          // Re-declare the key alt so it binds jsonic's @pairkey (which
          // uses the key token's *source* for number and value-keyword
          // keys, e.g. `1:x` -> "1", `__proto__:1`), replacing the strict
          // @tabnas/json version that uses the decoded token value. The
          // `clear` below drops @tabnas/json's pair open alts first.
          {
            s: '#KEY #CL',
            p: 'val',
            u: { pair: true },
            a: '@pairkey',
            g: 'map,pair,key,json',
          },

          // Ignore initial comma: {,a:1.
          { s: '#CA', g: 'map,pair,comma,jsonic' },

          // map.child: bare colon `:value` stores value on child$ property.
          p.cfg.map.child && {
            s: '#CL',
            p: 'val',
            u: { done: true, child: true },
            g: 'map,pair,child,jsonic',
          },
        ],
        { append: true, clear: true },
      )

      // NOTE: JSON pair.bc runs first, then this bc may override value.
      // .bc('@bc')
      .close(
        [
          // End of map, reset implicit depth counter so that
          // a:b:c:1,d:2 -> {a:{b:{c:1}},d:2}
          {
            s: '#CB',
            c: { 'n.pk': { $lte: 0 } },
            b: 1,
            g: 'map,pair,json',
          },

          // Ignore trailing comma at end of map.
          {
            s: '#CA #CB',
            c: { 'n.pk': { $lte: 0 } },
            b: 1,
            g: 'map,pair,comma,jsonic',
          },

          { s: [CA, ZZ], g: 'end,jsonic' },

          // Comma means a new pair at same pair-key level.
          {
            s: '#CA',
            c: { 'n.pk': { $lte: 0 } },
            r: 'pair',
            g: 'map,pair,json',
          },

          // TODO: try CA VAL ? works anywhere?
          // Comma means a new pair if implicit top level map.
          {
            s: '#CA',
            c: { 'n.dmap': { $lte: 1 } },
            r: 'pair',
            g: 'map,pair,jsonic',
          },

          // TODO: try VAL CL ? works anywhere?
          // Value means a new pair if implicit top level map.
          {
            s: '#KEY',
            c: { 'n.dmap': { $lte: 1 } },
            r: 'pair',
            b: 1,
            g: 'map,pair,imp,jsonic',
          },

          // End of implicit path (eg. a:b:1), keep closing until pk=0.
          {
            s: ['#CB #CA #CS #KEY'],
            c: { 'n.pk': { $gt: 0 } },
            b: 1,
            g: 'map,pair,imp,path,jsonic',
          },

          // Can't close a map with `]`
          { s: '#CS', e: (r: Rule) => r.c0, g: 'end,jsonic' },

          // Fail if auto-close option is false.
          { s: '#ZZ', e: '@finish', g: 'map,pair,json' },

          // Who needs commas anyway?
          {
            r: 'pair',
            b: 1,
            g: 'map,pair,imp,jsonic',
          },
        ],
        { append: true, delete: [0, 1] },
      )
  })

  // push onto node
  jsonic.rule('elem', (rs: RuleSpec, p: Parser) => {
    rs
      .fnref({
        ...fnm,
        // Take ownership of the elem close phase: @tabnas/json's strict
        // @elem-bc pushes every child node, so it would double-add jsonic's
        // done-flagged elements (implicit nulls, pair, child). Replace it
        // with the full handler — normal push (done-guarded) plus jsonic's
        // pair/child handling.
        '@elem-bc/replace': (r: Rule, ctx: Context) => {
          if (true !== r.u.done && undefined !== r.child.node) {
            r.node.push(r.child.node)
          }

          if (true === r.u.pair) {
            if (ctx.cfg.list.pair) {
              // list.pair: push pair as object element into the list
              let key = r.u.key
              let val = r.child.node
              val = undefined === val ? null : val
              let pairObj = Object.create(null)
              pairObj[key] = val
              r.node.push(pairObj)
            } else {
              pairval(r, ctx)
            }
          }

          if (true === r.u.child) {
            let val = r.child.node
            val = undefined === val ? null : val
            let prev = r.node['child$']

            if (undefined === prev) {
              r.node['child$'] = val
            } else {
              r.node['child$'] =
                ctx.cfg.map.merge
                  ? ctx.cfg.map.merge(prev, val, r, ctx)
                  : ctx.cfg.map.extend
                    ? deep(prev, val)
                    : val
            }
          }
        }
      })

      .open([
        // Empty commas insert null elements.
        // Note that close consumes a comma, so b:2 works.
        {
          s: '#CA #CA',
          b: 2,
          u: { done: true },
          a: (r: Rule) => r.node.push(null),
          g: 'list,elem,imp,null,jsonic',
        },

        {
          s: '#CA',
          u: { done: true },
          a: (r: Rule) => r.node.push(null),
          g: 'list,elem,imp,null,jsonic',
        },

        {
          s: '#KEY #CL',
          e: (p.cfg.list.property || p.cfg.list.pair) ? undefined :
            (_r: Rule, ctx: Context) => ctx.t0,
          p: 'val',
          n: { pk: 1, dmap: 1 },
          u: { done: true, pair: true, list: true },
          a: '@pairkey',
          g: 'elem,pair,jsonic',
        },

        // list.child: bare colon `:value` stores value on child$ property.
        p.cfg.list.child && {
          s: '#CL',
          p: 'val',
          u: { done: true, child: true, list: true },
          g: 'elem,child,jsonic',
        },
      ])
      // .bc('@bc')
      .close(
        [
          // Ignore trailing comma.
          { s: ['#CA', '#CS #ZZ'], b: 1, g: 'list,elem,comma,jsonic' },

          // Next element.
          { s: '#CA', r: 'elem', g: 'list,elem,json' },

          // End of list.
          { s: '#CS', b: 1, g: 'list,elem,json' },

          // Fail if auto-close option is false.
          { s: '#ZZ', e: '@finish', g: 'list,elem,json' },

          // Can't close a list with `}`
          { s: '#CB', e: (r: Rule) => r.c0, g: 'end,jsonic' },

          // Who needs commas anyway?
          { r: 'elem', b: 1, g: 'list,elem,imp,jsonic' },
        ],
        { delete: [-1, -2] },
      )
  })
}


function makeJSON(jsonic: any) {
  // A fresh instance limited to the json-tagged grammar alts. The
  // grammar is installed automatically by `make()`; the RuleSpec adder
  // filters its alts against `rule.include`, so only the strict-JSON
  // alternatives survive.
  return jsonic.make({
    text: { lex: false },
    number: {
      hex: false,
      oct: false,
      bin: false,
      sep: null,
      exclude: /^00+/,
    },
    string: {
      chars: '"',
      multiChars: '',
      allowUnknown: false,
      escape: { v: null },
    },
    comment: { lex: false },
    map: { extend: false },
    lex: { empty: false },
    rule: { finish: false, include: 'json' },
    result: { fail: [undefined, NaN] },
    tokenSet: {
      KEY: ['#ST', null, null, null],
    },
  })
}


// Register the relaxed-JSON grammar rules (val / map / list / pair /
// elem) on a `tabnas` engine instance. Exposed under an explicit name so
// other grammar plugins can layer their own syntax on top of the jsonic
// core — e.g. a CSV grammar that parses each cell as a jsonic value —
// without re-declaring it. This is the same role `registerJsonGrammar`
// plays for the strict-JSON fixture in the `tabnas` package.
const registerJsonicGrammar = grammar


// The idiomatic `tabnas` grammar plugin. On a bare engine it applies
// jsonic's option defaults (the engine already lexes relaxed JSON; this
// adds the jsonic error/hint branding) and then registers the
// relaxed-JSON grammar:
//
//   import { Tabnas } from '@tabnas/parser'
//   import { jsonic } from '@tabnas/jsonic'
//   const parser = new Tabnas().use(jsonic)
//   parser.parse('a:1,b:[x,y,z]')   // { a: 1, b: ['x','y','z'] }
//
// Registration order matters when another plugin builds on jsonic — use
// jsonic first so the value/map/list rules and tokens it defines are in
// place:  new Tabnas().use(jsonic).use(csv).
//
// The callable `Jsonic` API exported from this package is a legacy
// compatibility wrapper around this same plugin; new code that composes
// grammars should prefer the plugin.
const jsonicPlugin: Plugin = function jsonic(tn: any, _options?: any) {
  tn.options(defaults)
  registerJsonicGrammar(tn as unknown as Jsonic)
}


export { grammar, makeJSON, registerJsonicGrammar, jsonicPlugin }
