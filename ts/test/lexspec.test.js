/* Copyright (c) 2026 Richard Rodger, MIT License */
'use strict'

// The cross-port TOKEN-STREAM corpus — TypeScript side.
//
// Every other shared fixture asserts a decoded VALUE, which is one level
// too late for the defect class this project keeps hitting: both upstream
// bug reports were token-classification splits (a run lexing #TX in one
// port and #NR in the other), with the value difference only a downstream
// symptom. Downstream carries its token stream as a public contract.
//
// Render convention, shared with go/lexspec_test.go and nothing else:
//
//   <name>;<sI>;<len>;<row>x<col>[;<val>]   space-separated, in order

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { Jsonic, makeLex } = require('..')
const { loadTSV } = require('./utility')

const un = (s) =>
  s.replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\r/g, '\r').replace(/\\\\/g, '\\')

function renderToken(t) {
  const base = `${t.name};${t.sI};${t.len};${t.rI}x${t.cI}`
  // #ZZ / #BD carry no meaningful value, and the two ports represent
  // "no value" differently (Go: empty map, TS: undefined) — rendering it
  // would encode a representation difference as a token difference.
  if (undefined === t.val || '#ZZ' === t.name || '#BD' === t.name) return base
  if ('number' === typeof t.val && !Number.isFinite(t.val)) {
    return (
      base +
      ';' +
      (Number.isNaN(t.val) ? '"@@NaN"' : 0 < t.val ? '"@@Infinity"' : '"@@-Infinity"')
    )
  }
  return base + ';' + JSON.stringify(t.val)
}

function dumpTokens(src) {
  const j = Jsonic.make()
  const lex = makeLex({ src: () => src, cfg: j.internal().config, opts: j.options, sub: {} })
  const next = lex.next.bind(lex)
  const out = []
  for (let i = 0; i < 500; i++) {
    const t = next()
    if (!t) break
    out.push(renderToken(t))
    if ('#ZZ' === t.name || '#BD' === t.name) break
  }
  return out.join(' ')
}

describe('lexspec', () => {
  it('token streams match the shared corpus', () => {
    const rows = loadTSV('lex')
    assert.ok(0 < rows.length, 'lex.tsv has no rows')
    for (const { cols, row } of rows) {
      if (1 === cols.length && cols[0].startsWith('#')) continue
      assert.equal(cols.length, 2, `line ${row}: want 2 columns (input, tokens)`)
      const [input, want] = cols
      assert.equal(
        dumpTokens(un(input)),
        want,
        `line ${row}: token stream differs for ${JSON.stringify(input)}`,
      )
    }
  })
})
