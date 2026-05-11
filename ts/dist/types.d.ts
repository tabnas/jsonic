import type { AltAction, AltCond, AltError, AltMatch, AltModifier, AltSpec, Config, Context, Counters, FuncRef, GrammarSetting, GrammarSpec, Lex, LexCheck, LexMatcher, LexSub, MakeLexMatcher, NormAltSpec, Parser, Point, Rule, RuleDefiner, RuleSpec, RuleSpecMap, RuleState, RuleSub, StateAction, TabnasOptions, Tin, Token } from 'tabnas';
export type { AltAction, AltCond, AltError, AltMatch, AltModifier, AltSpec, Config, Context, Counters, FuncRef, GrammarSetting, GrammarSpec, Lex, LexCheck, LexMatcher, LexSub, MakeLexMatcher, NormAltSpec, Parser, Point, Rule, RuleDefiner, RuleSpec, RuleSpecMap, RuleState, RuleSub, StateAction, Tin, Token, };
export { OPEN, CLOSE, BEFORE, AFTER, EMPTY, SKIP } from 'tabnas';
export type Bag = {
    [key: string]: any;
};
export type Options = TabnasOptions;
export type JsonicParse = (src: any, meta?: any, parent_ctx?: any) => any;
export type BnfConvertOptions = {
    start?: string;
    tag?: string;
};
export type Plugin = ((jsonic: Jsonic, plugin_options?: any) => void | Jsonic) & {
    defaults?: Bag;
    options?: Bag;
};
export interface JsonicAPI {
    parse: JsonicParse;
    options: Options & ((change_options?: Bag | string) => Bag);
    config: () => Config;
    make: (options?: Options | string) => Jsonic;
    use: (plugin: Plugin, plugin_options?: Bag) => Jsonic;
    rule: (name?: string, define?: RuleDefiner | null) => Jsonic | RuleSpec | RuleSpecMap;
    empty: (options?: Options) => Jsonic;
    token: ((ref: string | Tin) => any) & {
        [k: string]: any;
    };
    tokenSet: ((ref: string | Tin) => any) & {
        [k: string]: any;
    };
    fixed: ((ref: string | Tin) => any) & {
        [k: string]: any;
    };
    id: string;
    toString: () => string;
    sub: (spec: {
        lex?: LexSub;
        rule?: RuleSub;
    }) => Jsonic;
    util: Bag;
    internal: () => any;
    grammar: (gs: GrammarSpec | string, setting?: GrammarSetting) => Jsonic;
    bnf: ((src: string, opts?: BnfConvertOptions) => GrammarSpec) & {
        toSpec: (src: string, opts?: BnfConvertOptions) => GrammarSpec;
    };
}
export type Jsonic = JsonicParse & JsonicAPI & {
    [prop: string]: any;
};
