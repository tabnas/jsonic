/* Copyright (c) 2026 Richard Rodger, MIT License */

// The live cross-port divergence ledger.
//
// test/spec/divergent.tsv records each KNOWN split as the value each port
// actually produces. This runner asserts the `go` column; the TS runner
// (ts/test/divergent.test.js) asserts the `ts` column and the Rust runner
// (rs/tests/divergent_test.rs) the `rust` column, from the same file.
//
// Columns are read by HEADER NAME, not by position, in all three runners:
// the ledger gained its `rust` column without any of them moving an index.
//
// The property that matters: a divergence which gets FIXED fails here just
// as loudly as one that regresses, forcing the row to be deleted. Prose
// cannot do that, and this repo has the scars — go/doc/differences.md
// claimed 2.e3 and 1e999 still diverged after they were aligned, and
// claimed base-prefixed overflow was aligned before it was.

package tabnasjsonic

import (
	"encoding/json"
	"errors"
	"fmt"
	"path/filepath"
	"slices"
	"strings"
	"testing"

	support "github.com/tabnas/support/go"
)

// divergentRuntimes names every runtime column the ledger is expected to
// carry. Named rather than inferred from the header, so a column lost in
// an edit fails here instead of silently leaving a port unasserted.
var divergentRuntimes = []string{"go", "ts", "rust"}

func TestDivergentLedger(t *testing.T) {
	spec, lerr := support.LoadSpec(filepath.Join(specDir(), "divergent.tsv"), nil)
	if lerr != nil {
		t.Fatalf("cannot load divergent.tsv: %v", lerr)
	}
	if len(spec.Rows) == 0 {
		t.Fatal("divergent.tsv has no rows; if the ledger is empty, delete the file and its runners")
	}
	for _, want := range divergentRuntimes {
		if !slices.Contains(spec.Header, want) {
			t.Fatalf("divergent.tsv has no %q column (header: %s)",
				want, strings.Join(spec.Header, ", "))
		}
	}
	for _, row := range spec.Rows {
		// A `#`-leading line with no tab is a comment. LoadSpec does not
		// filter these (the TS loader gained that filter earlier from the
		// other side of the same asymmetry), and this file is heavily
		// commented by design — a ledger row without its justification is
		// useless.
		if len(row.Cols) == 1 && strings.HasPrefix(row.Cols[0], "#") {
			continue
		}
		if len(row.Cols) < len(spec.Header) {
			t.Errorf("line %d: want %d columns (%s), got %d",
				row.Line, len(spec.Header), strings.Join(spec.Header, " "), len(row.Cols))
			continue
		}
		name := row.Named("name")
		optsRaw, input, want := row.Named("opts"), row.Named("input"), row.Named("go")
		if strings.TrimSpace(row.Named("justification")) == "" {
			t.Errorf("%s: a ledger row must carry a justification", name)
		}

		j, oerr := makeFromLedgerOpts(optsRaw)
		if oerr != nil {
			t.Errorf("%s: bad opts %q: %v", name, optsRaw, oerr)
			continue
		}

		got := renderOutcome(j, preprocessEscapes(input))
		if got != want {
			t.Errorf("%s: Go side of the ledger is stale.\n  input: %q\n  got:   %s\n  want:  %s\n"+
				"If Go now AGREES with the ts column, the divergence is fixed — delete this row.",
				name, input, got, want)
		}
	}
}

// renderOutcome renders a parse as the ledger spells it: the value as
// JSON, or ERROR:<code>@<row>:<col>.
//
// The position is rendered, not optional. Two ports can agree on a code
// and disagree on where they say the error happened, and a cell that
// pinned the code alone would sit green through exactly that split. The
// register carries one such row today (string-replace-control-row).
func renderOutcome(j *Jsonic, src string) string {
	v, err := j.Parse(src)
	if err != nil {
		var je *JsonicError
		if errors.As(err, &je) {
			return fmt.Sprintf("ERROR:%s@%d:%d", je.Code, je.Row, je.Col)
		}
		return "ERROR:" + err.Error()
	}
	b, mErr := json.Marshal(v)
	if mErr != nil {
		return "ERROR:unmarshalable"
	}
	return string(b)
}

// makeFromLedgerOpts builds the instance a ledger row asks for. The ledger
// spells options as JSON so the SAME text drives both ports; only the
// handful of option shapes the ledger actually uses are supported, and an
// unknown one is an error rather than a silent stock parser — a row that
// quietly ran without its options would assert the wrong thing.
func makeFromLedgerOpts(raw string) (*Jsonic, error) {
	if raw == "-" || raw == "" {
		return Make(), nil
	}
	var spec struct {
		String *struct {
			Replace map[string]string `json:"replace"`
		} `json:"string"`
		Number *struct {
			Sep *string `json:"sep"`
		} `json:"number"`
	}
	if err := json.Unmarshal([]byte(raw), &spec); err != nil {
		return nil, err
	}
	opts := Options{}
	known := false
	if spec.String != nil && spec.String.Replace != nil {
		rep := make(map[rune]string, len(spec.String.Replace))
		for k, v := range spec.String.Replace {
			r := []rune(k)
			if len(r) != 1 {
				return nil, errors.New("string.replace key must be one rune: " + k)
			}
			rep[r[0]] = v
		}
		opts.String = &StringOptions{Replace: rep}
		known = true
	}
	if spec.Number != nil && spec.Number.Sep != nil {
		opts.Number = &NumberOptions{Sep: *spec.Number.Sep}
		known = true
	}
	if !known {
		return nil, errors.New("unsupported ledger opts (extend makeFromLedgerOpts): " + raw)
	}
	return Make(opts), nil
}
