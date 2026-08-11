// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

import (
	"reflect"
	"strings"

	support "github.com/tabnas/support/go"
)

// splitGroupTags splits a comma-separated group-tag string into trimmed,
// non-empty parts, mirroring the engine's tag parsing. Test-only helper.
func splitGroupTags(g string) []string {
	out := make([]string, 0)
	for _, part := range strings.Split(g, ",") {
		part = strings.TrimSpace(part)
		if part != "" {
			out = append(out, part)
		}
	}
	return out
}

// boolPtr / intPtr return pointers to literals, for building the
// pointer-valued option fields in tests.
func boolPtr(b bool) *bool { return &b }
func intPtr(i int) *int    { return &i }

// asMap returns the key→value map for a parsed object node, accepting both
// the parser's insertion-ordered *OrderedMap and a plain map[string]any (as
// engine merge helpers like Deep may still return when neither input is
// ordered). It replaces the bare `x.(map[string]any)` assertion that now
// fails on an ordered object node. Returns nil for any other value.
func asMap(v any) map[string]any {
	if om, ok := v.(*OrderedMap); ok {
		return om.Vals
	}
	if m, ok := v.(map[string]any); ok {
		return m
	}
	return nil
}

// asMapOK is the two-value form of asMap: it reports whether v was an object
// node (either *OrderedMap or map[string]any) alongside its underlying map.
func asMapOK(v any) (map[string]any, bool) {
	if om, ok := v.(*OrderedMap); ok {
		return om.Vals, true
	}
	m, ok := v.(map[string]any)
	return m, ok
}

// plainDeep recursively rewrites every *OrderedMap object node into a plain
// map[string]any (dropping insertion order), recursing through slices and
// through ListRef/MapRef wrappers so nested ordered nodes are flattened too.
// It normalizes a parse result before a reflect.DeepEqual value comparison
// against a plain-map expected value: the switch from a bare map to an
// ordered object node changes only the container TYPE, not the values, and
// the resulting key order is verified separately where it matters. Non-object
// values pass through unchanged.
func plainDeep(v any) any {
	switch node := v.(type) {
	case *OrderedMap:
		m := make(map[string]any, len(node.Keys))
		for _, k := range node.Keys {
			m[k] = plainDeep(node.Vals[k])
		}
		return m
	case map[string]any:
		m := make(map[string]any, len(node))
		for k, elem := range node {
			m[k] = plainDeep(elem)
		}
		return m
	case []any:
		out := make([]any, len(node))
		for i, elem := range node {
			out[i] = plainDeep(elem)
		}
		return out
	case ListRef:
		out := make([]any, len(node.Val))
		for i, elem := range node.Val {
			out[i] = plainDeep(elem)
		}
		node.Val = out
		return node
	case MapRef:
		m := make(map[string]any, len(node.Val))
		for k, elem := range node.Val {
			m[k] = plainDeep(elem)
		}
		node.Val = m
		return node
	default:
		return v
	}
}

// deepEqualPlain reports whether got and want are equal once any ordered
// object nodes on either side are flattened to plain maps (order-agnostic).
func deepEqualPlain(got, want any) bool {
	return reflect.DeepEqual(plainDeep(got), plainDeep(want))
}

// preprocessEscapes decodes the escapes the shared fixture format allows
// in the input column, so the parser receives the real control characters.
// It is the shared codec now — same rules, same two languages — and it
// additionally decodes \\, which the hand-written version did not.
// Test-only helper.
func preprocessEscapes(s string) string {
	return support.Unescape(s)
}
