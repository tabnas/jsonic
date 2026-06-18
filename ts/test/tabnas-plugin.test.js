/* Copyright (c) 2024-2026 Richard Rodger and other contributors, MIT License */
'use strict'

// The relaxed-JSON grammar is shipped as an idiomatic `tabnas` plugin:
// `new Tabnas().use(jsonic)`. These tests pin that contract — the thing
// other grammar plugins depend on — independently of the legacy callable
// `Jsonic` API.

const { describe, it } = require('node:test')
const assert = require('node:assert')

const {
  Tabnas,
  jsonic,
  registerJsonicGrammar,
} = require('..')

describe('tabnas-plugin', function () {
  it('use-on-bare-engine', () => {
    const p = new Tabnas().use(jsonic)

    // Relaxed JSON: unquoted keys, implicit map, lists, comments, dive.
    assert.deepEqual(p.parse('a:1,b:[x,y,z],c:{d:e} // tail'), {
      a: 1,
      b: ['x', 'y', 'z'],
      c: { d: 'e' },
    })
    assert.deepEqual(p.parse('a:b:c:1'), { a: { b: { c: 1 } } })

    // Plain JSON still works.
    assert.deepEqual(p.parse('{"a":1,"b":[2,3]}'), { a: 1, b: [2, 3] })
  })

  it('applies-jsonic-branding', () => {
    // The engine ships the relaxed lexer defaults; the plugin layers on
    // jsonic's own error identity and hints.
    const p = new Tabnas().use(jsonic)
    assert.equal(p.options.errmsg.name, 'jsonic')
    assert.equal(p.options.errmsg.link, 'https://github.com/tabnas/jsonic')
  })

  it('plugins-at-construction', () => {
    const p = new Tabnas({ plugins: [jsonic] })
    assert.deepEqual(p.parse('x:1 y:2'), { x: 1, y: 2 })
  })

  it('layered-dependency', () => {
    // A second plugin builds on the grammar jsonic registered — the
    // "play nice as a dependency" use case. It adds a keyword value on
    // top of jsonic's map/value rules. Register jsonic first.
    function yesno(tn) {
      tn.options({ value: { def: { yes: { val: true }, no: { val: false } } } })
    }
    const p = new Tabnas().use(jsonic).use(yesno)
    assert.deepEqual(p.parse('a:yes,b:no,c:[yes,no]'), {
      a: true,
      b: false,
      c: [true, false],
    })
  })

  it('register-grammar-only', () => {
    // The grammar-only helper installs rules without re-applying options,
    // for plugins that manage their own option set.
    const p = new Tabnas()
    registerJsonicGrammar(p)
    assert.deepEqual(p.parse('a:1,b:2'), { a: 1, b: 2 })
  })

  it('plugin-name-is-jsonic', () => {
    // The plugin's function name drives its options namespace key.
    assert.equal(jsonic.name, 'jsonic')
  })
})
