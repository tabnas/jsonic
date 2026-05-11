/* Copyright (c) 2013-2026 Richard Rodger, MIT License */

/*  jsonic.ts
 *  Entry point and API.
 *
 *  jsonic = the `tabnas` parsing engine + the relaxed-JSON grammar +
 *  the BNF converter, wrapped in the historic `Jsonic` API: a parse
 *  function with the management methods attached as properties.
 *
 *  The lexer, parser, rule machinery, errors and utilities all live in
 *  the `tabnas` package now. This module no longer re-implements any of
 *  that — it just constructs `tabnas` engine instances and dresses them
 *  up in the callable-with-properties shape jsonic users expect.
 */

import {
  Tabnas,
  TabnasError as JsonicError,
  util,
  S,
  OPEN,
  CLOSE,
  BEFORE,
  AFTER,
  EMPTY,
  SKIP,
  makeLex,
  makeParser,
  makeToken,
  makePoint,
  makeRule,
  makeRuleSpec,
  makeFixedMatcher,
  makeSpaceMatcher,
  makeLineMatcher,
  makeStringMatcher,
  makeCommentMatcher,
  makeNumberMatcher,
  makeTextMatcher,
} from 'tabnas'

import { deep, assign, defprop, filterRules, parserwrap } from 'tabnas/utility'

import type {
  AltAction,
  AltCond,
  AltError,
  AltMatch,
  AltModifier,
  AltSpec,
  Bag,
  BnfConvertOptions,
  Config,
  Context,
  Counters,
  FuncRef,
  GrammarSetting,
  GrammarSpec,
  JsonicAPI,
  JsonicParse,
  Lex,
  LexCheck,
  LexMatcher,
  MakeLexMatcher,
  NormAltSpec,
  Options,
  Parser,
  Plugin,
  Point,
  Rule,
  RuleDefiner,
  RuleSpec,
  RuleSpecMap,
  RuleState,
  StateAction,
  Tin,
  Token,
} from './types'

import { defaults } from './defaults'

import { grammar, makeJSON } from './grammar'

import { bnf as bnfConvert } from './bnf'


// The full library type.
// NOTE: redeclared here (rather than imported) so the same name can be
// exported as both a type and the root instance value.
type Jsonic = JsonicParse & // A function that parses.
  JsonicAPI & { [prop: string]: any } // Plus the API methods. // Extensible by plugin decoration.


// Create a Jsonic instance.
//
//   make('jsonic')          -> the immutable root (restricted surface)
//   make('json')            -> the strict-JSON variant
//   make(options?, parent?) -> a fresh, customisable instance
//
// Each instance wraps a `tabnas` engine instance. The engine does the
// real work; this function just builds the callable + property surface
// and (re-)installs the relaxed-JSON grammar.
function make(param_options?: Bag | string, parent?: Jsonic): Jsonic {
  let injectFullAPI = true
  if ('jsonic' === param_options) {
    injectFullAPI = false
  } else if ('json' === param_options) {
    return makeJSON(root)
  }

  param_options = 'string' === typeof param_options ? {} : param_options

  // The underlying engine instance.
  let tabnas: Tabnas
  if (parent) {
    // A child engine: inherits the parent's merged options + config,
    // but starts with an empty rule set. The grammar and the parent's
    // plugins are re-applied below — against the *wrapper* — so that
    // plugin decorations land where the historic API expects them.
    const parentInst = parent.internal().tabnas as Tabnas
    tabnas = parentInst.make((param_options as Bag) || {})
  } else {
    const opts = deep(
      {},
      false === (param_options as Bag)?.defaults$ ? {} : defaults,
      (param_options as Bag) || {},
    )
    tabnas = new Tabnas(opts as any)
  }

  // Internal state record — mirrors the historic shape, but the live
  // values come straight off the engine instance (it swaps its parser
  // out on every options() change, hence the getters).
  const internal: any = {
    get parser() {
      return tabnas.internal().parser
    },
    get config() {
      return tabnas.internal().config
    },
    plugins: [] as Plugin[],
    get sub() {
      return tabnas.internal().sub
    },
    get mark() {
      return tabnas.internal().mark
    },
    get merged() {
      return tabnas.internal().merged
    },
    // Back-reference for child creation.
    tabnas,
  }

  // The primary parsing function. Drives the engine's parser directly
  // (rather than via tabnas.parse) so the *wrapper* — not the bare
  // engine instance — is what rule actions and plugins see as
  // `ctx.inst()`.
  const jsonic: any = function Jsonic(
    src: any,
    meta?: any,
    parent_ctx?: any,
  ): any {
    if (S.string === typeof src) {
      const opts_parser = (tabnas.options as any).parser
      const parser =
        opts_parser?.start ? parserwrap(opts_parser) : tabnas.internal().parser
      return parser.start(src, jsonic, meta, parent_ctx)
    }
    return src
  }

  // The API surface. Most members forward to the engine instance; for
  // methods that conventionally return "this", we return the wrapper so
  // chaining and `jsonic.use(p)('src')` keep working. `bnf` is jsonic's
  // own addition.
  const api: JsonicAPI = {
    parse: jsonic,

    token: tabnas.token as any,
    tokenSet: tabnas.tokenSet as any,
    fixed: tabnas.fixed as any,

    // Dual-shape callable: set via call, read via property. Sharing the
    // engine's own `options` object means option changes reconfigure
    // the engine (and swap its parser) as before.
    options: tabnas.options as any,

    config: () => tabnas.config(),

    // TODO: how to handle null plugin?
    use: function use(plugin: Plugin, plugin_options?: Bag): Jsonic {
      if (S.function !== typeof plugin) {
        throw new Error(
          'Jsonic.use: the first argument must be a function ' +
          'defining a plugin. See https://jsonic.senecajs.org/plugin',
        )
      }

      // Plugin name keys in options.plugin are the lower-cased plugin
      // function name.
      const plugin_name = plugin.name.toLowerCase()
      const full_plugin_options = deep(
        {},
        plugin.defaults || {},
        plugin_options || {},
      )

      ji.options({
        plugin: {
          [plugin_name]: full_plugin_options,
        },
      })
      const merged_plugin_options = ji.options.plugin[plugin_name]
      internal.plugins.push(plugin)
      plugin.options = merged_plugin_options

      return plugin(ji, merged_plugin_options) || ji
    },

    rule: (name?: string, define?: RuleDefiner | null) => {
      const r = tabnas.rule(name, define)
      return (r === tabnas ? jsonic : r) as any
    },

    make: (options?: Options | string) => make(options as any, jsonic),

    empty: (options?: Options) =>
      make({
        defaults$: false,
        standard$: false,
        grammar$: false,
        ...((options as Bag) || {}),
      }),

    id:
      'Jsonic/' +
      Date.now() +
      '/' +
      ('' + Math.random()).substring(2, 8).padEnd(6, '0') +
      (null == (tabnas.options as any).tag
        ? ''
        : '/' + (tabnas.options as any).tag),

    toString: () => api.id,

    sub: (spec: { lex?: any; rule?: any }) => {
      tabnas.sub(spec)
      return jsonic
    },

    util,

    grammar: (gs: GrammarSpec | string, setting?: GrammarSetting) => {
      if ('string' === typeof gs) {
        const parsed = make()(gs)
        if (null == parsed || 'object' !== typeof parsed) {
          return jsonic
        }
        gs = parsed as GrammarSpec
      }
      tabnas.grammar(gs as GrammarSpec, setting)
      return jsonic
    },

    // Convert a BNF grammar string into a jsonic GrammarSpec and install
    // it on this instance. Returns the generated spec so callers can
    // inspect, serialise or diff it. Use `bnf.toSpec(src, opts)` to
    // build the spec without installing it.
    bnf: (() => {
      const impl = (src: string, opts?: BnfConvertOptions) => {
        const spec = bnfConvert(src, opts)
        ;(ji as any).grammar(spec)
        return spec
      }
      impl.toSpec = (src: string, opts?: BnfConvertOptions) =>
        bnfConvert(src, opts)
      return impl
    })(),
  } as JsonicAPI

  // `api.make` reports its name as 'make' even though the enclosing
  // function is also named `make`.
  defprop(api.make, S.name, { value: S.make })

  // Assemble the public surface.
  if (injectFullAPI) {
    assign(jsonic, api)
  } else {
    // The immutable root exposes only a parse-and-fork surface
    // directly. NOTE: api.test.js pins this exact key set (plus the
    // deconstruction names added below).
    assign(jsonic, {
      empty: api.empty,
      parse: api.parse,
      sub: api.sub,
      id: api.id,
      toString: api.toString,
    })
  }

  // Hide internals where you can still find them.
  defprop(jsonic, 'internal', { value: () => internal })

  // The object the relaxed-JSON grammar (and re-run plugins) decorate.
  // For a full instance it is the instance itself; for the immutable
  // root it is a throwaway full-API view sharing the same engine.
  const ji: any = injectFullAPI
    ? jsonic
    : assign(Object.create(jsonic), api)

  if (parent) {
    // Transfer extra parent properties (preserves plugin decorations).
    for (const k in parent) {
      if (undefined === jsonic[k]) {
        jsonic[k] = (parent as any)[k]
      }
    }
    jsonic.parent = parent
  }

  // Install the relaxed-JSON grammar (unless suppressed).
  if (false !== (tabnas.options as any).grammar$) {
    grammar(ji)
  }

  // Re-run inherited plugins on the (now grammar-bearing) child, in
  // registration order, so option-conditional grammar alts get
  // re-evaluated against this instance's options.
  if (parent) {
    const inherited: Plugin[] = parent.internal().plugins
    for (const plugin of inherited) {
      ;(ji as any).use(plugin)
    }
  }

  // Apply rule.include / rule.exclude once the grammar and all plugins
  // have contributed their alts. The engine's RuleSpec adder no longer
  // filters in place (it returns a fresh spec instead), so the strict
  // 'json' variant — and any `make({ rule: { include/exclude } })` —
  // needs this final pass. Mirrors Tabnas#make's own filtering step.
  const cfg = tabnas.internal().config
  if (0 < cfg.rule.include.length || 0 < cfg.rule.exclude.length) {
    const tparser: any = tabnas.internal().parser
    const rsm = tparser.rule()
    const filtered: any = {}
    for (const rn of Object.keys(rsm)) {
      filtered[rn] = filterRules(rsm[rn], cfg)
    }
    tparser.rsm = filtered
    tparser.norm()
  }

  return jsonic
}


let root: any = undefined

// The global root Jsonic instance — its parsing rules cannot be
// modified. Use Jsonic.make() to create a modifiable instance.
let Jsonic: Jsonic = (root = make('jsonic'))

// Provide deconstruction export names.
root.Jsonic = root
root.JsonicError = JsonicError
root.makeLex = makeLex
root.makeParser = makeParser
root.makeToken = makeToken
root.makePoint = makePoint
root.makeRule = makeRule
root.makeRuleSpec = makeRuleSpec
root.makeFixedMatcher = makeFixedMatcher
root.makeSpaceMatcher = makeSpaceMatcher
root.makeLineMatcher = makeLineMatcher
root.makeStringMatcher = makeStringMatcher
root.makeCommentMatcher = makeCommentMatcher
root.makeNumberMatcher = makeNumberMatcher
root.makeTextMatcher = makeTextMatcher
root.OPEN = OPEN
root.CLOSE = CLOSE
root.BEFORE = BEFORE
root.AFTER = AFTER
root.EMPTY = EMPTY
root.SKIP = SKIP

root.util = util
root.make = make
root.S = S


// Export most of the engine types for use by plugins (re-exported from
// `tabnas` via ./types).
export type {
  AltAction,
  AltCond,
  AltError,
  AltMatch,
  AltModifier,
  AltSpec,
  Bag,
  BnfConvertOptions,
  Config,
  Context,
  Counters,
  FuncRef,
  GrammarSetting,
  GrammarSpec,
  Lex,
  LexCheck,
  LexMatcher,
  MakeLexMatcher,
  NormAltSpec,
  Options,
  Parser,
  Plugin,
  Point,
  Rule,
  RuleDefiner,
  RuleSpec,
  RuleSpecMap,
  RuleState,
  StateAction,
  Tin,
  Token,
}

export {
  // Jsonic is both a type and a value.
  Jsonic as Jsonic,
  JsonicError,
  util,
  make,
  makeToken,
  makePoint,
  makeRule,
  makeRuleSpec,
  makeLex,
  makeParser,
  makeFixedMatcher,
  makeSpaceMatcher,
  makeLineMatcher,
  makeStringMatcher,
  makeCommentMatcher,
  makeNumberMatcher,
  makeTextMatcher,
  OPEN,
  CLOSE,
  BEFORE,
  AFTER,
  EMPTY,
  SKIP,
  S,
  root,
}

export default Jsonic

if ('undefined' !== typeof module) {
  module.exports = Jsonic
}
