/* Copyright (c) 2026 Richard Rodger, MIT License */
'use strict'

// map.ordered — the TS mirror of the Go port's OrderedMap.
//
// Plain JS objects enumerate integer-like keys in ascending numeric order
// no matter the order the user wrote them — a language semantic, not a
// bug — so `{2:9,1:8}` irrecoverably loses its 2-before-1 key order at
// object creation, while the Go port's *jsonic.OrderedMap preserves it
// for every key. `map: { ordered: true }` records insertion order in a
// non-enumerable side channel read by `keyOrder`, restoring cross-port
// parity without changing the plain-object result shape.
//
// The TABLE below is duplicated verbatim in go/ordered_test.go, asserting
// OrderedMap.Keys — the same inputs must report the same key order in
// both ports. (A shared TSV cannot pin this: the comparison there is a
// JSON round-trip, and JSON.stringify re-loses integer-key order — the
// very thing under test.)

const { describe, it } = require('node:test')
const assert = require('node:assert')

const { Jsonic, keyOrder } = require('..')

// KEEP IN SYNC with go/ordered_test.go orderedCases.
const TABLE = [
  { src: '{2:9, 1:8}', order: ['2', '1'] },
  { src: '{10:a, 2:b, x:c}', order: ['10', '2', 'x'] },
  { src: '{a:1, 2:b, a:3}', order: ['a', '2'] }, // repeated key keeps first position
  { src: '{zz:1, 0:2, aa:3}', order: ['zz', '0', 'aa'] },
]

describe('ordered', () => {
  const jo = Jsonic.make({ map: { ordered: true } })

  it('keyOrder reports source insertion order (cross-port table)', () => {
    for (const { src, order } of TABLE) {
      assert.deepStrictEqual(keyOrder(jo.parse(src)), order, src)
    }
  })

  it('nested and deep-merged maps record order too', () => {
    assert.deepStrictEqual(keyOrder(jo.parse('a:b:1,a:c:2').a), ['b', 'c'])
    assert.deepStrictEqual(keyOrder(jo.parse('{9:{z:1}, 2:x}')['9']), ['z'])
  })

  it('the result is still a plain object — no consumer breakage', () => {
    const r = jo.parse('{2:9, 1:8}')
    assert.strictEqual(Object.getPrototypeOf(r), null)
    assert.deepStrictEqual(JSON.parse(JSON.stringify(r)), { 1: 8, 2: 9 })
    // The record is invisible to enumeration.
    assert.deepStrictEqual(Object.keys(r), ['1', '2'])
  })

  it('plain-object mode DOCUMENTED LIMITATION: integer-like key order is lost', () => {
    // Without map.ordered there is no record, and keyOrder falls back to
    // Object.keys — ascending integer order, regardless of the source.
    // This is JS enumeration semantics; the plain representation cannot
    // carry the information. Pinned so the limitation stays documented.
    const r = Jsonic.make().parse('{2:9, 1:8}')
    assert.deepStrictEqual(keyOrder(r), ['1', '2'])
  })
})
