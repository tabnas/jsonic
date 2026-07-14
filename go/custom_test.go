// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

// custom_test.go ports the TS `custom` test theme (ts/test/custom.test.js)
// plus the custom-matcher tests from ts/test/feature.test.js
// ('match-custom'), exercising options.match.token / options.match.value,
// custom fixed tokens, tokenSet overrides, string.replace, and the parser
// rule-definition API.
//
// Not ported (documented in doc/differences.md):
//   - 'parser-handler-actives': the Go Rule has no ao/bc/ac suppression
//     flags for the `h` alt modifier to clear.
//   - 'parser-action-errors' / 'parser-token-error-why': Go state actions
//     have no return value, so they cannot return error tokens.
//   - 'parser-empty-clean': covered by TestEmpty (plugin_test.go); Go
//     Empty() clears rule alternates but keeps token registrations.
//   - 'merge': covered by TestAlignmentMapMergeFunc (alignment_test.go).

import (
	"fmt"
	"reflect"
	"regexp"
	"strconv"
	"strings"
	"testing"
)

// fixedTok builds the pointer-valued fixed.token option map (nil values
// mean deletion in the engine option, which these tests never need).
func fixedTok(m map[string]string) map[string]*string {
	out := make(map[string]*string, len(m))
	for k, v := range m {
		v := v
		out[k] = &v
	}
	return out
}

// makeNoRules mirrors the TS test helper make_norules: a jsonic instance
// with every grammar rule deleted, so tests can build rules from scratch.
func makeNoRules(opts ...Options) *Jsonic {
	j := Make(opts...)
	names := make([]string, 0, len(j.RSM()))
	for name := range j.RSM() {
		names = append(names, name)
	}
	for _, name := range names {
		j.Rule(name, nil)
	}
	return j
}

func parseWant(t *testing.T, j *Jsonic, src string, want any) {
	t.Helper()
	got, err := j.Parse(src)
	if err != nil {
		t.Fatalf("Parse(%q) unexpected error: %v", src, err)
	}
	if !reflect.DeepEqual(got, want) {
		t.Errorf("Parse(%q): got %#v, want %#v", src, got, want)
	}
}

func parseErrContains(t *testing.T, j *Jsonic, src string, frag string) {
	t.Helper()
	_, err := j.Parse(src)
	if err == nil {
		t.Fatalf("Parse(%q): expected error containing %q, got none", src, frag)
	}
	if !strings.Contains(err.Error(), frag) {
		t.Errorf("Parse(%q): error should contain %q, got: %v", src, frag, err)
	}
}

// --- custom: fixed-tokens ---

func TestCustomFixedTokens(t *testing.T) {
	j := Make(Options{Fixed: &FixedOptions{Token: fixedTok(map[string]string{
		"#NOT":     "~",
		"#IMPLIES": "=>",
		"#DEFINE":  ":-",
		"#MARK":    "##",
		"#TRIPLE":  "///",
	})}})

	NOT := j.Token("#NOT")
	IMPLIES := j.Token("#IMPLIES")
	DEFINE := j.Token("#DEFINE")
	MARK := j.Token("#MARK")
	TRIPLE := j.Token("#TRIPLE")

	node := func(val string) AltAction {
		return func(r *Rule, _ *Context) { r.Node = val }
	}

	j.Rule("val", func(rs *RuleSpec, _ *Parser) {
		rs.PrependOpen(
			&AltSpec{S: [][]Tin{{NOT}}, A: node("<not>")},
			&AltSpec{S: [][]Tin{{IMPLIES}}, A: node("<implies>")},
			&AltSpec{S: [][]Tin{{DEFINE}}, A: node("<define>")},
			&AltSpec{S: [][]Tin{{MARK}}, A: node("<mark>")},
			&AltSpec{S: [][]Tin{{TRIPLE}}, A: node("<triple>")},
		)
	})

	parseWant(t, j, "a:~,b:1,c:~,d:=>,e::-,f:##,g:///,h:a,i:# foo",
		map[string]any{
			"a": "<not>",
			"b": 1.0,
			"c": "<not>",
			"d": "<implies>",
			"e": "<define>",
			"f": "<mark>",
			"g": "<triple>",
			"h": "a",
			"i": nil, // implicit null
		})
}

// --- custom: tokenset-idenkey (match.token function + regexp forms) ---
//
// The TS test also extends the KEY/VAL token sets so #ID works as an
// unquoted map key; in Go the jsonic grammar resolves #KEY/#VAL statically
// when its rules are built, so a tokenSet override cannot reach those
// alternates (see doc/differences.md). The matchers themselves are fully
// supported: this port gates them with explicit `val` alternates instead.

func TestCustomMatchTokenSetIdenKey(t *testing.T) {
	days := map[string]string{
		"monday":  "mon",
		"tuesday": "tue",
	}

	j := Make(Options{
		Match: &MatchOptions{
			// Function-form custom token matcher (TS match.token with a
			// LexMatcher value).
			TokenFn: map[string]LexMatcher{
				"#DAY": func(lex *Lex, _ *Rule) *Token {
					pnt := lex.Cursor()
					end := pnt.SI + 11
					if end > pnt.Len {
						end = pnt.Len
					}
					daystr := strings.ToLower(lex.Src[pnt.SI:end])
					for day, short := range days {
						if strings.HasPrefix(daystr, day) {
							daylen := len(day)
							tkn := lex.Token("#VL", TinVL, short, lex.Src[pnt.SI:pnt.SI+daylen])
							pnt.SI += daylen
							pnt.CI += daylen
							return tkn
						}
					}
					return nil
				},
			},
			// Regexp-form custom token matcher (TS match.token with a
			// RegExp value). Go regexps must anchor with ^ explicitly.
			Token: map[string]*regexp.Regexp{
				"#ID": regexp.MustCompile(`^[a-zA-Z_][a-zA-Z_0-9]*`),
			},
		},
	})

	DAY := j.Token("#DAY")

	// The matchers are gated by the rule position: they only fire where
	// their token is expected. Expecting only DAY at `val` open leaves
	// ordinary unquoted keys and values lexing as #TX.
	j.Rule("val", func(rs *RuleSpec, _ *Parser) {
		rs.PrependOpen(&AltSpec{S: [][]Tin{{DAY}}})
	})

	parseWant(t, j, "a:1", map[string]any{"a": 1.0})
	parseWant(t, j, "a:x", map[string]any{"a": "x"})
	parseWant(t, j, "a:monday", map[string]any{"a": "mon"})
	parseWant(t, j, "a:tuesday", map[string]any{"a": "tue"})
	parseWant(t, j, "monday", "mon")

	// The regexp-form matcher fires wherever #ID is position-expected.
	// NOTE: unlike TS (where tokenSet KEY gains #ID), expecting #ID at
	// `val` open makes unquoted keys lex as #ID, which the statically
	// resolved KEY alternates reject — so this instance is value-only
	// (quoted keys still work).
	j2 := Make(Options{Match: &MatchOptions{
		Token: map[string]*regexp.Regexp{
			"#ID": regexp.MustCompile(`^[a-zA-Z_][a-zA-Z_0-9]*`),
		},
	}})
	ID2 := j2.Token("#ID")
	j2.Rule("val", func(rs *RuleSpec, _ *Parser) {
		rs.PrependOpen(&AltSpec{S: [][]Tin{{ID2}}})
	})

	parseWant(t, j2, "xyz", "xyz")
	parseWant(t, j2, "[qq,rr]", []any{"qq", "rr"})
	parseWant(t, j2, `"a":xyz`, map[string]any{"a": "xyz"})

	// The default parser is unaffected.
	def := Make()
	parseWant(t, def, "a:1", map[string]any{"a": 1.0})
	parseWant(t, def, "a*:1", map[string]any{"a*": 1.0})
	parseWant(t, def, "b:monday", map[string]any{"b": "monday"})
}

// --- feature: match-custom (match.value regexp + handler, match.token) ---

func TestCustomMatchValueAndToken(t *testing.T) {
	j := Make(Options{Match: &MatchOptions{
		Value: map[string]*MatchValueSpec{
			"foobar": {
				Match: regexp.MustCompile(`^foobar(\d)`),
				Val: func(res []string) any {
					n, _ := strconv.Atoi(res[1])
					return float64(n)
				},
			},
			// No need to turn off number lexing: the match matcher runs
			// first.
			"commadigits": {
				Match: regexp.MustCompile(`^\d+(,\d+)+`),
				Val: func(res []string) any {
					n, _ := strconv.Atoi(strings.ReplaceAll(res[0], ",", ""))
					return float64(20 * n)
				},
			},
		},
		Token: map[string]*regexp.Regexp{
			"FOO": regexp.MustCompile(`^foo`),
		},
	}})

	FOO := j.Token("FOO")
	j.Rule("val", func(rs *RuleSpec, _ *Parser) {
		rs.PrependOpen(&AltSpec{
			S: [][]Tin{{FOO}},
			A: func(r *Rule, _ *Context) { r.Node = "Foo" },
		})
	})

	parseWant(t, j, "foo", "Foo")
	parseWant(t, j, "foobar1", 1.0)

	// Still parses plain numbers.
	parseWant(t, j, "[1 2,3 4]", []any{1.0, 460.0, 4.0})
}

// --- feature: value-custom (value.def regexp + handler) ---

func TestCustomValueDefMatch(t *testing.T) {
	f := false
	j := Make(Options{
		Number: &NumberOptions{Lex: &f}, // needed for commadigits
		Value: &ValueOptions{Def: map[string]*ValueDef{
			"foo": {Val: 99.0},
			"bar": {Val: map[string]any{"x": 1.0}},
			"zed": {
				Match: regexp.MustCompile(`Z(\d)`),
				ValFunc: func(res []string) any {
					n, _ := strconv.Atoi(res[1])
					return float64(n)
				},
			},
			// Stops at tokens.
			"cap": {
				Match:   regexp.MustCompile(`[A-Z]+`),
				ValFunc: func(res []string) any { return strings.ToLower(res[0]) },
			},
			// Does not stop at tokens.
			"commadigits": {
				Match: regexp.MustCompile(`^\d+(,\d+)+`),
				ValFunc: func(res []string) any {
					n, _ := strconv.Atoi(strings.ReplaceAll(res[0], ",", ""))
					return float64(20 * n)
				},
				Consume: true,
			},
		}},
	})

	parseWant(t, j, "", nil)
	parseWant(t, j, "foo", 99.0)
	parseWant(t, j, "bar", map[string]any{"x": 1.0})
	parseWant(t, j, "a:foo", map[string]any{"a": 99.0})
	parseWant(t, j, "a:bar", map[string]any{"a": map[string]any{"x": 1.0}})

	parseWant(t, j, "a:Z1", map[string]any{"a": 1.0})
	parseWant(t, j, "a:Zx", map[string]any{"a": "Zx"})

	parseWant(t, j, "[A,B]", []any{"a", "b"})
	parseWant(t, j, "[1 2,3] ", []any{"1", 460.0})
}

// --- custom: string-replace (with TS-aligned unprintable errors) ---

func TestCustomStringReplace(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	j0 := Make(Options{String: &StringOptions{
		Replace: map[rune]string{'A': "B", 'D': ""},
	}})

	parseWant(t, j0, `"aAc"`, "aBc")
	parseWant(t, j0, `"aAcDe"`, "aBce")

	// A raw newline inside a non-multiline string is `unprintable`,
	// reported at the newline itself (line 2 = ` "Ac\n`, so column 5).
	wantUnprintableAt(t, j0, "x:\n \"Ac\n\"", 2, 5)

	// A replaced control char is legal body for the error scan: the
	// '\n' below is replaced, so the error is the '\r' at column 6.
	// NOTE: the successful-parse side of TS string.replace on control
	// chars (j1('"aAc\n"') === 'aBcX') is not supported by the Go
	// engine's string matcher — see doc/differences.md.
	j1 := Make(Options{String: &StringOptions{
		Replace: map[rune]string{'A': "B", '\n': "X"},
	}})
	wantUnprintableAt(t, j1, "x:\n \"ac\n\r\"", 2, 6)

	j2 := Make(Options{String: &StringOptions{
		Replace: map[rune]string{'A': "B", '\n': ""},
	}})
	wantUnprintableAt(t, j2, "x:\n \"ac\n\r\"", 2, 6)
}

func wantUnprintableAt(t *testing.T, j *Jsonic, src string, row, col int) {
	t.Helper()
	_, err := j.Parse(src)
	if err == nil {
		t.Fatalf("Parse(%q): expected unprintable error, got none", src)
	}
	je, ok := err.(*JsonicError)
	if !ok {
		t.Fatalf("Parse(%q): expected *JsonicError, got %T: %v", src, err, err)
	}
	if je.Code != "unprintable" {
		t.Errorf("Parse(%q): code got %q, want %q", src, je.Code, "unprintable")
	}
	if je.Row != row || je.Col != col {
		t.Errorf("Parse(%q): position got %d:%d, want %d:%d",
			src, je.Row, je.Col, row, col)
	}
}

// --- feature: unprintable / unterminated error codes (TS-aligned) ---

func TestCustomStringUnprintableCodes(t *testing.T) {
	j := Make()

	// Multiline quotes accept raw newlines.
	parseWant(t, j, "`\n`", "\n")

	// Raw control chars in a non-multiline string are unprintable.
	for _, src := range []string{
		"\"\n\"", "\"\t\"", "\"\f\"", "\"\b\"", "\"\v\"", "\"\x00\"",
		"\"\x1a\"",
		// ... including in a multiline string (only line chars are legal).
		"`\x1a`",
	} {
		_, err := j.Parse(src)
		je, ok := err.(*JsonicError)
		if !ok {
			t.Fatalf("Parse(%q): expected *JsonicError, got %v", src, err)
		}
		if je.Code != "unprintable" {
			t.Errorf("Parse(%q): code got %q, want %q", src, je.Code, "unprintable")
		}
	}

	// Escaped control chars are fine.
	parseWant(t, j, `"\n"`, "\n")
	parseWant(t, j, `"\t"`, "\t")
	parseWant(t, j, `"\f"`, "\f")
	parseWant(t, j, `"\b"`, "\b")
	parseWant(t, j, `"\v"`, "\v")
	parseWant(t, j, `"\w"`, "w")
	parseWant(t, j, `"\0"`, "0")

	// Plain unterminated strings still report unterminated_string.
	for _, src := range []string{`"x`, ` "x`, `a:"x`, "'''..."} {
		_, err := j.Parse(src)
		je, ok := err.(*JsonicError)
		if !ok {
			t.Fatalf("Parse(%q): expected *JsonicError, got %v", src, err)
		}
		if je.Code != "unterminated_string" {
			t.Errorf("Parse(%q): code got %q, want %q",
				src, je.Code, "unterminated_string")
		}
	}
}

// --- jsonic's built-in lex.match entry merges with caller entries ---

func TestCustomLexMatchMergesWithBuiltin(t *testing.T) {
	// A caller-supplied lex.match matcher must coexist with the
	// jsonic$unprintable matcher jsonicOptions installs.
	j := Make(Options{Lex: &LexOptions{Match: map[string]*MatchSpec{
		"dollar": {Order: 1_500_000, Make: func(_ *LexConfig, _ *Options) LexMatcher {
			return func(lex *Lex, _ *Rule) *Token {
				pnt := lex.Cursor()
				if pnt.SI+2 <= pnt.Len && lex.Src[pnt.SI:pnt.SI+2] == "$$" {
					tkn := lex.Token("#VL", TinVL, "DOLLAR", "$$")
					pnt.SI += 2
					pnt.CI += 2
					return tkn
				}
				return nil
			}
		}},
	}}})

	parseWant(t, j, "$$", "DOLLAR")
	wantUnprintableAt(t, j, "\"\n\"", 1, 2)
}

// --- custom: parser-empty-fixed ---

func TestCustomParserEmptyFixed(t *testing.T) {
	j := Empty(Options{
		Fixed: &FixedOptions{
			Lex:   boolPtr(true),
			Token: fixedTok(map[string]string{"#T0": "t0"}),
		},
		Rule: &RuleOptions{Start: "r0"},
	})
	T0 := j.Token("#T0")
	j.Rule("r0", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{T0}}}).
			AddBC(func(r *Rule, _ *Context) { r.Node = "~T0~" })
	})

	parseWant(t, j, "t0", "~T0~")
}

// --- custom: parser-before-after-state ---

func TestCustomParserBeforeAfterState(t *testing.T) {
	cases := []struct {
		phase string
		want  string
	}{
		{"bo", "BO"}, {"ao", "AO"}, {"bc", "BC"}, {"ac", "AC"},
	}
	for _, c := range cases {
		j := makeNoRules(Options{Rule: &RuleOptions{Start: "top"}})
		set := func(r *Rule, _ *Context) { r.Node = c.want }
		j.Rule("top", func(rs *RuleSpec, _ *Parser) {
			rs.AddOpen(&AltSpec{S: [][]Tin{{TinAA}, {TinAA}}}).
				AddClose(&AltSpec{S: [][]Tin{{TinAA}, {TinAA}}})
			switch c.phase {
			case "bo":
				rs.AddBO(set)
			case "ao":
				rs.AddAO(set)
			case "bc":
				rs.AddBC(set)
			case "ac":
				rs.AddAC(set)
			}
		})
		parseWant(t, j, "a", c.want)
	}
}

// --- custom: parser-empty-seq ---

func TestCustomParserEmptySeq(t *testing.T) {
	j := makeNoRules(Options{Rule: &RuleOptions{Start: "top"}})
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{TinAA}}}).
			AddBO(func(r *Rule, _ *Context) { r.Node = 4444 })
	})
	parseWant(t, j, "a", 4444)
}

// --- custom: parser-alt-ops (prepend / append / delete / move) ---

func TestCustomParserAltOps(t *testing.T) {
	j := makeNoRules(Options{
		Fixed: &FixedOptions{Token: fixedTok(map[string]string{
			"Ta": "a", "Tb": "b", "Tc": "c", "Td": "d", "Te": "e",
		})},
		Rule: &RuleOptions{Start: "top"},
	})

	Ta, Tb, Tc := j.Token("Ta"), j.Token("Tb"), j.Token("Tc")
	Td, Te := j.Token("Td"), j.Token("Te")

	var acc string
	add := func(s string) AltAction {
		return func(*Rule, *Context) { acc += s }
	}
	groups := func() []string {
		out := make([]string, 0)
		for _, alt := range j.RSM()["top"].OpenAlts() {
			out = append(out, alt.G)
		}
		return out
	}
	run := func(src string, wantAcc string, wantGroups []string) {
		t.Helper()
		acc = ""
		if _, err := j.Parse(src); err != nil {
			t.Fatalf("Parse(%q) unexpected error: %v", src, err)
		}
		if acc != wantAcc {
			t.Errorf("Parse(%q): acc got %q, want %q", src, acc, wantAcc)
		}
		if g := groups(); !reflect.DeepEqual(g, wantGroups) {
			t.Errorf("groups after %q: got %v, want %v", src, g, wantGroups)
		}
	}

	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(
			&AltSpec{S: [][]Tin{{TinZZ}}, G: "gz"},
			&AltSpec{S: [][]Tin{{Ta}}, R: "top", A: add("A"), G: "ga"},
		)
	})
	run("a", "A", []string{"gz", "ga"})

	// TS rs.open() prepends by default.
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.PrependOpen(&AltSpec{S: [][]Tin{{Tb}}, R: "top", A: add("B"), G: "gb"})
	})
	run("ab", "AB", []string{"gb", "gz", "ga"})

	// Append flag.
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{Tc}}, R: "top", A: add("C"), G: "gc"})
	})
	run("abc", "ABC", []string{"gb", "gz", "ga", "gc"})

	// Append + delete op.
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{Td}}, R: "top", A: add("D"), G: "gd"}).
			ModifyOpen(&AltModListOpts{Delete: []int{2}})
	})
	run("bcd", "BCD", []string{"gb", "gz", "gc", "gd"})

	// Append + move ops.
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{Te}}, R: "top", A: add("E"), G: "ge"}).
			ModifyOpen(&AltModListOpts{Move: []int{2, -1, 0, 1}})
	})
	run("bcde", "BCDE", []string{"gz", "gb", "gd", "ge", "gc"})

	// Delete ops only.
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.ModifyOpen(&AltModListOpts{Delete: []int{1, 3}})
	})
	run("cd", "CD", []string{"gz", "gd", "gc"})
}

// --- custom: parser-any-def ---

func TestCustomParserAnyDef(t *testing.T) {
	j := makeNoRules(Options{Rule: &RuleOptions{Start: "top"}})
	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{TinAA}, {TinTX}}}).
			AddAC(func(r *Rule, _ *Context) {
				r.Node = fmt.Sprintf("%v%v", r.O0.Val, r.O1.Val)
			})
	})

	parseWant(t, j, "a\nb", "ab")
	parseErrContains(t, j, "AAA,", "unexpected")
}

// --- custom: parser-multi-alts ---

func TestCustomParserMultiAlts(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	j := makeNoRules(Options{
		Fixed: &FixedOptions{Token: fixedTok(map[string]string{
			"Ta": "a", "Tb": "b", "Tc": "c",
		})},
		Rule: &RuleOptions{Start: "top"},
	})

	Ta, Tb, Tc := j.Token("Ta"), j.Token("Tb"), j.Token("Tc")

	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{Ta}, {Tb, Tc}}}).
			AddAC(func(r *Rule, _ *Context) {
				r.Node = strings.ToUpper(r.O0.Src + r.O1.Src)
			})
	})

	parseWant(t, j, "ab", "AB")
	parseWant(t, j, "ac", "AC")
	parseErrContains(t, j, "ad", "unexpected")
}

// --- custom: parser-value ---

func TestCustomParserValue(t *testing.T) {
	o0 := map[string]any{"x": 1.0}

	j := Make(Options{Value: &ValueOptions{Def: map[string]*ValueDef{
		"foo": {Val: "FOO"},
		"bar": {Val: "BAR"},
		"zed": {Val: 123.0},
		"qaz": {Val: false},
		"obj": {Val: o0},

		// Functions build values dynamically.
		"fun": {Val: TokenValFunc(func(*Rule, *Context) any { return "f0" })},
	}}})

	parseWant(t, j, "foo", "FOO")
	parseWant(t, j, "bar", "BAR")
	parseWant(t, j, "zed", 123.0)
	parseWant(t, j, "qaz", false)

	// NOTE: TS copies options deeply, so mutating o0 after make() does
	// not affect later parses there; Go value defs keep the caller's
	// reference (see doc/differences.md).
	parseWant(t, j, "obj", map[string]any{"x": 1.0})

	parseWant(t, j, "fun", "f0")
}

// --- custom: parser-mixed-token ---

func TestCustomParserMixedToken(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	cs := []string{
		"Q", // generic char
		"/", // mixed use as comment marker
	}

	for _, c := range cs {
		j := Make(Options{Fixed: &FixedOptions{
			Token: fixedTok(map[string]string{"#T/": c}),
		}})

		FS := j.Token("#T/")

		j.Rule("val", func(rs *RuleSpec, _ *Parser) {
			rs.PrependOpen(&AltSpec{
				S: [][]Tin{{FS}, {TinTX}},
				A: func(r *Rule, _ *Context) {
					r.O0.Val = "@" + fmt.Sprintf("%v", r.O1.Val)
				},
			})
		})

		j.Rule("elem", func(rs *RuleSpec, _ *Parser) {
			rs.PrependClose(&AltSpec{
				S: [][]Tin{{FS}, {TinTX}},
				R: "elem",
				B: 2,
			})
		})

		parseWant(t, j, "["+c+"x"+c+"y]", []any{"@x", "@y"})
	}
}

// --- custom: parser-condition-depth ---

func TestCustomParserConditionDepth(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	j := makeNoRules(Options{
		Fixed: &FixedOptions{Token: fixedTok(map[string]string{"#F": "f", "#B": "b"})},
		Rule:  &RuleOptions{Start: "top"},
	})

	FT, BT := j.Token("#F"), j.Token("#B")

	var acc string
	add := func(s string) StateAction {
		return func(*Rule, *Context) { acc += s }
	}

	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{P: "foo", C: func(r *Rule, _ *Context) bool { return r.D <= 0 }}).
			AddBO(add("T"))
	})
	j.Rule("foo", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{FT}},
			P: "bar",
			C: func(r *Rule, _ *Context) bool { return r.D <= 1 },
		}).AddAO(add("F"))
	})
	j.Rule("bar", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{BT}},
			C: func(r *Rule, _ *Context) bool { return r.D <= 2 },
		}).AddAO(add("B"))
	})

	acc = ""
	if _, err := j.Parse("fb"); err != nil {
		t.Fatalf("Parse(fb) unexpected error: %v", err)
	}
	if acc != "TFB" {
		t.Errorf("acc: got %q, want %q", acc, "TFB")
	}

	j.Rule("bar", func(rs *RuleSpec, _ *Parser) {
		rs.Clear().
			AddOpen(&AltSpec{
				S: [][]Tin{{BT}},
				C: func(r *Rule, _ *Context) bool { return r.D <= 0 },
			}).AddAO(add("B"))
	})

	parseErrContains(t, j, "fb", "unexpected")
}

// --- custom: parser-condition-counter ---

func TestCustomParserConditionCounter(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	j := makeNoRules(Options{
		Fixed: &FixedOptions{Token: fixedTok(map[string]string{"#F": "f", "#B": "b"})},
		Rule:  &RuleOptions{Start: "top"},
	})

	FT, BT := j.Token("#F"), j.Token("#B")

	var acc string
	add := func(s string) StateAction {
		return func(*Rule, *Context) { acc += s }
	}

	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		// incr x=1, y=2
		rs.AddOpen(&AltSpec{P: "foo", N: map[string]int{"x": 1, "y": 2}}).
			AddBO(add("T"))
	})
	j.Rule("foo", func(rs *RuleSpec, _ *Parser) {
		// (x <= 1, y <= 2) -> pass; y reset to 0 for children.
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{FT}},
			P: "bar",
			C: func(r *Rule, _ *Context) bool {
				return r.Lte("x", 1) && r.Lte("y", 2)
			},
			N: map[string]int{"y": 0},
		}).AddAO(add("F"))
	})
	j.Rule("bar", func(rs *RuleSpec, _ *Parser) {
		// (x <= 1, y <= 0) -> pass
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{BT}},
			C: func(r *Rule, _ *Context) bool {
				return r.Lte("x", 1) && r.Lte("y", 0)
			},
		}).AddAO(add("B"))
	})

	acc = ""
	if _, err := j.Parse("fb"); err != nil {
		t.Fatalf("Parse(fb) unexpected error: %v", err)
	}
	if acc != "TFB" {
		t.Errorf("acc: got %q, want %q", acc, "TFB")
	}

	j.Rule("bar", func(rs *RuleSpec, _ *Parser) {
		// !(x <= 0) -> fail
		rs.Clear().
			AddOpen(&AltSpec{
				S: [][]Tin{{BT}},
				C: func(r *Rule, _ *Context) bool { return r.Lte("x", 0) },
			}).AddAO(add("B"))
	})

	parseErrContains(t, j, "fb", "unexpected")
}

// --- custom: parser-keep-propagates ---

func TestCustomParserKeepPropagates(t *testing.T) {
	parseWant(t, Make(), "a:1", map[string]any{"a": 1.0})

	j := makeNoRules(Options{
		Fixed: &FixedOptions{Token: fixedTok(map[string]string{
			"#F": "f", "#B": "b", "#Z": "z",
		})},
		Rule: &RuleOptions{Start: "top"},
	})

	FT, BT, ZT := j.Token("#F"), j.Token("#B"), j.Token("#Z")

	var out []string
	fmtv := func(v any) string {
		if v == nil {
			return "undefined"
		}
		return fmt.Sprintf("%v", v)
	}
	push := func(tag string) StateAction {
		return func(r *Rule, _ *Context) {
			out = append(out, fmt.Sprintf("%s<%s,%s>",
				tag, fmtv(r.K["color"]), fmtv(r.U["planet"])))
		}
	}

	j.Rule("top", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{
			P: "foo",
			K: map[string]any{"color": "red"},
			U: map[string]any{"planet": "mars"},
		}).
			AddAO(push("AO-TOP")).
			AddBC(push("BC-TOP"))
	})
	j.Rule("foo", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{S: [][]Tin{{FT}}, P: "bar"}).
			AddAO(push("AO-FOO")).
			AddBC(push("BC-FOO"))
	})
	j.Rule("bar", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{BT}},
			P: "zed",
			U: map[string]any{"planet": "earth"},
		}).
			AddAO(push("AO-BAR")).
			AddBC(push("BC-BAR"))
	})
	j.Rule("zed", func(rs *RuleSpec, _ *Parser) {
		rs.AddOpen(&AltSpec{
			S: [][]Tin{{ZT}},
			K: map[string]any{"color": "green"},
		}).
			AddAO(push("AO-ZED")).
			AddBC(push("BC-ZED"))
	})

	if _, err := j.Parse("fbz"); err != nil {
		t.Fatalf("Parse(fbz) unexpected error: %v", err)
	}

	want := []string{
		"AO-TOP<red,mars>",
		"AO-FOO<red,undefined>",
		"AO-BAR<red,earth>",
		"AO-ZED<green,undefined>",
		"BC-ZED<green,undefined>",
		"BC-BAR<red,earth>",
		"BC-FOO<red,undefined>",
		"BC-TOP<red,mars>",
	}
	if !reflect.DeepEqual(out, want) {
		t.Errorf("out:\n got %v\nwant %v", out, want)
	}
}
