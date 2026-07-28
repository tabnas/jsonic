package tabnasjsonic

import "testing"

// TestMapPlainOption pins the Map.Plain opt-out: by default a parsed object
// is the insertion-ordered *OrderedMap, but Map.Plain:true yields a plain
// unordered map[string]any (for consumers that track key order themselves).
func TestMapPlainOption(t *testing.T) {
	// Default — ordered node, source key order preserved.
	got, err := Make().Parse(`{b:1,a:2}`)
	if err != nil {
		t.Fatalf("parse: %v", err)
	}
	om, ok := got.(*OrderedMap)
	if !ok {
		t.Fatalf("default: want *OrderedMap, got %T", got)
	}
	if len(om.Keys) != 2 || om.Keys[0] != "b" || om.Keys[1] != "a" {
		t.Errorf("default: want source order [b a], got %v", om.Keys)
	}

	// Plain:true — a plain unordered map[string]any (opt-out).
	tr := true
	got2, err := Make(Options{Map: &MapOptions{Plain: &tr}}).Parse(`{b:1,a:2}`)
	if err != nil {
		t.Fatalf("parse (plain): %v", err)
	}
	m, ok := got2.(map[string]any)
	if !ok {
		t.Fatalf("Plain:true: want map[string]any, got %T", got2)
	}
	if len(m) != 2 {
		t.Errorf("Plain:true: want 2 keys, got %v", m)
	}
	if _, ok := m["a"]; !ok {
		t.Errorf("Plain:true: missing key a: %v", m)
	}
	if _, ok := m["b"]; !ok {
		t.Errorf("Plain:true: missing key b: %v", m)
	}
}
