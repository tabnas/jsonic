import { TabnasError as JsonicError, util, S, OPEN, CLOSE, BEFORE, AFTER, EMPTY, SKIP, makeLex, makeParser, makeToken, makePoint, makeRule, makeRuleSpec, makeFixedMatcher, makeSpaceMatcher, makeLineMatcher, makeStringMatcher, makeCommentMatcher, makeNumberMatcher, makeTextMatcher } from 'tabnas';
import type { AltAction, AltCond, AltError, AltMatch, AltModifier, AltSpec, Bag, BnfConvertOptions, Config, Context, Counters, FuncRef, GrammarSetting, GrammarSpec, JsonicAPI, JsonicParse, Lex, LexCheck, LexMatcher, MakeLexMatcher, NormAltSpec, Options, Parser, Plugin, Point, Rule, RuleDefiner, RuleSpec, RuleSpecMap, RuleState, StateAction, Tin, Token } from './types';
type Jsonic = JsonicParse & // A function that parses.
JsonicAPI & {
    [prop: string]: any;
};
declare function make(param_options?: Bag | string, parent?: Jsonic): Jsonic;
declare let root: any;
declare let Jsonic: Jsonic;
export type { AltAction, AltCond, AltError, AltMatch, AltModifier, AltSpec, Bag, BnfConvertOptions, Config, Context, Counters, FuncRef, GrammarSetting, GrammarSpec, Lex, LexCheck, LexMatcher, MakeLexMatcher, NormAltSpec, Options, Parser, Plugin, Point, Rule, RuleDefiner, RuleSpec, RuleSpecMap, RuleState, StateAction, Tin, Token, };
export { Jsonic as Jsonic, JsonicError, util, make, makeToken, makePoint, makeRule, makeRuleSpec, makeLex, makeParser, makeFixedMatcher, makeSpaceMatcher, makeLineMatcher, makeStringMatcher, makeCommentMatcher, makeNumberMatcher, makeTextMatcher, OPEN, CLOSE, BEFORE, AFTER, EMPTY, SKIP, S, root, };
export default Jsonic;
