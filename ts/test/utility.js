/* Copyright (c) 2013-2026 Richard Rodger and other contributors, MIT License */

// The TSV fixture loader, now a thin shim over @tabnas/support.
//
// The fixtures live at the repo root (`test/spec`), above both runtimes, so
// `go/` runs the same files — and both runtimes now read them with the same
// loader, in two languages written to behave identically. That is what this
// repo's own history argues for: its loader used to decode escapes in EVERY
// column, so an `expected` value holding a JSON string escape ("ab\n😀")
// became a raw control character and failed JSON.parse; and it used to keep
// `#`-leading comment lines that the Go runners dropped, so a documented
// fixture ran clean in Go and crashed here. Both were fixed by hand, twice.
// There is one loader now.
//
// The shape returned is unchanged — `{ cols, row }`, with only the INPUT
// column escape-decoded — so the eight suites that call this need no edit.
// Two things do change, both improvements: `row` is the PHYSICAL line
// number rather than an index among non-blank lines, so a failure points an
// editor at the offending row; and `\\` is decoded, so a fixture can carry
// a literal backslash.

const { join } = require('path')

const { findSpecDir, loadSpec } = require('@tabnas/support')

const SPEC = findSpecDir(__dirname)

function loadTSV(name) {
  const spec = loadSpec(join(SPEC, name + '.tsv'))

  return spec.rows.map((row) => ({
    // Only the input column is decoded. The expected column is raw JSON,
    // which carries its own escape rules and must not be decoded twice.
    cols: [row.unesc(0), ...row.cols.slice(1)],
    row: row.line,
  }))
}

module.exports = { loadTSV }
