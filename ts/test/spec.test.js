'use strict'

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { loadTSV } = require('./utility')

describe('spec', function () {
  it('loadTSV-not-found', () => {
    assert.throws(() => loadTSV('does-not-exist'), Error,
      /spec file not found/,)
  })

  it('loadTSV-returns-rows', () => {
    const entries = loadTSV('happy')
    assert.ok(entries.length > 0)
    // `row` is the PHYSICAL line number, so a failure message points an
    // editor at the offending row: line 1 is the header, so the first data
    // row is line 2. It used to be an index among non-blank lines, which
    // drifted from the file's own numbering as soon as a fixture had a
    // blank line in it.
    assert.equal(entries[0].row, 2)
    assert.ok(Array.isArray(entries[0].cols))
    assert.deepEqual(entries[0].cols.length, 2)
  })
})
