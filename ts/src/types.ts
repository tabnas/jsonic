/* Copyright (c) 2021-2026 Richard Rodger, MIT License */

/*  types.ts
 *  Jsonic-specific type definitions.
 *
 *  The parsing engine — lexer, parser, rule machinery, errors and
 *  utilities — now lives in the `tabnas` package. Its types are
 *  re-exported here so that existing imports of `./types` (and, via
 *  `jsonic.ts`, of `./jsonic`) keep working unchanged.
 */

import type {
  AltAction,
  AltCond,
  AltError,
  AltMatch,
  AltModifier,
  AltSpec,
  Config,
  Context,
  Counters,
  FuncRef,
  GrammarSetting,
  GrammarSpec,
  Lex,
  LexCheck,
  LexMatcher,
  LexSub,
  MakeLexMatcher,
  NormAltSpec,
  Parser,
  Point,
  Rule,
  RuleDefiner,
  RuleSpec,
  RuleSpecMap,
  RuleState,
  RuleSub,
  StateAction,
  TabnasOptions,
  Tin,
  Token,
} from 'tabnas'

// Re-export the engine types verbatim. (NOTE: `Plugin` is *not*
// re-exported — jsonic plugins receive the callable `Jsonic` instance,
// not the bare engine class, so jsonic defines its own `Plugin` below.)
export type {
  AltAction,
  AltCond,
  AltError,
  AltMatch,
  AltModifier,
  AltSpec,
  Config,
  Context,
  Counters,
  FuncRef,
  GrammarSetting,
  GrammarSpec,
  Lex,
  LexCheck,
  LexMatcher,
  LexSub,
  MakeLexMatcher,
  NormAltSpec,
  Parser,
  Point,
  Rule,
  RuleDefiner,
  RuleSpec,
  RuleSpecMap,
  RuleState,
  RuleSub,
  StateAction,
  Tin,
  Token,
}

// Re-export the engine constants.
export { OPEN, CLOSE, BEFORE, AFTER, EMPTY, SKIP } from 'tabnas'


// Jsonic shorthand for an open-ended object.
export type Bag = { [key: string]: any }

// Jsonic's parsing options are the engine's parsing options.
export type Options = TabnasOptions

// The main top-level parse function.
export type JsonicParse = (src: any, meta?: any, parent_ctx?: any) => any

// BNF -> grammar converter options. See src/bnf.ts.
export type BnfConvertOptions = {
  start?: string
  tag?: string
}

// Define a plugin to extend the provided Jsonic instance. Unlike the
// engine's own plugin type, the first argument is the callable `Jsonic`
// instance (so plugins can both parse with it and decorate it).
export type Plugin = ((
  jsonic: Jsonic,
  plugin_options?: any,
) => void | Jsonic) & {
  defaults?: Bag
  options?: Bag // TODO: InstalledPlugin.options is always defined ?
}


// The Jsonic API: management methods attached to the main parse
// function. Most of these forward to the underlying `tabnas` engine
// instance; `bnf` is jsonic's own addition.
export interface JsonicAPI {
  // Explicit parse method.
  parse: JsonicParse

  // Get and set partial option trees. Accepts a Bag object or a
  // jsonic-format string that is parsed into a Bag before applying.
  options: Options & ((change_options?: Bag | string) => Bag)

  // Get the current resolved configuration (derived from options).
  config: () => Config

  // Create a new (customisable) Jsonic instance.
  make: (options?: Options | string) => Jsonic

  // Apply a plugin.
  use: (plugin: Plugin, plugin_options?: Bag) => Jsonic

  // Get, get-by-name, or define parser rules.
  rule: (
    name?: string,
    define?: RuleDefiner | null,
  ) => Jsonic | RuleSpec | RuleSpecMap

  // Create a bare instance (no defaults, no standard tokens, no grammar).
  empty: (options?: Options) => Jsonic

  // Token lookup-or-create, by name or Tin.
  token: ((ref: string | Tin) => any) & { [k: string]: any }

  // Token-set lookup, by name or Tin.
  tokenSet: ((ref: string | Tin) => any) & { [k: string]: any }

  // Fixed-token source <-> Tin lookup.
  fixed: ((ref: string | Tin) => any) & { [k: string]: any }

  // Unique identifier string for each Jsonic instance.
  id: string

  // Identifier for string conversion.
  toString: () => string

  // Subscribe to lexing and parsing events.
  sub: (spec: { lex?: LexSub; rule?: RuleSub }) => Jsonic

  // Utility bag (shared with the engine's `util`).
  util: Bag

  // Internal-state accessor.
  internal: () => any

  // Apply a declarative GrammarSpec (or a jsonic-format string).
  grammar: (gs: GrammarSpec | string, setting?: GrammarSetting) => Jsonic

  // Convert a BNF grammar string into a jsonic GrammarSpec and install
  // it on this instance. Returns the generated spec. See src/bnf.ts.
  // `bnf.toSpec(src, opts)` returns the spec without installing.
  bnf: ((src: string, opts?: BnfConvertOptions) => GrammarSpec) & {
    toSpec: (src: string, opts?: BnfConvertOptions) => GrammarSpec
  }
}


// The full library type: a parse function with the API methods
// attached, extensible by plugin decoration.
export type Jsonic = JsonicParse &
  JsonicAPI & { [prop: string]: any }
