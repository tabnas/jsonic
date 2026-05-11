"use strict";
/* Copyright (c) 2013-2026 Richard Rodger, MIT License */
Object.defineProperty(exports, "__esModule", { value: true });
exports.root = exports.S = exports.SKIP = exports.EMPTY = exports.AFTER = exports.BEFORE = exports.CLOSE = exports.OPEN = exports.makeTextMatcher = exports.makeNumberMatcher = exports.makeCommentMatcher = exports.makeStringMatcher = exports.makeLineMatcher = exports.makeSpaceMatcher = exports.makeFixedMatcher = exports.makeParser = exports.makeLex = exports.makeRuleSpec = exports.makeRule = exports.makePoint = exports.makeToken = exports.util = exports.JsonicError = exports.Jsonic = void 0;
exports.make = make;
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
const tabnas_1 = require("tabnas");
Object.defineProperty(exports, "JsonicError", { enumerable: true, get: function () { return tabnas_1.TabnasError; } });
Object.defineProperty(exports, "util", { enumerable: true, get: function () { return tabnas_1.util; } });
Object.defineProperty(exports, "S", { enumerable: true, get: function () { return tabnas_1.S; } });
Object.defineProperty(exports, "OPEN", { enumerable: true, get: function () { return tabnas_1.OPEN; } });
Object.defineProperty(exports, "CLOSE", { enumerable: true, get: function () { return tabnas_1.CLOSE; } });
Object.defineProperty(exports, "BEFORE", { enumerable: true, get: function () { return tabnas_1.BEFORE; } });
Object.defineProperty(exports, "AFTER", { enumerable: true, get: function () { return tabnas_1.AFTER; } });
Object.defineProperty(exports, "EMPTY", { enumerable: true, get: function () { return tabnas_1.EMPTY; } });
Object.defineProperty(exports, "SKIP", { enumerable: true, get: function () { return tabnas_1.SKIP; } });
Object.defineProperty(exports, "makeLex", { enumerable: true, get: function () { return tabnas_1.makeLex; } });
Object.defineProperty(exports, "makeParser", { enumerable: true, get: function () { return tabnas_1.makeParser; } });
Object.defineProperty(exports, "makeToken", { enumerable: true, get: function () { return tabnas_1.makeToken; } });
Object.defineProperty(exports, "makePoint", { enumerable: true, get: function () { return tabnas_1.makePoint; } });
Object.defineProperty(exports, "makeRule", { enumerable: true, get: function () { return tabnas_1.makeRule; } });
Object.defineProperty(exports, "makeRuleSpec", { enumerable: true, get: function () { return tabnas_1.makeRuleSpec; } });
Object.defineProperty(exports, "makeFixedMatcher", { enumerable: true, get: function () { return tabnas_1.makeFixedMatcher; } });
Object.defineProperty(exports, "makeSpaceMatcher", { enumerable: true, get: function () { return tabnas_1.makeSpaceMatcher; } });
Object.defineProperty(exports, "makeLineMatcher", { enumerable: true, get: function () { return tabnas_1.makeLineMatcher; } });
Object.defineProperty(exports, "makeStringMatcher", { enumerable: true, get: function () { return tabnas_1.makeStringMatcher; } });
Object.defineProperty(exports, "makeCommentMatcher", { enumerable: true, get: function () { return tabnas_1.makeCommentMatcher; } });
Object.defineProperty(exports, "makeNumberMatcher", { enumerable: true, get: function () { return tabnas_1.makeNumberMatcher; } });
Object.defineProperty(exports, "makeTextMatcher", { enumerable: true, get: function () { return tabnas_1.makeTextMatcher; } });
const utility_1 = require("tabnas/utility");
const defaults_1 = require("./defaults");
const grammar_1 = require("./grammar");
const bnf_1 = require("./bnf");
// Create a Jsonic instance.
//
//   make('jsonic')          -> the immutable root (restricted surface)
//   make('json')            -> the strict-JSON variant
//   make(options?, parent?) -> a fresh, customisable instance
//
// Each instance wraps a `tabnas` engine instance. The engine does the
// real work; this function just builds the callable + property surface
// and (re-)installs the relaxed-JSON grammar.
function make(param_options, parent) {
    let injectFullAPI = true;
    if ('jsonic' === param_options) {
        injectFullAPI = false;
    }
    else if ('json' === param_options) {
        return (0, grammar_1.makeJSON)(root);
    }
    param_options = 'string' === typeof param_options ? {} : param_options;
    // The underlying engine instance.
    let tabnas;
    if (parent) {
        // A child engine: inherits the parent's merged options + config,
        // but starts with an empty rule set. The grammar and the parent's
        // plugins are re-applied below — against the *wrapper* — so that
        // plugin decorations land where the historic API expects them.
        const parentInst = parent.internal().tabnas;
        tabnas = parentInst.make(param_options || {});
    }
    else {
        const opts = (0, utility_1.deep)({}, false === param_options?.defaults$ ? {} : defaults_1.defaults, param_options || {});
        tabnas = new tabnas_1.Tabnas(opts);
    }
    // Internal state record — mirrors the historic shape, but the live
    // values come straight off the engine instance (it swaps its parser
    // out on every options() change, hence the getters).
    const internal = {
        get parser() {
            return tabnas.internal().parser;
        },
        get config() {
            return tabnas.internal().config;
        },
        plugins: [],
        get sub() {
            return tabnas.internal().sub;
        },
        get mark() {
            return tabnas.internal().mark;
        },
        get merged() {
            return tabnas.internal().merged;
        },
        // Back-reference for child creation.
        tabnas,
    };
    // The primary parsing function. Drives the engine's parser directly
    // (rather than via tabnas.parse) so the *wrapper* — not the bare
    // engine instance — is what rule actions and plugins see as
    // `ctx.inst()`.
    const jsonic = function Jsonic(src, meta, parent_ctx) {
        if (tabnas_1.S.string === typeof src) {
            const opts_parser = tabnas.options.parser;
            const parser = opts_parser?.start ? (0, utility_1.parserwrap)(opts_parser) : tabnas.internal().parser;
            return parser.start(src, jsonic, meta, parent_ctx);
        }
        return src;
    };
    // The API surface. Most members forward to the engine instance; for
    // methods that conventionally return "this", we return the wrapper so
    // chaining and `jsonic.use(p)('src')` keep working. `bnf` is jsonic's
    // own addition.
    const api = {
        parse: jsonic,
        token: tabnas.token,
        tokenSet: tabnas.tokenSet,
        fixed: tabnas.fixed,
        // Dual-shape callable: set via call, read via property. Sharing the
        // engine's own `options` object means option changes reconfigure
        // the engine (and swap its parser) as before.
        options: tabnas.options,
        config: () => tabnas.config(),
        // TODO: how to handle null plugin?
        use: function use(plugin, plugin_options) {
            if (tabnas_1.S.function !== typeof plugin) {
                throw new Error('Jsonic.use: the first argument must be a function ' +
                    'defining a plugin. See https://jsonic.senecajs.org/plugin');
            }
            // Plugin name keys in options.plugin are the lower-cased plugin
            // function name.
            const plugin_name = plugin.name.toLowerCase();
            const full_plugin_options = (0, utility_1.deep)({}, plugin.defaults || {}, plugin_options || {});
            ji.options({
                plugin: {
                    [plugin_name]: full_plugin_options,
                },
            });
            const merged_plugin_options = ji.options.plugin[plugin_name];
            internal.plugins.push(plugin);
            plugin.options = merged_plugin_options;
            return plugin(ji, merged_plugin_options) || ji;
        },
        rule: (name, define) => {
            const r = tabnas.rule(name, define);
            return (r === tabnas ? jsonic : r);
        },
        make: (options) => make(options, jsonic),
        empty: (options) => make({
            defaults$: false,
            standard$: false,
            grammar$: false,
            ...(options || {}),
        }),
        id: 'Jsonic/' +
            Date.now() +
            '/' +
            ('' + Math.random()).substring(2, 8).padEnd(6, '0') +
            (null == tabnas.options.tag
                ? ''
                : '/' + tabnas.options.tag),
        toString: () => api.id,
        sub: (spec) => {
            tabnas.sub(spec);
            return jsonic;
        },
        util: tabnas_1.util,
        grammar: (gs, setting) => {
            if ('string' === typeof gs) {
                const parsed = make()(gs);
                if (null == parsed || 'object' !== typeof parsed) {
                    return jsonic;
                }
                gs = parsed;
            }
            tabnas.grammar(gs, setting);
            return jsonic;
        },
        // Convert a BNF grammar string into a jsonic GrammarSpec and install
        // it on this instance. Returns the generated spec so callers can
        // inspect, serialise or diff it. Use `bnf.toSpec(src, opts)` to
        // build the spec without installing it.
        bnf: (() => {
            const impl = (src, opts) => {
                const spec = (0, bnf_1.bnf)(src, opts);
                ji.grammar(spec);
                return spec;
            };
            impl.toSpec = (src, opts) => (0, bnf_1.bnf)(src, opts);
            return impl;
        })(),
    };
    // `api.make` reports its name as 'make' even though the enclosing
    // function is also named `make`.
    (0, utility_1.defprop)(api.make, tabnas_1.S.name, { value: tabnas_1.S.make });
    // Assemble the public surface.
    if (injectFullAPI) {
        (0, utility_1.assign)(jsonic, api);
    }
    else {
        // The immutable root exposes only a parse-and-fork surface
        // directly. NOTE: api.test.js pins this exact key set (plus the
        // deconstruction names added below).
        (0, utility_1.assign)(jsonic, {
            empty: api.empty,
            parse: api.parse,
            sub: api.sub,
            id: api.id,
            toString: api.toString,
        });
    }
    // Hide internals where you can still find them.
    (0, utility_1.defprop)(jsonic, 'internal', { value: () => internal });
    // The object the relaxed-JSON grammar (and re-run plugins) decorate.
    // For a full instance it is the instance itself; for the immutable
    // root it is a throwaway full-API view sharing the same engine.
    const ji = injectFullAPI
        ? jsonic
        : (0, utility_1.assign)(Object.create(jsonic), api);
    if (parent) {
        // Transfer extra parent properties (preserves plugin decorations).
        for (const k in parent) {
            if (undefined === jsonic[k]) {
                jsonic[k] = parent[k];
            }
        }
        jsonic.parent = parent;
    }
    // Install the relaxed-JSON grammar (unless suppressed).
    if (false !== tabnas.options.grammar$) {
        (0, grammar_1.grammar)(ji);
    }
    // Re-run inherited plugins on the (now grammar-bearing) child, in
    // registration order, so option-conditional grammar alts get
    // re-evaluated against this instance's options.
    if (parent) {
        const inherited = parent.internal().plugins;
        for (const plugin of inherited) {
            ;
            ji.use(plugin);
        }
    }
    // Apply rule.include / rule.exclude once the grammar and all plugins
    // have contributed their alts. The engine's RuleSpec adder no longer
    // filters in place (it returns a fresh spec instead), so the strict
    // 'json' variant — and any `make({ rule: { include/exclude } })` —
    // needs this final pass. Mirrors Tabnas#make's own filtering step.
    const cfg = tabnas.internal().config;
    if (0 < cfg.rule.include.length || 0 < cfg.rule.exclude.length) {
        const tparser = tabnas.internal().parser;
        const rsm = tparser.rule();
        const filtered = {};
        for (const rn of Object.keys(rsm)) {
            filtered[rn] = (0, utility_1.filterRules)(rsm[rn], cfg);
        }
        tparser.rsm = filtered;
        tparser.norm();
    }
    return jsonic;
}
let root = undefined;
exports.root = root;
// The global root Jsonic instance — its parsing rules cannot be
// modified. Use Jsonic.make() to create a modifiable instance.
let Jsonic = (exports.root = root = make('jsonic'));
exports.Jsonic = Jsonic;
// Provide deconstruction export names.
root.Jsonic = root;
root.JsonicError = tabnas_1.TabnasError;
root.makeLex = tabnas_1.makeLex;
root.makeParser = tabnas_1.makeParser;
root.makeToken = tabnas_1.makeToken;
root.makePoint = tabnas_1.makePoint;
root.makeRule = tabnas_1.makeRule;
root.makeRuleSpec = tabnas_1.makeRuleSpec;
root.makeFixedMatcher = tabnas_1.makeFixedMatcher;
root.makeSpaceMatcher = tabnas_1.makeSpaceMatcher;
root.makeLineMatcher = tabnas_1.makeLineMatcher;
root.makeStringMatcher = tabnas_1.makeStringMatcher;
root.makeCommentMatcher = tabnas_1.makeCommentMatcher;
root.makeNumberMatcher = tabnas_1.makeNumberMatcher;
root.makeTextMatcher = tabnas_1.makeTextMatcher;
root.OPEN = tabnas_1.OPEN;
root.CLOSE = tabnas_1.CLOSE;
root.BEFORE = tabnas_1.BEFORE;
root.AFTER = tabnas_1.AFTER;
root.EMPTY = tabnas_1.EMPTY;
root.SKIP = tabnas_1.SKIP;
root.util = tabnas_1.util;
root.make = make;
root.S = tabnas_1.S;
exports.default = Jsonic;
if ('undefined' !== typeof module) {
    module.exports = Jsonic;
}
//# sourceMappingURL=jsonic.js.map