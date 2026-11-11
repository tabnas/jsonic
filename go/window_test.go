// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

// A parse from a reader (Tabnas.ParseReader, admin ADR-30 and ADR-33).
//
// jsonic's whole parsers declare lex.window: Make and MakeJSON, so that a
// parse from a reader holds a window of the source rather than all of it.
// The Grammar plugin and Empty declare none: a host installs the plugin
// beneath matchers of its own, and a grammar built on Empty brings its
// own. Make is also the base the relaxed grammars build on, and each of
// them declares its own setting on its own whole parser, after its plugin.
// What jsonic installs reads the source only through the window, and these
// tests hold the parsers its constructors build to that, as built, with
// nothing set for the test: every row of the shared fixtures parses the
// same from a reader, handed over a few bytes at a time, as from its text;
// and so do documents whose strings are longer than the window, with what
// the string pre-scan (unprintable.go) looks for placed across the
// window's edge.
//
// Go's window sizes are fixed at 64 KiB (the engine's small-window hook is
// unexported), and before each token the window reaches 64 KiB past the
// cursor, so a token meets the window's edge only where it is longer than
// that: the long documents here are made of such tokens.

import (
	"errors"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"reflect"
	"sort"
	"strings"
	"testing"

	tabnas "github.com/tabnas/parser/go"
	support "github.com/tabnas/support/go"
)

// readerMargin is how far past the cursor the window reaches before each
// token is cut: a token longer than this meets the window's edge.
const readerMargin = 64 << 10

// pieces is a reader that hands its bytes over at most size at a time.
type pieces struct {
	text []byte
	at   int
	size int
}

func (p *pieces) Read(buf []byte) (int, error) {
	if p.at == len(p.text) {
		return 0, io.EOF
	}
	n := min(p.size, len(buf), len(p.text)-p.at)
	copy(buf, p.text[p.at:p.at+n])
	p.at += n
	return n, nil
}

// declaring holds j to declaring lex.window itself, as jsonic's whole
// parsers do (TestJsonicDeclaresTheWindow), so that a reader test runs the
// parser as its constructor built it, with nothing set for the test.
func declaring(t *testing.T, j *Jsonic) *Jsonic {
	t.Helper()
	if !j.Config().Window {
		t.Fatal("the parser does not declare lex.window")
	}
	return j
}

// outcome is what a parse shows: its value, or its error's code, position,
// row, column and token text. An error's excerpt is left out: a parse from
// a reader shows a long line only as far back as the window reaches
// (tabnas/parser DIVERGENCE.md), and that is the one difference allowed.
type outcome struct {
	value any
	err   string
}

func outcomeOf(v any, err error) outcome {
	if err == nil {
		return outcome{value: v}
	}
	var je *JsonicError
	if errors.As(err, &je) {
		return outcome{err: fmt.Sprintf("%s at %d %d:%d %q", je.Code, je.Pos, je.Row, je.Col, je.Src)}
	}
	return outcome{err: "not a JsonicError: " + err.Error()}
}

func (o outcome) String() string {
	if o.err != "" {
		return "error " + o.err
	}
	return "value " + formatValue(stripRefs(o.value))
}

// sameFromReader parses text whole and from a reader in pieces of each
// size, and reports where the two differ. It returns the number of reader
// parses compared.
func sameFromReader(t *testing.T, j *Jsonic, at, text string, sizes []int) int {
	t.Helper()
	whole := outcomeOf(j.Parse(text))
	for _, size := range sizes {
		read := outcomeOf(j.ParseReader(&pieces{text: []byte(text), size: size}))
		if read.err != whole.err || !reflect.DeepEqual(read.value, whole.value) {
			t.Errorf("%s in pieces of %d:\n  whole:  %s\n  reader: %s", at, size, whole, read)
		}
	}
	return len(sizes)
}

func comments(defs map[string]*CommentDef) func() *Jsonic {
	return func() *Jsonic { return Make(Options{Comment: &CommentOptions{Def: defs}}) }
}

func made(o Options) func() *Jsonic {
	return func() *Jsonic { return Make(o) }
}

// readerFixtures is every fixture file whose rows are parsed, each with
// the parser its runner builds (alignment_test.go, feature_tsv_test.go,
// jsonic_test.go). lex.tsv's inputs are lexed by its own runner, and are
// parsed here by a stock parser, which reaches the same matchers.
var readerFixtures = func() []struct {
	file string
	make func() *Jsonic
} {
	yes := true
	stock := func() *Jsonic { return Make() }
	type entry = struct {
		file string
		make func() *Jsonic
	}
	list := []entry{
		{"lex-errors.tsv", made(Options{Rule: &RuleOptions{Exclude: "jsonic,imp"}})},
		{"lex-errors.tsv", made(Options{Rule: &RuleOptions{Exclude: "jsonic,imp,comma"}})},
		{"exclude-strict-json.tsv", made(Options{Rule: &RuleOptions{Exclude: "jsonic,imp"}})},
		{"exclude-strict-json-errors.tsv", made(Options{Rule: &RuleOptions{Exclude: "jsonic,imp"}})},
		{"exclude-comma.tsv", made(Options{Rule: &RuleOptions{Exclude: "comma"}})},
		{"exclude-comma-errors.tsv", made(Options{Rule: &RuleOptions{Exclude: "comma"}})},
		{"rule-finish-errors.tsv", made(Options{Rule: &RuleOptions{Finish: boolPtr(false)}})},
		{"string-allow-control.tsv", made(Options{String: &StringOptions{AllowControl: boolPtr(true)}})},
		{"include-json.tsv", made(Options{Rule: &RuleOptions{Include: "json"}})},
		{"include-json-errors.tsv", made(Options{Rule: &RuleOptions{Include: "json"}})},
		{"alignment-strict-json-mode.tsv", MakeJSON},
		{"alignment-strict-json-mode-errors.tsv", MakeJSON},
		{"feature-comment-suffix-line.tsv", comments(map[string]*CommentDef{
			"hash":  {Line: boolPtr(true), Start: "#", Lex: &yes, Suffix: "@@"},
			"line":  {Line: boolPtr(true), Start: "//", Lex: &yes},
			"block": {Line: boolPtr(false), Start: "/*", End: "*/", Lex: &yes},
		})},
		{"feature-comment-suffix-block.tsv", comments(map[string]*CommentDef{
			"hash":  {Line: boolPtr(true), Start: "#", Lex: &yes},
			"line":  {Line: boolPtr(true), Start: "//", Lex: &yes},
			"block": {Line: boolPtr(false), Start: "/*", End: "*/", Lex: &yes, Suffix: "!!"},
		})},
		{"feature-comment-def-add.tsv", comments(map[string]*CommentDef{
			"semi": {Line: boolPtr(true), Start: ";", Lex: &yes},
		})},
		{"feature-comment-def-tweak.tsv", comments(map[string]*CommentDef{
			"hash": {EatLine: &yes},
		})},
		{"feature-comment-def-remove.tsv", comments(map[string]*CommentDef{"hash": nil})},
		{"feature-comment-def-remove-errors.tsv", comments(map[string]*CommentDef{"hash": nil})},
		{"feature-comment-def-block-conv.tsv", comments(map[string]*CommentDef{
			"hash": {Line: Bool(false), Start: "#", End: "@@", Lex: &yes},
		})},
		{"feature-comment-def-nolex-errors.tsv", comments(map[string]*CommentDef{
			"semi": {Line: boolPtr(true), Start: ";"},
		})},
		{"feature-list-pair.tsv", made(Options{List: &ListOptions{Pair: boolPtr(true)}})},
		{"feature-map-child.tsv", made(Options{Map: &MapOptions{Child: boolPtr(true)}})},
		{"feature-map-child-deep.tsv", made(Options{
			Map:  &MapOptions{Child: boolPtr(true)},
			List: &ListOptions{Child: boolPtr(true)},
		})},
		{"feature-list-child.tsv", made(Options{List: &ListOptions{Child: boolPtr(true)}})},
		{"feature-list-child-deep.tsv", made(Options{List: &ListOptions{Child: boolPtr(true)}})},
		{"feature-list-child-pair.tsv", made(Options{List: &ListOptions{Child: boolPtr(true), Pair: boolPtr(true)}})},
		{"feature-list-child-pair-deep.tsv", made(Options{List: &ListOptions{Child: boolPtr(true), Pair: boolPtr(true)}})},
	}
	for _, file := range []string{
		"alignment-empty.tsv",
		"alignment-errors.tsv",
		"alignment-map-merge.tsv",
		"alignment-number-prefix-separator.tsv",
		"alignment-number-text.tsv",
		"alignment-safe-key.tsv",
		"alignment-structure.tsv",
		"alignment-utf8.tsv",
		"alignment-values.tsv",
		"comma-implicit-comma.tsv",
		"comma-optional-comma.tsv",
		"feature-debug-cases.tsv",
		"feature-implicit-map.tsv",
		"feature-implicit-object.tsv",
		"feature-nested-space-pairs.tsv",
		"fv-arrays.tsv",
		"fv-comma.tsv",
		"fv-deep.tsv",
		"fv-drop-outs.tsv",
		"fv-numbers.tsv",
		"fv-subobj.tsv",
		"fv-types.tsv",
		"fv-works.tsv",
		"happy.tsv",
		"jsonic-basic-array-tree.tsv",
		"jsonic-basic-json.tsv",
		"jsonic-basic-mixed-tree.tsv",
		"jsonic-basic-object-tree.tsv",
		"jsonic-funky-keys.tsv",
		"jsonic-process-array.tsv",
		"jsonic-process-implicit-object.tsv",
		"jsonic-process-mixed-nodes.tsv",
		"jsonic-process-object-tree.tsv",
		"jsonic-process-scalars.tsv",
		"jsonic-process-text.tsv",
		"jsonic-process-whitespace.tsv",
		"lex-errors.tsv",
		"lex.tsv",
	} {
		list = append(list, entry{file, stock})
	}
	return list
}()

// The fixtures that are not parses of their input, with why.
var readerExempt = map[string]string{
	"divergent.tsv":         "each row builds its own parser from its opts column; TestReaderLedger runs it",
	"utility-deep.tsv":      "util.deep, not a parse",
	"utility-modlist.tsv":   "util.modlist, not a parse",
	"utility-str.tsv":       "util.str, not a parse",
	"utility-strinject.tsv": "util.strinject, not a parse",
}

// TestReaderFixtures: every row of every fixture that parses its input
// parses the same from a reader, in pieces of 1, 3, 64 and 4096 bytes, as
// from its text, under the parser its runner builds, which declares
// lex.window.
func TestReaderFixtures(t *testing.T) {
	listed := map[string]bool{}
	compared := 0
	for n, fixture := range readerFixtures {
		listed[fixture.file] = true
		rows, err := loadTSV(filepath.Join(specDir(), fixture.file))
		if err != nil {
			t.Fatalf("%s: %v", fixture.file, err)
		}
		j := declaring(t, fixture.make())
		for _, row := range rows {
			if len(row.cols) == 0 {
				continue
			}
			at := fmt.Sprintf("%s:%d", fixture.file, row.lineNo)
			compared += sameFromReader(t, j, at, preprocessEscapes(row.cols[0]), []int{1, 3, 64, 4096})
		}
		if (n+1)%10 == 0 || n+1 == len(readerFixtures) {
			t.Logf("reader fixtures: %d of %d files (%d%%), %d reader parses",
				n+1, len(readerFixtures), (n+1)*100/len(readerFixtures), compared)
		}
	}
	if compared < 4000 {
		t.Errorf("only %d reader parses compared", compared)
	}

	// Every fixture is in the table or exempt with a reason, so that a new
	// fixture is read from a reader too.
	paths, err := filepath.Glob(filepath.Join(specDir(), "*.tsv"))
	if err != nil {
		t.Fatal(err)
	}
	var missing []string
	for _, path := range paths {
		name := filepath.Base(path)
		if !listed[name] && readerExempt[name] == "" {
			missing = append(missing, name)
		}
	}
	for name := range readerExempt {
		if _, err := os.Stat(filepath.Join(specDir(), name)); err != nil {
			missing = append(missing, name+" (exempt, but there is no such fixture)")
		}
	}
	sort.Strings(missing)
	if len(missing) > 0 {
		t.Errorf("fixtures not read from a reader: %s", strings.Join(missing, ", "))
	}
}

// TestReaderLedger: the divergence register's rows, each under the parser
// its opts column builds, which declares lex.window, parse the same from a
// reader as from their text.
func TestReaderLedger(t *testing.T) {
	spec, err := support.LoadSpec(filepath.Join(specDir(), "divergent.tsv"), nil)
	if err != nil {
		t.Fatal(err)
	}
	compared := 0
	for _, row := range spec.Rows {
		j, err := makeFromLedgerOpts(row.Named("opts"))
		if err != nil {
			t.Fatalf("%s: %v", row.Named("name"), err)
		}
		compared += sameFromReader(t, declaring(t, j), row.Named("name"),
			preprocessEscapes(row.Named("input")), []int{1, 3, 64, 4096})
	}
	if compared == 0 {
		t.Error("the register has no rows")
	}
}

// TestReaderLongStrings: strings longer than the window, with what the
// string pre-scan looks for placed across the window's edge, parse the same
// from a reader as from their text. Each document begins with the string,
// so the edge lies readerMargin bytes past its opening quote in pieces of
// 4096 bytes, which divide it, and 3 bytes further in pieces of
// readerMargin+3; the shift moves each feature across both a byte at a
// time.
func TestReaderLongStrings(t *testing.T) {
	pad := func(n int) string { return strings.Repeat("a", n) }
	var docs []string
	for shift := -12; shift <= 12; shift++ {
		edge := readerMargin + shift
		docs = append(docs,
			// A raw control character in the string, the pre-scan's own
			// error, past an escape or plain text that holds the edge.
			`"`+pad(edge-1)+"\x01"+`b"`,
			`"`+pad(edge-6)+`\u{1F600}`+"\x02"+`"`,
			`"`+pad(edge-4)+`\x41\u0041\n\t`+"\x03"+`"`,
			// Escapes the engine refuses: the pre-scan defers to it.
			`"`+pad(edge-4)+`\u{110000}`+"\x04"+`"`,
			`"`+pad(edge-3)+`\x4`+"\x05"+`"`,
			`"`+pad(edge-3)+`\u12`+"\x06"+`"`,
			`"`+pad(edge-2)+`\u{12`+"\x07"+`"`,
			// No control character: the string's value, a closing quote
			// at the edge, and the source ending inside the string.
			`"`+pad(edge-3)+`\u{1F600}"`,
			`"`+pad(edge-1)+`"`,
			`"`+pad(edge),
			`"`+pad(edge-1)+`\`,
			// Single quotes, and a multi-line backtick string whose rows
			// and columns the pre-scan counts across the edge.
			`'`+pad(edge-1)+"\x0e'",
			"`"+strings.Repeat("ab\r\n", (edge+3)/4)+"\x10`",
		)
	}
	// A string the pre-scan reads after the window has moved on, at a
	// position the window no longer starts at.
	for shift := -1; shift <= 1; shift++ {
		docs = append(docs, "["+strings.Repeat(`"x", `, 30000)+`"`+pad(readerMargin+shift)+"\x11"+`"]`)
	}
	plain := declaring(t, Make())
	replaced := declaring(t, Make(Options{String: &StringOptions{Replace: map[rune]string{'\x01': "X"}}}))
	compared := 0
	for n, doc := range docs {
		at := fmt.Sprintf("document %d (%q...%q, %d bytes)", n, doc[:2], doc[len(doc)-4:], len(doc))
		compared += sameFromReader(t, plain, at, doc, []int{4096, readerMargin + 3})
		compared += sameFromReader(t, replaced, at+" with \\x01 replaced", doc, []int{4096})
		if (n+1)%50 == 0 || n+1 == len(docs) {
			t.Logf("reader long strings: %d of %d documents (%d%%), %d reader parses",
				n+1, len(docs), (n+1)*100/len(docs), compared)
		}
	}
}

// TestReaderLongDocument: a document of over a megabyte, its tokens of
// every kind long and short, the long ones longer than the window, parses
// the same from a reader as from its text, and so does the same document
// with an error at its end, which a parse from a reader places where the
// whole text does after the window has moved on many times.
func TestReaderLongDocument(t *testing.T) {
	long := readerMargin + 2<<10
	var b strings.Builder
	b.WriteString("{\n")
	for i := 0; i < 3000; i++ {
		fmt.Fprintf(&b, "  r%d: { id: %d, name: \"ñ%d\\u00e9\", tags: [a, 'b', `c`], n: -1.5e3, x: 0x1F }, # row %d\n", i, i, i, i)
		if i == 1000 || i == 2000 {
			fmt.Fprintf(&b, "  t%d: %s,\n", i, strings.Repeat("t", long))
			fmt.Fprintf(&b, "  n%d: 1%s,\n", i, strings.Repeat("0", long))
			fmt.Fprintf(&b, "  s%d: \"%s\",\n", i, strings.Repeat("s\\n", long/3))
			fmt.Fprintf(&b, "  m%d: `%s`,\n", i, strings.Repeat("m\n", long/2))
			fmt.Fprintf(&b, "  // %s\n", strings.Repeat("c", long))
			fmt.Fprintf(&b, "  /* %s */\n", strings.Repeat("b\n", long/2))
			fmt.Fprintf(&b, "%s\n%s", strings.Repeat(" ", long), strings.Repeat("\n", long))
		}
	}
	text := b.String() + "}\n"
	broken := b.String() + "  z: \"tail\x01\"\n}\n"
	if len(text) < 1<<20 {
		t.Fatalf("the document is %d bytes", len(text))
	}
	j := declaring(t, Make())
	if _, err := j.Parse(text); err != nil {
		t.Fatalf("the long document does not parse: %v", err)
	}
	if _, err := j.Parse(broken); err == nil || err.(*JsonicError).Code != "unprintable" {
		t.Fatalf("the broken document is not unprintable: %v", err)
	}
	sizes := []int{64, 4096, readerMargin + 3}
	sameFromReader(t, j, "the long document", text, sizes)
	t.Logf("reader long document: 1 of 2 (50%%), %d bytes", len(text))
	sameFromReader(t, j, "the long document with an error at its end", broken, sizes)
	t.Logf("reader long document: 2 of 2 (100%%)")
}

// TestReaderPreScanStandsAlone: the pre-scan finds a control character past
// the window's edge itself. Where it stops at the edge, the engine's string
// matcher, which runs after it, reads on to the edge and has the token cut
// again over a wider window, the pre-scan with it; a grammar's own
// quoted-string matcher that reads through the window and comes after the
// pre-scan does not, and stands in for one here. ASCII only, so that a
// column is a byte.
func TestReaderPreScanStandsAlone(t *testing.T) {
	quoted := func(_ *LexConfig, _ *Options) LexMatcher {
		return func(l *Lex, _ *Rule) *Token {
			pnt := l.Cursor()
			if pnt.SI >= len(l.Src) || l.Src[pnt.SI] != '"' {
				return nil
			}
			for end := pnt.SI + 1; ; end++ {
				if end >= len(l.Src) && !l.Fill(end-pnt.SI+1) {
					return nil
				}
				if l.Src[end] == '"' {
					src := l.Src[pnt.SI : end+1]
					tkn := l.Token("#ST", TinST, src[1:len(src)-1], src)
					pnt.SI += len(src)
					pnt.CI += len(src)
					return tkn
				}
			}
		}
	}
	j := declaring(t, Make(Options{Lex: &LexOptions{Match: map[string]*MatchSpec{
		"quoted": {Order: 4_800_000, Make: quoted},
	}}}))
	// The window reaches the first piece boundary at least readerMargin
	// past the string's start, which in pieces of readerMargin+3 is twice
	// that: the control character lies past it in every pieces size here.
	doc := `["` + strings.Repeat("a", 2*readerMargin+16) + "\x01" + `b", 1]`
	if _, err := j.Parse(doc); err == nil || err.(*JsonicError).Code != "unprintable" {
		t.Fatalf("the pre-scan does not run before the quoted matcher: %v", err)
	}
	sameFromReader(t, j, "a control character past the edge", doc, []int{64, 4096, readerMargin + 3})
	sameFromReader(t, j, "no control character", strings.Replace(doc, "\x01", "c", 1), []int{4096})
}

// TestJsonicDeclaresTheWindow: jsonic's whole parsers declare lex.window,
// whatever else their options set; a caller's own Lex.Window wins over it;
// and the Grammar plugin and Empty declare none, since a host installs the
// plugin beneath matchers of its own and a grammar built on Empty brings
// its own.
func TestJsonicDeclaresTheWindow(t *testing.T) {
	yes, no := true, false
	plugin := tabnas.Make()
	if err := plugin.Use(Grammar, nil); err != nil {
		t.Fatal(err)
	}
	derived, err := Make().Derive()
	if err != nil {
		t.Fatal(err)
	}
	for _, c := range []struct {
		name string
		j    *Jsonic
		want bool
	}{
		{"Make", Make(), true},
		{"Make with lex options", Make(Options{Lex: &LexOptions{Empty: &no}}), true},
		{"Make with other options", Make(Options{Number: &NumberOptions{Lex: &no}}), true},
		{"MakeJSON", MakeJSON(), true},
		{"Make().Derive()", derived, true},
		{"Make with Lex.Window false", Make(Options{Lex: &LexOptions{Window: &no}}), false},
		{"Empty", Empty(), false},
		{"Empty with Lex.Window true", Empty(Options{Lex: &LexOptions{Window: &yes}}), true},
		{"tabnas.Make().Use(Grammar)", plugin, false},
	} {
		if got := c.j.Config().Window; got != c.want {
			t.Errorf("%s: lex.window is %v, want %v", c.name, got, c.want)
		}
	}
}
