// Copyright (c) 2024-2026 Richard Rodger, MIT License

package tabnasjsonic_test

// RegisterJsonicGrammar is the grammar-only registration helper (the Go
// counterpart of the TS `registerJsonicGrammar` export in ts/src/grammar.ts):
// it installs the relaxed-JSON val/map/list/pair/elem rules on an engine
// WITHOUT applying jsonic's option/error branding, so other grammar plugins
// can layer on the jsonic core while managing their own option set. These
// tests mirror ts/test/tabnas-plugin.test.js ("register-grammar-only").

import (
	"reflect"
	"strings"
	"testing"

	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

func TestRegisterJsonicGrammarOnBareEngine(t *testing.T) {
	// The grammar-only helper installs rules without re-applying options,
	// for plugins that manage their own option set.
	j := tabnas.Make()
	if err := jsonic.RegisterJsonicGrammar(j); err != nil {
		t.Fatal(err)
	}

	out, err := j.Parse("a:1,b:[x,y,z],c:{d:e}")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]any{
		"a": float64(1),
		"b": []any{"x", "y", "z"},
		"c": map[string]any{"d": "e"},
	}
	if !reflect.DeepEqual(out, want) {
		t.Errorf("relaxed parse: got %#v, want %#v", out, want)
	}

	// Registration is idempotent (decoration-guarded): a second call is a
	// no-op and must not clobber the installed rules.
	if err := jsonic.RegisterJsonicGrammar(j); err != nil {
		t.Fatal(err)
	}
	if out, _ := j.Parse("a:1,b:2"); !reflect.DeepEqual(out,
		map[string]any{"a": float64(1), "b": float64(2)}) {
		t.Errorf("parse after re-register: got %#v", out)
	}
}

func TestRegisterJsonicGrammarSkipsBranding(t *testing.T) {
	// Unlike the Grammar plugin, the grammar-only helper does not apply the
	// jsonic errmsg identity: errors keep the engine's own tag so a layering
	// plugin can brand errors itself.
	j := tabnas.Make()
	if err := jsonic.RegisterJsonicGrammar(j); err != nil {
		t.Fatal(err)
	}
	_, err := j.Parse(`"unterminated`)
	if err == nil {
		t.Fatal("expected error")
	}
	if strings.Contains(err.Error(), "[jsonic/") {
		t.Errorf("grammar-only registration must not brand errors as jsonic, got:\n%s",
			err.Error())
	}
}
