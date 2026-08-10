/* Copyright (c) 2026 Richard Rodger, MIT License */

// The live cross-port divergence ledger.
//
// test/spec/divergent.tsv records each KNOWN split as the value each port
// actually produces. This runner asserts the `go` column; the TS runner
// (ts/test/divergent.test.js) asserts the `ts` column, from the same file.
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
	"path/filepath"
	"strings"
	"testing"
)

func TestDivergentLedger(t *testing.T) {
	rows, lerr := loadTSV(filepath.Join(specDir(), "divergent.tsv"))
	if lerr != nil {
		t.Fatalf("cannot load divergent.tsv: %v", lerr)
	}
	if len(rows) == 0 {
		t.Fatal("divergent.tsv has no rows; if the ledger is empty, delete the file and its runners")
	}
	for _, row := range rows {
		// A `#`-leading line with no tab is a comment. loadTSV does not
		// filter these (the TS loader gained that filter earlier from the
		// other side of the same asymmetry), and this file is heavily
		// commented by design — a ledger row without its justification is
		// useless.
		if len(row.cols) == 1 && strings.HasPrefix(row.cols[0], "#") {
			continue
		}
		if len(row.cols) < 6 {
			t.Errorf("line %d: want 6 columns (name opts input go ts justification), got %d",
				row.lineNo, len(row.cols))
			continue
		}
		name, optsRaw, input, want := row.cols[0], row.cols[1], row.cols[2], row.cols[3]
		if strings.TrimSpace(row.cols[5]) == "" {
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

// renderOutcome renders a parse as the ledger spells it: ERROR:<code> or
// the value as JSON.
func renderOutcome(j *Jsonic, src string) string {
	v, err := j.Parse(src)
	if err != nil {
		var je *JsonicError
		if errors.As(err, &je) {
			return "ERROR:" + je.Code
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
	}
	if err := json.Unmarshal([]byte(raw), &spec); err != nil {
		return nil, err
	}
	opts := Options{}
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
	} else {
		return nil, errors.New("unsupported ledger opts (extend makeFromLedgerOpts): " + raw)
	}
	return Make(opts), nil
}
