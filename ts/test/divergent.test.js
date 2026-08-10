/* Copyright (c) 2026 Richard Rodger, MIT License */
'use strict'

// The live cross-port divergence ledger — TypeScript side.
//
// test/spec/divergent.tsv records each KNOWN split as the value each port
// actually produces. This runner asserts the `ts` column; the Go runner
// (go/divergent_test.go) asserts the `go` column, from the same file.
//
// The property that matters: a divergence which gets FIXED fails here just
// as loudly as one that regresses, forcing the row to be deleted. Prose
// cannot do that — go/doc/differences.md claimed 2.e3 and 1e999 still
// diverged after they were aligned, and this very ledger caught a stale
// error code (unterminated_string, actually unprintable) on its first run.

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { Jsonic } = require('..')
const { loadTSV } = require('./utility')

// Build the instance a ledger row asks for. Options are spelled as JSON so
// the SAME text drives both ports. An unrecognised shape throws rather than
// silently yielding a stock parser: a row that ran without its options
// would assert the wrong thing while looking green.
function makeFromLedgerOpts(raw) {
  if ('-' === raw || '' === raw) return Jsonic.make()
  const spec = JSON.parse(raw)
  const known = Object.keys(spec).every((k) => 'string' === k)
  if (!known) {
    throw new Error('unsupported ledger opts (extend makeFromLedgerOpts): ' + raw)
  }
  return Jsonic.make(spec)
}

function outcome(j, src) {
  try {
    return JSON.stringify(j.parse(src))
  } catch (e) {
    return 'ERROR:' + (e.code || e.message)
  }
}

const un = (s) =>
  s.replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\r/g, '\r').replace(/\\\\/g, '\\')

describe('divergent', () => {
  it('every ledger row still diverges exactly as recorded', () => {
    const rows = loadTSV('divergent')
    assert.ok(
      0 < rows.length,
      'divergent.tsv has no rows; if the ledger is empty, delete the file and its runners',
    )
    for (const { cols, row } of rows) {
      if (1 === cols.length && cols[0].startsWith('#')) continue
      assert.equal(
        cols.length,
        6,
        `line ${row}: want 6 columns (name opts input go ts justification), got ${cols.length}`,
      )
      const [name, opts, input, , wantTs, why] = cols
      assert.ok(why && why.trim(), `${name}: a ledger row must carry a justification`)

      const got = outcome(makeFromLedgerOpts(opts), un(input))
      assert.equal(
        got,
        wantTs,
        `${name}: TS side of the ledger is stale.\n  input: ${JSON.stringify(input)}\n` +
          `  got:   ${got}\n  want:  ${wantTs}\n` +
          `If TS now AGREES with the go column, the divergence is fixed — delete this row.`,
      )
    }
  })
})
