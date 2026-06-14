// Copyright (c) 2013-2026 Richard Rodger, MIT License

package jsonic

import (
	tjson "github.com/tabnas/json/go"
	tabnas "github.com/tabnas/parser/go"
)

// Version is the current version of the jsonic Go module.
const Version = "0.1.22"

// grammarMark is the decoration key that records whether the relaxed-JSON
// grammar has already been installed on an instance. It guards the Grammar
// plugin so that re-running it (which the engine does on every SetOptions)
// does not rebuild the base rules and clobber caller rule modifications,
// while still letting Derive build the grammar fresh on a child (Derive
// copies decorations only after re-running plugins).
const grammarMark = "jsonic$grammar"

// jsonicOptions are the option overrides the grammar plugin layers on top
// of the engine's already-relaxed lexer defaults: the jsonic error
// identity. Error message templates and the relaxed lexer config are
// shared with the engine defaults, so only the branding differs.
func jsonicOptions() Options {
	return Options{
		ErrMsg: &ErrMsgOptions{
			Name: "jsonic",
			Link: "https://jsonic.senecajs.org",
		},
	}
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
// compatibility layer that installs this same plugin.
func Grammar(j *Jsonic, opts map[string]any) error {
	j.SetOptions(jsonicOptions())
	return grammarPlugin(j, opts)
}

// grammarPlugin installs the relaxed-JSON rules without applying jsonic's
// option branding. It is what the legacy Make registers (Make applies the
// branding as a base option, before caller options, so a caller-supplied
// errmsg.name wins). The decoration guard makes it idempotent under the
// engine's SetOptions plugin re-run while still letting Derive build the
// grammar fresh on a child.
func grammarPlugin(j *Jsonic, _ map[string]any) error {
	if mark, _ := j.Decoration(grammarMark).(bool); mark {
		return nil
	}
	j.Decorate(grammarMark, true)
	// Install the standard-JSON grammar core (val / map / list / pair /
	// elem) from the @tabnas/json plugin, then layer jsonic's relaxed
	// extensions on top of it.
	tjson.RegisterJSONGrammar(j)
	buildGrammar(j.RSM(), j.Config())
	return nil
}

// init registers jsonic as the engine's text parser so SetOptionsText and
// GrammarText (which parse a jsonic-format options/grammar string) work —
// the grammar-free engine has no parser of its own.
func init() {
	tabnas.RegisterTextParser(func(src string) (any, error) {
		return Make().Parse(src)
	})
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

	// Construct the engine with jsonic's branding as a base and caller
	// options merged on top (so a caller errmsg.name / tag / lexer setting
	// wins). Constructing with the merged options means the instance id,
	// config, and any option-conditional grammar alternates (list.child,
	// list.pair, …) are all decided against the final settings.
	merged := Deep(jsonicOptions(), base).(Options)
	j := tabnas.Make(merged)

	// Register (not just run) the grammar so the engine re-applies it when
	// deriving a child instance. The relaxed-JSON grammar is a plugin.
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
	return Make(Options{
		Text: &TextOptions{Lex: &f},
		Number: &NumberOptions{
			Hex: &f, Oct: &f, Bin: &f,
			Sep: "",
			Exclude: func(s string) bool {
				return len(s) >= 2 && s[0] == '0' && s[1] == '0'
			},
		},
		String: &StringOptions{
			Chars:        `"`,
			MultiChars:   "",
			AllowUnknown: &f,
		},
		Comment: &CommentOptions{Lex: &f},
		Map:     &MapOptions{Extend: &f},
		Lex:     &LexOptions{Empty: &f},
		Rule: &RuleOptions{
			Finish:  &f,
			Include: "json",
		},
	})
}

// Parse parses a jsonic source string with default settings and returns
// the resulting value, or a *JsonicError on failure.
func Parse(src string) (any, error) {
	return Make().Parse(src)
}
