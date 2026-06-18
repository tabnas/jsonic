// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

import "strings"

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

// preprocessEscapes unescapes the \n, \r and \t sequences that appear
// literally in the shared .tsv conformance fixtures' input column, so the
// parser receives the real control characters. Test-only helper.
func preprocessEscapes(s string) string {
	if len(s) == 0 {
		return s
	}

	runes := []rune(s)
	var out []rune
	i := 0
	for i < len(runes) {
		if runes[i] == '\\' && i+1 < len(runes) {
			switch runes[i+1] {
			case 'n':
				out = append(out, '\n')
				i += 2
			case 'r':
				out = append(out, '\r')
				i += 2
			case 't':
				out = append(out, '\t')
				i += 2
			default:
				out = append(out, runes[i])
				i++
			}
		} else {
			out = append(out, runes[i])
			i++
		}
	}
	return string(out)
}
