// Copyright (c) 2013-2026 Richard Rodger, MIT License

package tabnasjsonic

// unprintable.go aligns jsonic's string error codes with the TypeScript
// runtime. The TS lexer reports a raw control character (code point below
// 32) inside a quoted string as `unprintable`, positioned at the offending
// character; the Go engine's string matcher instead abandons the scan and
// reports `unterminated_string` at the string start. Rather than forking
// the engine, jsonic installs a custom lex matcher just before the
// engine's string matcher: it pre-scans a quoted string and, when the
// first problem the TS lexer would hit is a raw control character, emits
// the `unprintable` error itself. In every other case it emits nothing
// and defers to the engine's string matcher, so values, escapes, and the
// other string error codes (unterminated_string, invalid_unicode,
// invalid_ascii, unexpected) are untouched.
//
// The matcher reads the source only through the lexer's window, so that a
// grammar built on jsonic may declare lex.window (admin ADR-30): in a parse
// from a reader l.Src is the window, which may end before the string does,
// and a scan that reaches the window's end asks it to fill (Lex.Fill) and
// carries on, so that it finds what the whole text shows it. In a whole
// text a fill only answers.

import (
	"strconv"
	"strings"
	"unicode/utf8"
)

// unprintableOrder places the matcher between the engine's line (4e6) and
// string (5e6) matchers, so it inspects a quoted string immediately before
// the engine's string matcher would consume it.
const unprintableOrder = 4_500_000

// escapeReach is how many bytes escapeLen reads from the byte after an
// escape character: the designator and the longest payload, `u{` with six
// hex digits and its `}`, are nine.
const escapeReach = 9

// unprintableMatchSpecs is the lex.match entry set that jsonicOptions
// layers onto the engine defaults (alongside the jsonic error branding).
func unprintableMatchSpecs() map[string]*MatchSpec {
	return map[string]*MatchSpec{
		"jsonic$unprintable": {Order: unprintableOrder, Make: makeUnprintableMatcher},
	}
}

// makeUnprintableMatcher is the MatchSpec factory. Configuration is read
// from l.Config at match time (not captured here) so later SetOptions
// calls — quote chars, replace maps, escape settings — are honoured.
func makeUnprintableMatcher(_ *LexConfig, _ *Options) LexMatcher {
	return unprintableMatch
}

// unprintableMatch pre-scans a quoted string for a raw control character.
// It mirrors the TS string matcher's stop-dispatch order — multi-line
// newline consumption, closing quote, replace map, escape sequence, then
// control character — so the reported code and position match TS exactly.
// A nil return defers to the engine's string matcher.
func unprintableMatch(l *Lex, _ *Rule) *Token {
	cfg := l.Config
	if cfg == nil || !cfg.StringLex || cfg.StringAbandon {
		// String lexing off, or string.abandon lets other matchers try
		// on failure (TS returns undefined instead of unprintable).
		return nil
	}
	pnt := l.Cursor()
	if !held(l, pnt.SI) {
		return nil
	}
	q, qlen := utf8.DecodeRuneInString(l.Src[pnt.SI:])
	if !cfg.StringChars[q] {
		return nil
	}
	multiline := cfg.MultiChars[q]

	sI := pnt.SI + qlen
	rI := pnt.RI
	cI := pnt.CI + 1

	// l.Src is read again at each step: a fill grows the window, and the
	// text it holds with it. Positions in it do not move while a token is
	// cut.
	for held(l, sI) {
		src := l.Src
		c, csize := utf8.DecodeRuneInString(src[sI:])

		// Multi-line quotes consume raw line chars as string body.
		if multiline && c < 32 && cfg.LineChars[c] {
			if cfg.RowChars[c] {
				rI++
				cI = 1
			} else {
				// Non-row line char (e.g. '\r'): column resets without
				// starting a new row, matching the engine's body scan.
				cI = 1
			}
			sI += csize
			continue
		}

		// Closing quote before any unprintable: nothing to report.
		if c == q {
			return nil
		}

		// Replaced chars are legal string body — even control chars
		// (TS consults the replace map before the control-char class).
		if _, ok := cfg.StringReplace[c]; ok {
			sI += csize
			cI++
			continue
		}

		// Escape sequence: skip what the engine's string matcher would
		// consume; defer whenever that matcher would itself error here
		// (invalid_ascii, invalid_unicode, unexpected), so the earlier
		// error wins, as in TS.
		if c == cfg.EscapeChar {
			// The window is filled past the longest escape first, so
			// that escapeLen sees the escape whole or the source's end.
			held(l, sI+csize+escapeReach-1)
			src = l.Src
			n := escapeLen(cfg, src, sI+csize)
			if n < 0 {
				return nil
			}
			cI += 1 + utf8.RuneCountInString(src[sI+csize:sI+csize+n])
			sI += csize + n
			continue
		}

		// A raw control char in the string body: TS reports this as
		// `unprintable`, positioned at the character itself.
		if c < 32 {
			// ...unless string.allowControl is set, which TS honours by
			// taking a NON-LINE control char as ordinary string body
			// (lexer.ts: `else if (allowControl && !lineBM[cc])`). This
			// pre-scan reads StringLex and StringAbandon from l.Config at
			// match time but never read this flag, so it answered
			// `unprintable` regardless — and a grammar built on jsonic
			// could not enable an engine option TypeScript already
			// honours. Line chars are excluded for the same reason they
			// are excluded there: they stay governed by MultiChars, and
			// are handled above.
			if cfg.AllowControl && !cfg.LineChars[c] {
				sI += csize
				cI++
				continue
			}
			tkn := MakeToken("#BD", TinBD, nil, src[sI:sI+csize],
				Point{Len: len(src), Site: Site{SI: sI, RI: rI, CI: cI}})
			tkn.Why = "unprintable"
			return tkn
		}

		sI += csize
		cI++
	}

	// Unterminated string (or dangling escape at EOF): defer to the
	// engine's string matcher, which reports unterminated_string as TS
	// does when no control char was hit first.
	return nil
}

// held reports whether the text the lexer holds reaches the byte at index
// i of l.Src, filling the window first where it does not: in a parse from
// a reader the window may end before the source does. It asks for twice
// what the scan has passed since the cursor, so that a long string costs
// its length to read and not its square. In a whole text it only answers.
func held(l *Lex, i int) bool {
	if i < len(l.Src) {
		return true
	}
	passed := i - l.Cursor().SI
	l.Fill(max(passed+1, 2*passed))
	return i < len(l.Src)
}

// escapeLen returns the number of source bytes the engine's string matcher
// consumes after the escape lead character (the escape designator plus its
// payload), or -1 when that matcher would error at this escape — the
// caller then defers so the engine reports the correct earlier error.
// src holds escapeReach bytes from eI, or the rest of the source where
// that is shorter, so the answer is the one the whole text gives.
func escapeLen(cfg *LexConfig, src string, eI int) int {
	if eI >= len(src) {
		return -1 // dangling escape at EOF: unterminated_string
	}
	e := src[eI] // the engine reads a single byte here

	// Custom escape map is consulted first (mirrors the engine).
	if cfg.EscapeMap != nil {
		if _, ok := cfg.EscapeMap[string(e)]; ok {
			return 1
		}
	}

	// Removed escapes (and \x under strict mode) fall back to
	// unknown-escape handling.
	if cfg.EscapeRemoved[string(e)] || (cfg.EscapeStrict && e == 'x') {
		if cfg.AllowUnknownEscape {
			return 1
		}
		return -1 // engine errors: unexpected
	}

	switch e {
	case 'b', 'f', 'n', 'r', 't', 'v', '"', '\'', '`', '\\', '/':
		return 1
	case 'x':
		// \xHH ASCII escape.
		if eI+3 > len(src) || !isHexByte(src[eI+1]) || !isHexByte(src[eI+2]) {
			return -1 // engine errors: invalid_ascii
		}
		return 3
	case 'u':
		// \u{H...H} (1-6 hex digits) unless strict, else \uHHHH.
		if !cfg.EscapeStrict && eI+1 < len(src) && src[eI+1] == '{' {
			endI := strings.IndexByte(src[eI+2:], '}')
			if endI < 1 || endI > 6 {
				return -1 // engine errors: invalid_unicode
			}
			cc, err := strconv.ParseInt(src[eI+2:eI+2+endI], 16, 64)
			if err != nil || cc > 0x10FFFF {
				return -1
			}
			return 2 + endI + 1
		}
		if eI+5 > len(src) {
			return -1
		}
		for k := 1; k <= 4; k++ {
			if !isHexByte(src[eI+k]) {
				return -1
			}
		}
		return 5
	}

	// Unknown escape: kept when allowed (even a raw control char — the
	// escape makes it legal, as in TS), otherwise the engine errors.
	if cfg.AllowUnknownEscape {
		_, esize := utf8.DecodeRuneInString(src[eI:])
		return esize
	}
	return -1 // engine errors: unexpected
}

// isHexByte reports whether b is an ASCII hex digit.
func isHexByte(b byte) bool {
	return ('0' <= b && b <= '9') || ('a' <= b && b <= 'f') || ('A' <= b && b <= 'F')
}
