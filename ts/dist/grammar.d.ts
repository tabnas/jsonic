import type { Plugin } from 'tabnas';
import { Jsonic } from './jsonic';
declare function grammar(jsonic: Jsonic): void;
declare function makeJSON(jsonic: any): any;
declare const registerJsonicGrammar: typeof grammar;
declare const jsonicPlugin: Plugin;
export { grammar, makeJSON, registerJsonicGrammar, jsonicPlugin };
