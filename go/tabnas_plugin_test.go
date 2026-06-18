// Copyright (c) 2024-2026 Richard Rodger, MIT License

package tabnasjsonic_test

// The relaxed-JSON grammar is shipped as an idiomatic tabnas plugin:
// tabnas.Make().Use(jsonic.Grammar). These external tests pin that
// contract — the thing other grammar plugins depend on — independently of
// the legacy jsonic.Make() API.

import (
	"reflect"
	"strings"
	"testing"

	jsonic "github.com/tabnas/jsonic/go"
	tabnas "github.com/tabnas/parser/go"
)

func TestGrammarPluginOnBareEngine(t *testing.T) {
	j := tabnas.Make()
	if err := j.Use(jsonic.Grammar); err != nil {
		t.Fatal(err)
	}

	out, err := j.Parse("a:1,b:[x,y,z],c:{d:e} // tail")
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

	// Path diving and plain JSON also work.
	if out, _ := j.Parse("a:b:c:1"); !reflect.DeepEqual(out,
		map[string]any{"a": map[string]any{"b": map[string]any{"c": float64(1)}}}) {
		t.Errorf("path dive: got %#v", out)
	}
}

func TestGrammarPluginAppliesBranding(t *testing.T) {
	// The engine ships the relaxed lexer defaults; the plugin layers on the
	// jsonic error identity, so failures carry the [jsonic/...] tag.
	j := tabnas.Make()
	_ = j.Use(jsonic.Grammar)
	_, err := j.Parse(`"unterminated`)
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(err.Error(), "[jsonic/unterminated_string]") {
		t.Errorf("expected jsonic-branded error, got:\n%s", err.Error())
	}
}

func TestGrammarPluginLayeredDependency(t *testing.T) {
	// A second plugin builds on the grammar jsonic registered — the
	// "play nice as a dependency" use case. It adds keyword values on top
	// of jsonic's map/value rules. Register jsonic first.
	yesno := func(j *tabnas.Tabnas, _ map[string]any) error {
		j.SetOptions(tabnas.Options{
			Value: &tabnas.ValueOptions{
				Def: map[string]*tabnas.ValueDef{
					"yes": {Val: true},
					"no":  {Val: false},
				},
			},
		})
		return nil
	}

	j := tabnas.Make()
	_ = j.Use(jsonic.Grammar)
	_ = j.Use(yesno)

	out, err := j.Parse("a:yes,b:no,c:[yes,no]")
	if err != nil {
		t.Fatal(err)
	}
	want := map[string]any{
		"a": true,
		"b": false,
		"c": []any{true, false},
	}
	if !reflect.DeepEqual(out, want) {
		t.Errorf("layered parse: got %#v, want %#v", out, want)
	}
}
