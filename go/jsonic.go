// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

import (
	"fmt"
	"regexp"
	"sync"

	tjson "github.com/tabnas/json/go"
	tabnas "github.com/tabnas/parser/go"
)

// VERSION is this module's version. It MUST equal ts/package.json
// "version": the release orchestrator rewrites both, and
// TestVersionMatchesPackageJSON fails the build if they drift.
const VERSION = "0.6.4"

// grammarMark is the decoration key that records whether the relaxed-JSON
// grammar has already been installed on an instance. It guards the Grammar
// plugin so that re-running it (which the engine does on every SetOptions)
// does not rebuild the base rules and clobber caller rule modifications,
// while still letting Derive build the grammar fresh on a child (Derive
// copies decorations only after re-running plugins).
const grammarMark = "jsonic$grammar"

// jsonicOptions are the option overrides the grammar plugin layers on top
// of the engine's already-relaxed lexer defaults: the jsonic error
// identity, the `unprintable` string-error alignment matcher (see
// unprintable.go), and the comment markers held in *option* space.
// Error message templates and the rest of the relaxed lexer config are
// shared with the engine defaults.
//
// The comment defs mirror TS defaults.ts. The engine also lexes these
// markers by default, but only from config-space; declaring them as
// options (as TS does) makes them part of the option merge, so a caller
// adding a comment def extends this set instead of replacing it.
func jsonicOptions() Options {
	return Options{
		ErrMsg: &ErrMsgOptions{
			Name: "jsonic",
			Link: "https://github.com/tabnas/jsonic",
		},
		Lex: &LexOptions{
			Match: unprintableMatchSpecs(),
		},
		Comment: &CommentOptions{
			Def: commentDefDefaults(),
		},
	}
}

// commentDefDefaults are jsonic's default comment markers, mirroring the
// TS defaults.ts comment.def entries (hash / slash / multi).
func commentDefDefaults() map[string]*CommentDef {
	t, f := true, false
	return map[string]*CommentDef{
		"hash":  {Line: true, Start: "#", Lex: &t, EatLine: &f},
		"slash": {Line: true, Start: "//", Lex: &t, EatLine: &f},
		"multi": {Line: false, Start: "/*", End: "*/", Lex: &t, EatLine: &f},
	}
}

// normalizeCommentDefs aligns caller comment defs with the TS option
// merge: a partial def for a default name (hash / slash / multi)
// inherits the fields it leaves unset (start, end, line, lex, eatline)
// instead of replacing the whole definition — with Line false honored
// as an explicit block conversion when End is also set — a nil def
// stays nil (the removal marker), and a def for a new name is inactive
// unless it sets Lex — TS makeCommentMatcher reads `lex: !!om.lex`,
// while the Go engine would default an unset Lex to true. Needed
// because the engine's option Deep merge replaces map values wholesale
// per key.
func normalizeCommentDefs(o *Options) {
	if o == nil || o.Comment == nil || o.Comment.Def == nil {
		return
	}
	defs := commentDefDefaults()
	norm := make(map[string]*CommentDef, len(o.Comment.Def))
	for name, def := range o.Comment.Def {
		if def == nil {
			norm[name] = nil
			continue
		}
		if base, ok := defs[name]; ok {
			if merged, ok := Deep(base, def).(*CommentDef); ok {
				// Honor an explicit block conversion (TS `line: false`):
				// the struct merge treats the zero bool as unset and
				// would keep the default Line:true, but Line false with
				// an End marker is unambiguous — line comments never
				// use End.
				if !def.Line && def.End != "" {
					merged.Line = false
				}
				norm[name] = merged
				continue
			}
		}
		cp := *def
		if cp.Lex == nil {
			f := false
			cp.Lex = &f
		}
		norm[name] = &cp
	}
	cc := *o.Comment
	cc.Def = norm
	o.Comment = &cc
}

// Grammar is the idiomatic tabnas grammar plugin: it applies jsonic's
// option defaults and registers the relaxed-JSON grammar (the val / map /
// list / pair / elem rules) on the engine instance. Use it the standard
// way, and `use` it before any plugin that builds on jsonic's rules:
//
//	j := tabnas.Make()
//	j.Use(jsonic.Grammar)
//	j.Use(csv) // builds on jsonic's value/map/list rules
//	out, _ := j.Parse("a:1,b:[x,y,z]")
//
// The Jsonic-style helpers (Make, Parse, MakeJSON) are a legacy
// compatibility layer that installs this same plugin. To register the
// grammar rules without the jsonic branding (mirroring the TS
// `registerJsonicGrammar` export), use RegisterJsonicGrammar in grammar.go.
func Grammar(j *Jsonic, _ map[string]any) (err error) {
	// Never let grammar installation panic out of the plugin; report it.
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("jsonic: Grammar plugin failed: %v", r)
		}
	}()
	j.SetOptions(jsonicOptions())
	return RegisterJsonicGrammar(j)
}

// grammarPlugin installs the relaxed-JSON rules without applying jsonic's
// option branding. It is what the legacy Make registers (Make applies the
// branding as a base option, before caller options, so a caller-supplied
// errmsg.name wins), and the body of the exported RegisterJsonicGrammar
// (grammar.go), the Go counterpart of TS `registerJsonicGrammar`. The
// decoration guard makes it idempotent under the engine's SetOptions plugin
// re-run while still letting Derive build the grammar fresh on a child.
func grammarPlugin(j *Jsonic, _ map[string]any) (err error) {
	// Convert any unexpected panic (e.g. from the @tabnas/json dependency)
	// into an error so plugin application never crashes the caller.
	defer func() {
		if r := recover(); r != nil {
			err = fmt.Errorf("jsonic: grammar installation failed: %v", r)
		}
	}()
	if mark, _ := j.Decoration(grammarMark).(bool); mark {
		return nil
	}
	j.Decorate(grammarMark, true)
	// Install the standard-JSON grammar core (val / map / list / pair /
	// elem) from the @tabnas/json plugin, then layer jsonic's relaxed
	// extensions on top of it.
	tjson.RegisterJSONGrammar(j)
	return buildGrammar(j.RSM(), j.Config())
}

// init registers jsonic as the engine's text parser so SetOptionsText and
// GrammarText (which parse a jsonic-format options/grammar string) work —
// the grammar-free engine has no parser of its own.
//
// The engine's text-options consumers (SetOptionsText, GrammarText,
// MapToOptions) expect a plain map[string]any tree and assert on it deeply.
// Since a jsonic parse now yields an insertion-ordered *OrderedMap object
// node, the registered parser flattens that node (recursively) back to a
// plain map before returning: an options/grammar spec is configuration, so
// its key order carries no meaning and dropping it keeps those consumers
// working unchanged.
func init() {
	tabnas.RegisterTextParser(func(src string) (any, error) {
		v, err := Make().Parse(src)
		if err != nil {
			return v, err
		}
		return plainNode(v), nil
	})
}

// plainNode recursively rewrites any *OrderedMap object node into a plain
// map[string]any (dropping insertion order) and copies []any elements,
// leaving all other values untouched. It is used only to feed the engine's
// map-shaped text-options consumers; jsonic's own Parse result keeps its
// *OrderedMap so callers still see source order.
func plainNode(v any) any {
	switch node := v.(type) {
	case *OrderedMap:
		m := make(map[string]any, len(node.Keys))
		for _, k := range node.Keys {
			m[k] = plainNode(node.Vals[k])
		}
		return m
	case map[string]any:
		m := make(map[string]any, len(node))
		for k, elem := range node {
			m[k] = plainNode(elem)
		}
		return m
	case []any:
		out := make([]any, len(node))
		for i, elem := range node {
			out[i] = plainNode(elem)
		}
		return out
	default:
		return v
	}
}

// Make creates a relaxed-JSON parser instance: a tabnas engine with the
// jsonic grammar plugin installed, plus any caller options on top.
// Equivalent to the historic jsonic.make().
func Make(opts ...Options) *Jsonic {
	var o Options
	if len(opts) > 0 {
		o = opts[0]
	}

	// Hold back rule include/exclude: they must filter the grammar only
	// after it is built, not at construction.
	base := o
	var inc, exc string
	if o.Rule != nil {
		inc, exc = o.Rule.Include, o.Rule.Exclude
		ruleCopy := *o.Rule
		ruleCopy.Include = ""
		ruleCopy.Exclude = ""
		base.Rule = &ruleCopy
	}

	// Merge partial comment defs with jsonic's defaults per name (the
	// whole-Options Deep below replaces map values wholesale per key).
	normalizeCommentDefs(&base)

	// Construct the engine with jsonic's branding as a base and caller
	// options merged on top (so a caller errmsg.name / tag / lexer setting
	// wins). Constructing with the merged options means the instance id,
	// config, and any option-conditional grammar alternates (list.child,
	// list.pair, …) are all decided against the final settings.
	merged, ok := Deep(jsonicOptions(), base).(Options)
	if !ok {
		// Deep merging two Options always yields Options; fall back rather
		// than panic on the impossible case.
		merged = base
	}
	j := tabnas.Make(merged)

	// Register (not just run) the grammar so the engine re-applies it when
	// deriving a child instance. The relaxed-JSON grammar is a plugin.
	// grammarPlugin never panics (it recovers internally); its error only
	// fires on an incompatible @tabnas/json core and cannot be surfaced
	// through Make's legacy *Jsonic signature — use the Grammar plugin
	// directly (it returns the error) when that matters.
	_ = j.Use(grammarPlugin, nil)

	if inc != "" || exc != "" {
		j.SetOptions(Options{Rule: &RuleOptions{Include: inc, Exclude: exc}})
	}
	return j
}

// Empty creates a parser instance with the jsonic configuration but no
// grammar rules, for building a grammar from scratch. Matches the historic
// jsonic.empty().
func Empty(opts ...Options) *Jsonic {
	j := Make(opts...)
	for _, rs := range j.RSM() {
		rs.Clear()
	}
	return j
}

// MakeJSON creates an instance configured to accept strict JSON only.
// Mirrors the TypeScript Jsonic.make('json'): it installs the full jsonic
// grammar, then restricts it to the json-tagged alternates and disables
// the lenient lexer features. Rejects unquoted keys/values, comments,
// hex/octal/binary numbers, trailing commas, leading-zero numbers,
// single-quoted or backtick strings, and empty input.
func MakeJSON() *Jsonic {
	f := false
	tr := true
	return Make(Options{
		Text: &TextOptions{Lex: &f},
		Number: &NumberOptions{
			Hex: &f, Oct: &f, Bin: &f,
			Sep:     "",
			Exclude: notStrictJSONNumber,
		},
		String: &StringOptions{
			Chars:        `"`,
			MultiChars:   "",
			AllowUnknown: &f,
			// No \xHH or \u{...}; strict JSON has only \uXXXX.
			EscapeStrict: &tr,
		},
		Comment: &CommentOptions{Lex: &f},
		Map:     &MapOptions{Extend: &f},
		Lex:     &LexOptions{Empty: &f},
		// Mirrors the TS `tokenSet: { KEY: ['#ST', null, null, null] }`:
		// strict JSON keys are quoted strings only, never #TX/#NR/#VL.
		// This is what makes `{1:1}` and `{null:null}` errors here, as
		// they are in TS and in encoding/json.
		TokenSet: map[string][]string{
			"KEY": {"#ST"},
		},
		Rule: &RuleOptions{
			Finish:  &f,
			Include: "json",
		},
	})
}

// strictJSONNumber is the RFC 8259 / ECMA-404 number grammar:
//
//	-? (0 | [1-9][0-9]*) (. [0-9]+)? ([eE] [-+]? [0-9]+)?
var strictJSONNumber = regexp.MustCompile(
	`^-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][-+]?[0-9]+)?$`)

// notStrictJSONNumber is the strict-JSON `number.exclude` predicate. The
// relaxed number matcher also accepts a leading `+`, leading zeros (`012`)
// and a bare leading or trailing `.` (`.123`, `1.`), none of which are
// valid JSON — exclude them so they fall through to the (disabled) text
// matcher and raise an error instead of parsing silently. Mirrors the TS
// `number.exclude` regexp in makeJSON (ts/src/grammar.ts).
func notStrictJSONNumber(s string) bool {
	return !strictJSONNumber.MatchString(s)
}

// defaultParser is a lazily-created instance reused by Parse, so repeated
// calls don't rebuild the engine and grammar each time. Building the jsonic
// grammar dominates a parse (see perf_test.go), so a rebuild-per-call Parse
// is many times slower than necessary. Parsing builds a fresh context per
// call and only reads instance state, so the shared instance is safe for
// concurrent use. Mirrors @tabnas/json's Parse.
var (
	defaultOnce   sync.Once
	defaultParser *Jsonic
)

// Parse parses a jsonic source string with default settings and returns
// the resulting value, or a *JsonicError on failure.
func Parse(src string) (any, error) {
	defaultOnce.Do(func() { defaultParser = Make() })
	return defaultParser.Parse(src)
}
