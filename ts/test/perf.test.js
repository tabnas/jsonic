/* Copyright (c) 2026 tabnas, MIT License */
'use strict'

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { Jsonic, make } = require('..')

// Guard against a performance regression where the convenience parse
// (`Jsonic(src)` / the default export) rebuilds the expensive jsonic grammar
// on every call instead of reusing the cached module-level root instance.
// Building the grammar dominates a parse, so a rebuild-per-call convenience
// parse would be many times slower than reusing one instance.
//
// The default `Jsonic` export is already a single, lazily-created root
// instance (`make('jsonic')` is run once at module load), so this test both
// documents that contract and would FAIL if someone changed the convenience
// path to `make().parse(src)` per call.
//
// The check is machine-INDEPENDENT: it compares the convenience parse against
// reusing ONE `make()` instance on the SAME machine in the SAME run, so a
// slow CI box cannot make it flaky (both sides scale together). There is
// deliberately NO wall-clock budget.
describe('perf', function () {
  it('convenience-parse-reuses-instance', () => {
    const src = 'a:1,b:2,c:3'
    const n = 3000

    // Warm both paths so the comparison is steady-state.
    for (let i = 0; i < 200; i++) Jsonic(src)
    const j = make()
    for (let i = 0; i < 200; i++) j(src)

    const t0 = process.hrtime.bigint()
    for (let i = 0; i < n; i++) {
      assert.deepEqual(Jsonic(src), { a: 1, b: 2, c: 3 })
    }
    const conv = Number(process.hrtime.bigint() - t0)

    const t1 = process.hrtime.bigint()
    for (let i = 0; i < n; i++) {
      assert.deepEqual(j(src), { a: 1, b: 2, c: 3 })
    }
    const reuse = Number(process.hrtime.bigint() - t1)

    // A cached convenience parse is ~= instance reuse; allow 4x for GC /
    // scheduling noise. A rebuild-per-call convenience parse would be many
    // times slower here, so this catches the regression without depending on
    // absolute wall-clock speed.
    const ratio = conv / reuse
    assert.ok(
      conv <= 4 * reuse,
      `convenience parse appears to rebuild the grammar on every call: ` +
        `${n} Jsonic(src) calls took ${(conv / 1e6).toFixed(1)}ms vs ` +
        `${(reuse / 1e6).toFixed(1)}ms reusing one instance ` +
        `(ratio ${ratio.toFixed(1)}x, limit 4x). ` +
        `The default Jsonic must reuse the cached root instance.`,
    )
  })
})
