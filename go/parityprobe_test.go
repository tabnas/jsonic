/* Copyright (c) 2026 Richard Rodger, MIT License */

// parity-probe, Go side. Driven by scripts/parity-probe.sh, which sets
// PARITY_PROBE_IN; a normal test run has no such env var and skips.
//
// Emits one PROBE-prefixed line per input source: ERROR:<code> or the value
// as JSON. The prefix survives go test's own output framing, and the shell
// wrapper strips it.
//
// Shares no rendering code with the TS probe beyond that convention — a
// shared renderer could hide a divergence inside itself.

package tabnasjsonic

import (
	"encoding/json"
	"errors"
	"fmt"
	"math"
	"os"
	"strings"
	"testing"
)

func TestParityProbe(t *testing.T) {
	in := os.Getenv("PARITY_PROBE_IN")
	if in == "" {
		t.Skip("PARITY_PROBE_IN unset — this test is a tool, driven by scripts/parity-probe.sh")
	}
	raw, err := os.ReadFile(in)
	if err != nil {
		t.Fatalf("cannot read %s: %v", in, err)
	}
	lines := strings.Split(string(raw), "\n")
	if len(lines) > 0 && lines[len(lines)-1] == "" {
		lines = lines[:len(lines)-1]
	}
	// Keep EVERY line, blank ones included: the wrapper pastes go and ts
	// output side by side, so dropping one would misalign every later row.
	for _, line := range lines {
		fmt.Printf("PROBE\t%s\n", probeOutcome(preprocessEscapes(line)))
	}
}

// nonFiniteMarker renders ±Inf/NaN as the corpus's own marker strings.
// Neither JSON encoder can express them and they disagree about how to
// fail: Go's json.Marshal errors, JS's JSON.stringify yields null. Left
// alone, `1e400` reports DIFFER when both parsers in fact agree the value
// is +Inf — a renderer artifact presented as a parser divergence, which is
// the one thing a probe must never do.
func nonFiniteMarker(v any) (string, bool) {
	f, ok := v.(float64)
	if !ok {
		return "", false
	}
	switch {
	case math.IsInf(f, 1):
		return `"@@Infinity"`, true
	case math.IsInf(f, -1):
		return `"@@-Infinity"`, true
	case math.IsNaN(f):
		return `"@@NaN"`, true
	}
	return "", false
}

func probeOutcome(src string) string {
	v, err := Make().Parse(src)
	if err != nil {
		var je *JsonicError
		if errors.As(err, &je) {
			return "ERROR:" + je.Code
		}
		return "ERROR:" + strings.SplitN(err.Error(), "\n", 2)[0]
	}
	if m, ok := nonFiniteMarker(v); ok {
		return m
	}
	b, mErr := json.Marshal(v)
	if mErr != nil {
		return "ERROR:unmarshalable"
	}
	return string(b)
}
