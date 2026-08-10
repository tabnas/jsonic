/* Copyright (c) 2026 Richard Rodger, MIT License */

// Every shared fixture must be registered in BOTH runners.
//
// test/spec/ looked like a cross-port corpus and largely was not: 39 of its
// 64 files were referenced by neither runner, and one (alignment-safe-key)
// by Go alone — so it was not a parity fixture at all despite living in the
// shared directory. A corpus nothing executes reports green while measuring
// nothing, which is the failure mode the whole directory exists to prevent.
//
// This gate makes that a build failure. Adding a .tsv without wiring it into
// both ports now fails here rather than sitting inert.
//
// Fixtures needing a bespoke runner (extra columns, non-parse semantics) are
// listed in bespokeShape with the reason. That list is the ONLY escape, and
// it is asserted to be accurate: an entry that is in fact standard-shaped
// fails too, so the exemption cannot outlive its justification.

package tabnasjsonic

import (
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"testing"
)

// Fixtures whose column shape the generic runParserTSV cannot express.
// Each is run by its own dedicated test; the value is the reason.
var bespokeShape = map[string]string{
	"feature-list-child":           "3 cols: input, expected_array, expected_child",
	"feature-list-child-deep":      "3 cols: input, expected_array, expected_child",
	"feature-list-child-pair":      "3 cols: input, expected_array, expected_child",
	"feature-list-child-pair-deep": "3 cols: input, expected_array, expected_child",
	"lex": "2 cols: input, token stream — asserts TOKENS not values, so it " +
		"is run by go/lexspec_test.go and ts/test/lexspec.test.js, not runParserTSV",
	"divergent": "6 cols: name, opts, input, go, ts, justification — the " +
		"parity-debt ledger; each port asserts its OWN column, so it is run by " +
		"go/divergent_test.go and ts/test/divergent.test.js, not runParserTSV",
	"utility-deep":      "5 cols: arg1..arg4, expected — util.deep, not a parse",
	"utility-modlist":   "3 cols: input, opts, expected — util.modlist, not a parse",
	"utility-str":       "3 cols: input, maxlen, expected — util.str, not a parse",
	"utility-strinject": "3 cols: template, values, expected — util.strinject, not a parse",
}

func TestSpecFixturesRegisteredInBothRunners(t *testing.T) {
	specs, err := filepath.Glob(filepath.Join(specDir(), "*.tsv"))
	if err != nil || len(specs) == 0 {
		t.Fatalf("cannot list %s: %v", specDir(), err)
	}

	goSrc := mustRead(t, "alignment_test.go") + mustRead(t, "feature_tsv_test.go")
	tsSrc := mustRead(t, filepath.Join("..", "ts", "test", "alignment.test.js"))

	for _, p := range specs {
		name := strings.TrimSuffix(filepath.Base(p), ".tsv")
		if reason, ok := bespokeShape[name]; ok {
			// The exemption must still be true: a standard 2-column
			// input/expected file has no business being exempt.
			if hdr := header(t, p); hdr == "input\texpected" {
				t.Errorf("%s is exempt as %q but has the standard shape — "+
					"remove it from bespokeShape and register it", name, reason)
			}
			continue
		}
		if !strings.Contains(goSrc, `"`+name+`.tsv"`) {
			t.Errorf("%s.tsv is not registered in the Go runner "+
				"(add a runParserTSV call, or a bespokeShape entry saying why not)", name)
		}
		if !strings.Contains(tsSrc, `'`+name+`'`) {
			t.Errorf("%s.tsv is not registered in the TS runner "+
				"(ts/test/alignment.test.js) — a fixture only one port runs is "+
				"not a parity fixture", name)
		}
	}
}

func mustRead(t *testing.T, rel string) string {
	t.Helper()
	b, err := os.ReadFile(rel)
	if err != nil {
		t.Fatalf("cannot read %s: %v", rel, err)
	}
	return string(b)
}

var firstLine = regexp.MustCompile(`^[^\r\n]*`)

func header(t *testing.T, path string) string {
	t.Helper()
	b, err := os.ReadFile(path)
	if err != nil {
		t.Fatalf("cannot read %s: %v", path, err)
	}
	return firstLine.FindString(string(b))
}
