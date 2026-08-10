/* Copyright (c) 2026 Richard Rodger, MIT License */

// The cross-port TOKEN-STREAM corpus.
//
// Every other shared fixture asserts a decoded VALUE. That is one level too
// late for the defect class this project keeps hitting: both upstream bug
// reports were token-classification splits — a run lexing #TX in one port
// and #NR in the other — and the value difference was a downstream symptom.
// A value-only corpus catches those late, or not at all when two ports
// happen to coerce different tokens to the same value. Downstream carries
// its token stream as a public contract, which is why it had to keep a
// lexer shim rather than wait for a value-level fixture to notice.
//
// Render convention, shared with ts/test/lexspec.test.js and NOTHING else:
//
//	<name>;<sI>;<len>;<row>x<col>[;<val>]   space-separated, in order
//
// val is omitted for tokens whose value is not meaningful (#ZZ, fixed
// punctuation). It is the token's own JSON, so a number that lexes
// differently shows up here even when the parsed value would coincide.

package tabnasjsonic

import (
	"encoding/json"
	"fmt"
	"math"
	"path/filepath"
	"strings"
	"testing"
)

func TestLexSpec(t *testing.T) {
	rows, err := loadTSV(filepath.Join(specDir(), "lex.tsv"))
	if err != nil {
		t.Fatalf("cannot load lex.tsv: %v", err)
	}
	if len(rows) == 0 {
		t.Fatal("lex.tsv has no rows")
	}
	for _, row := range rows {
		if len(row.cols) == 1 && strings.HasPrefix(row.cols[0], "#") {
			continue
		}
		if len(row.cols) < 2 {
			t.Errorf("line %d: want 2 columns (input, tokens), got %d", row.lineNo, len(row.cols))
			continue
		}
		input, want := preprocessEscapes(row.cols[0]), row.cols[1]
		got := dumpTokens(input)
		if got != want {
			t.Errorf("line %d: token stream differs for %q\n  got:  %s\n  want: %s",
				row.lineNo, row.cols[0], got, want)
		}
	}
}

// dumpTokens lexes src with a stock jsonic instance and renders the stream.
func dumpTokens(src string) string {
	j := Make()
	lex := NewLex(src, j.Config())
	var out []string
	for i := 0; i < 500; i++ {
		tkn := lex.Next()
		if tkn == nil {
			break
		}
		out = append(out, renderToken(tkn))
		if tkn.Name == "#ZZ" || tkn.Name == "#BD" {
			break
		}
	}
	return strings.Join(out, " ")
}

func renderToken(t *Token) string {
	base := fmt.Sprintf("%s;%d;%d;%dx%d", t.Name, t.SI, len(t.Src), t.RI, t.CI)
	// #ZZ (end of source) and fixed punctuation carry no meaningful value.
	// Go gives #ZZ an empty map and TS gives it undefined, so rendering it
	// would encode a representation difference as a token difference.
	if t.Val == nil || t.Name == "#ZZ" || t.Name == "#BD" {
		return base
	}
	// Non-finite numbers have no JSON spelling and the two encoders
	// disagree about how to fail, so spell them the way the corpus does.
	if f, ok := t.Val.(float64); ok {
		switch {
		case math.IsInf(f, 1):
			return base + `;"@@Infinity"`
		case math.IsInf(f, -1):
			return base + `;"@@-Infinity"`
		case math.IsNaN(f):
			return base + `;"@@NaN"`
		}
	}
	b, err := json.Marshal(t.Val)
	if err != nil {
		return base + ";?"
	}
	return base + ";" + string(b)
}
