/* Copyright (c) 2026 Richard Rodger, MIT License */

// Key-order parity with the TS port's `map: { ordered: true }` mode.
//
// The Go port's *OrderedMap preserves insertion order for every key by
// construction. The TS port returns plain JS objects, which reorder
// integer-like keys ascending at creation; its opt-in ordered mode
// records insertion order in a side channel read by `keyOrder`. The
// TABLE below is duplicated verbatim in ts/test/ordered.test.js — the
// same inputs must report the same key order in both ports. (A shared
// TSV cannot pin this: its comparison is a JSON round-trip, and
// JSON.stringify re-loses integer-key order — the thing under test.)

package tabnasjsonic

import (
	"reflect"
	"testing"
)

// KEEP IN SYNC with ts/test/ordered.test.js TABLE.
var orderedCases = []struct {
	src   string
	order []string
}{
	{"{2:9, 1:8}", []string{"2", "1"}},
	{"{10:a, 2:b, x:c}", []string{"10", "2", "x"}},
	{"{a:1, 2:b, a:3}", []string{"a", "2"}}, // repeated key keeps first position
	{"{zz:1, 0:2, aa:3}", []string{"zz", "0", "aa"}},
}

func TestOrderedKeyParity(t *testing.T) {
	for _, c := range orderedCases {
		j := Make()
		v, err := j.Parse(c.src)
		if err != nil {
			t.Fatalf("Parse(%q): %v", c.src, err)
		}
		om, ok := v.(*OrderedMap)
		if !ok {
			t.Fatalf("Parse(%q): expected *OrderedMap, got %T", c.src, v)
		}
		if !reflect.DeepEqual(om.Keys, c.order) {
			t.Errorf("Parse(%q) key order = %v, want %v (cross-port table)",
				c.src, om.Keys, c.order)
		}
	}
}

func TestOrderedKeyNestedMerge(t *testing.T) {
	j := Make()
	v, err := j.Parse("a:b:1,a:c:2")
	if err != nil {
		t.Fatal(err)
	}
	a, _ := v.(*OrderedMap).Get("a")
	if got := a.(*OrderedMap).Keys; !reflect.DeepEqual(got, []string{"b", "c"}) {
		t.Errorf("merged nested key order = %v, want [b c]", got)
	}
}
