// Copyright (c) 2013-2026 Richard Rodger, MIT License

// Package jsonic is the relaxed-JSON grammar plugin for the tabnas
// parsing engine (github.com/tabnas/parser/go). The engine ships no
// grammar; this package supplies the lenient-JSON one and a legacy
// Jsonic-style API on top of it.
//
// engine.go re-exports the engine's public surface under the historic
// jsonic names so existing callers (and this package's own grammar and
// tests) keep compiling unchanged. The single source of truth for the
// parser, lexer, rule machinery, options and utilities is the tabnas
// engine — nothing here re-implements it.
package jsonic

import (
	tabnas "github.com/tabnas/parser/go"
)

// --- Core engine type -------------------------------------------------

// Jsonic is a configured parser instance. It is the tabnas engine type;
// the relaxed-JSON grammar is installed onto it by Make / the Grammar
// plugin. Kept under the historic name for backward compatibility.
type Jsonic = tabnas.Tabnas

// JsonicError is the structured error returned by Parse on failure.
// It is the engine's error type; the [jsonic/<code>] tag comes from the
// errmsg.name option that the grammar plugin sets to "jsonic".
type JsonicError = tabnas.TabnasError

// --- Engine type aliases ---------------------------------------------

type (
	AltAction      = tabnas.AltAction
	AltCond        = tabnas.AltCond
	AltError       = tabnas.AltError
	AltModifier    = tabnas.AltModifier
	AltModListOpts = tabnas.AltModListOpts
	AltSpec        = tabnas.AltSpec
	ColorConfig    = tabnas.ColorConfig
	ColorOptions   = tabnas.ColorOptions
	CommentDef     = tabnas.CommentDef
	CommentOptions = tabnas.CommentOptions
	ConfigModifier = tabnas.ConfigModifier
	Context        = tabnas.Context
	Entry          = tabnas.Entry
	ErrMsgOptions  = tabnas.ErrMsgOptions
	FixedOptions   = tabnas.FixedOptions
	FuncRef        = tabnas.FuncRef

	GrammarAltListSpec = tabnas.GrammarAltListSpec
	GrammarAltSpec     = tabnas.GrammarAltSpec
	GrammarInjectSpec  = tabnas.GrammarInjectSpec
	GrammarRuleSpec    = tabnas.GrammarRuleSpec
	GrammarSetting     = tabnas.GrammarSetting
	GrammarSettingAlt  = tabnas.GrammarSettingAlt
	GrammarSettingRule = tabnas.GrammarSettingRule
	GrammarSpec        = tabnas.GrammarSpec

	InfoOptions     = tabnas.InfoOptions
	Lex             = tabnas.Lex
	LexCheck        = tabnas.LexCheck
	LexCheckResult  = tabnas.LexCheckResult
	LexConfig       = tabnas.LexConfig
	LexMatcher      = tabnas.LexMatcher
	LexOptions      = tabnas.LexOptions
	LexSub          = tabnas.LexSub
	LineOptions     = tabnas.LineOptions
	ListOptions     = tabnas.ListOptions
	ListRef         = tabnas.ListRef
	MakeLexMatcher  = tabnas.MakeLexMatcher
	MapMergeFunc    = tabnas.MapMergeFunc
	MapOptions      = tabnas.MapOptions
	MapRef          = tabnas.MapRef
	MatchOptions    = tabnas.MatchOptions
	MatchSpec       = tabnas.MatchSpec
	MatchTokenEntry = tabnas.MatchTokenEntry
	MatchValueEntry = tabnas.MatchValueEntry
	MatchValueSpec  = tabnas.MatchValueSpec
	MatcherEntry    = tabnas.MatcherEntry
	ModListOpts     = tabnas.ModListOpts
	NumberOptions   = tabnas.NumberOptions
	Options         = tabnas.Options
	ParseOptions    = tabnas.ParseOptions
	Parser          = tabnas.Parser
	ParserOptions   = tabnas.ParserOptions

	// Plugin is a function that configures a parser instance. It is the
	// engine's plugin type; jsonic's own grammar (see Grammar) is one.
	Plugin = tabnas.Plugin

	Point           = tabnas.Point
	PropertyOptions = tabnas.PropertyOptions
	ResultOptions   = tabnas.ResultOptions
	Rule            = tabnas.Rule
	RuleDefiner     = tabnas.RuleDefiner
	RuleOptions     = tabnas.RuleOptions
	RuleSpec        = tabnas.RuleSpec
	RuleState       = tabnas.RuleState
	RuleSub         = tabnas.RuleSub
	SafeOptions     = tabnas.SafeOptions
	ScanOut         = tabnas.ScanOut
	ScanSpec        = tabnas.ScanSpec
	SpaceOptions    = tabnas.SpaceOptions
	StateAction     = tabnas.StateAction
	StringOptions   = tabnas.StringOptions
	Text            = tabnas.Text
	TextOptions     = tabnas.TextOptions
	Tin             = tabnas.Tin
	Token           = tabnas.Token
	TokenValFunc    = tabnas.TokenValFunc
	UtilBag         = tabnas.UtilBag
	ValModifier     = tabnas.ValModifier
	ValueDef        = tabnas.ValueDef
	ValueDefEntry   = tabnas.ValueDefEntry
	ValueOptions    = tabnas.ValueOptions
)

// --- Constants --------------------------------------------------------

// Token identification numbers (Tin) for the standard tokens.
const (
	TinBD  = tabnas.TinBD
	TinZZ  = tabnas.TinZZ
	TinUK  = tabnas.TinUK
	TinAA  = tabnas.TinAA
	TinSP  = tabnas.TinSP
	TinLN  = tabnas.TinLN
	TinCM  = tabnas.TinCM
	TinNR  = tabnas.TinNR
	TinST  = tabnas.TinST
	TinTX  = tabnas.TinTX
	TinVL  = tabnas.TinVL
	TinOB  = tabnas.TinOB
	TinCB  = tabnas.TinCB
	TinOS  = tabnas.TinOS
	TinCS  = tabnas.TinCS
	TinCL  = tabnas.TinCL
	TinCA  = tabnas.TinCA
	TinMAX = tabnas.TinMAX

	// Rule states.
	OPEN  = tabnas.OPEN
	CLOSE = tabnas.CLOSE
)

// --- Vars -------------------------------------------------------------

var (
	// Undefined is the sentinel for "no value" (distinct from nil).
	Undefined = tabnas.Undefined

	// NoToken / NoRule are the zero sentinels used in rule actions.
	NoToken = tabnas.NoToken
	NoRule  = tabnas.NoRule

	// Skip is the deep-merge skip sentinel.
	Skip = tabnas.Skip

	// FixedTokens is the global fixed-token table (src → Tin).
	FixedTokens = tabnas.FixedTokens

	// TinSetVAL / TinSetKEY are the default value and key token sets.
	TinSetVAL = tabnas.TinSetVAL
	TinSetKEY = tabnas.TinSetKEY
)

// --- Function re-exports ----------------------------------------------

var (
	Deep                    = tabnas.Deep
	IsUndefined             = tabnas.IsUndefined
	UnwrapUndefined         = tabnas.UnwrapUndefined
	MakeRule                = tabnas.MakeRule
	MakeRuleCond            = tabnas.MakeRuleCond
	MakeToken               = tabnas.MakeToken
	MapToOptions            = tabnas.MapToOptions
	NewLex                  = tabnas.NewLex
	NewParser               = tabnas.NewParser
	ParseAlts               = tabnas.ParseAlts
	ResolveFuncRefs         = tabnas.ResolveFuncRefs
	ResolveGrammarAltStatic = tabnas.ResolveGrammarAltStatic
	Str                     = tabnas.Str
	Snip                    = tabnas.Snip
	StrInject               = tabnas.StrInject
	Keys                    = tabnas.Keys
	Values                  = tabnas.Values
	Entries                 = tabnas.Entries
	Omap                    = tabnas.Omap
	ValidateGroupTags       = tabnas.ValidateGroupTags
	Scan                    = tabnas.Scan
	BuildCharRunSpec        = tabnas.BuildCharRunSpec
	BuildLineRunSpec        = tabnas.BuildLineRunSpec
	BuildStringBodySpec     = tabnas.BuildStringBodySpec
	NormAlt                 = tabnas.NormAlt
	NormAlts                = tabnas.NormAlts
	ModList                 = tabnas.ModList
	LookupRef               = tabnas.LookupRef
	RequireRef              = tabnas.RequireRef
	IsFuncRef               = tabnas.IsFuncRef
	IsSkip                  = tabnas.IsSkip
	RegisterTextParser      = tabnas.RegisterTextParser
	DefaultLexConfig        = tabnas.DefaultLexConfig
	CEq                     = tabnas.CEq
	CGt                     = tabnas.CGt
	CGte                    = tabnas.CGte
	CLt                     = tabnas.CLt
	CLte                    = tabnas.CLte
	CNe                     = tabnas.CNe
)
