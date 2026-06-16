/* Copyright (c) 2013-2022 Richard Rodger and other contributors, MIT License */
'use strict'

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { Jsonic, JsonicError } = require('..')
const { Debug } = require('@tabnas/debug')

describe('debug', function () {
  it('plugin', () => {
    let jd = Jsonic.make().use(Debug)
    assert.ok(jd.debug.describe() != null)
  })

  it('model() returns the structured grammar', () => {
    // The `.use()` shortcut installs Debug onto the jsonic instance, whose
    // engine is the tabnas parser — so debug.model() yields the structured
    // form of jsonic's relaxed-JSON grammar.
    const jd = Jsonic.make().use(Debug)
    const m = jd.debug.model()

    assert.deepEqual(
      m.rules.map((r) => r.name).sort(),
      ['elem', 'list', 'map', 'pair', 'val'],
    )
    assert.equal(m.config.start, 'val')
    assert.ok(m.plugins.some((p) => p.name === 'Debug'), 'plugins should list Debug')
    assert.equal(typeof m.abnf, 'string')

    // The grammar portion is JSON-serialisable.
    const grammar = { tokens: m.tokens, rules: m.rules, graph: m.graph, config: m.config }
    assert.deepEqual(JSON.parse(JSON.stringify(grammar)).rules, m.rules)
  })
})
