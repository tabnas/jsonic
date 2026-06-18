// Copyright (c) 2024-2026 Richard Rodger, MIT License

package tabnasjsonic

import (
	"strings"
	"testing"

	tabnas "github.com/tabnas/parser/go"
)

// Grammar installation must never panic — it returns an error instead, so a
// malformed or incompatible @tabnas/json core surfaces cleanly.

func TestBuildGrammarMissingCoreReturnsError(t *testing.T) {
	// Empty rsm: the standard-JSON core was not installed. buildGrammar must
	// return an error, not panic on an index-out-of-range.
	err := buildGrammar(map[string]*RuleSpec{}, &LexConfig{})
	if err == nil {
		t.Fatal("expected an error for a missing @tabnas/json core")
	}
	if !strings.Contains(err.Error(), "tabnas/json") {
		t.Errorf("error should mention the missing dependency, got: %v", err)
	}
}

func TestBuildGrammarShortAltsReturnsError(t *testing.T) {
	// Core rules present but with too few alternates (version drift): error,
	// not a panic.
	rsm := map[string]*RuleSpec{
		"val":  {Name: "val"},
		"map":  {Name: "map"},
		"list": {Name: "list"},
		"elem": {Name: "elem"},
	}
	if err := buildGrammar(rsm, &LexConfig{}); err == nil {
		t.Fatal("expected an error for an unexpected grammar shape")
	}
}

func TestGrammarOnBareEngineSucceeds(t *testing.T) {
	// The Grammar plugin installs the json core itself, so it succeeds on a
	// bare engine and is idempotent (re-applying returns nil, no panic).
	j := tabnas.Make()
	if err := j.Use(Grammar); err != nil {
		t.Fatalf("Grammar on a bare engine should succeed, got: %v", err)
	}
	if err := j.Use(Grammar); err != nil {
		t.Fatalf("re-applying Grammar should be a no-op error-free, got: %v", err)
	}
}
