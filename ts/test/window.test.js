/* Copyright (c) 2026 Richard Rodger and other contributors, MIT License */
'use strict'

// A parse from a reader (`tn.parseReader`, admin ADR-30 and ADR-33).
//
// jsonic's constructors and its plugin do not declare `lex.window`: jsonic
// is the base the relaxed grammars build on (`new Tabnas().use(jsonic)`,
// then their own plugin), and each of them declares the window on its own
// whole parser once its own matchers read through it. In TypeScript jsonic
// installs no matcher: its tokens are the engine's built-in matchers', and
// its actions read tokens, never the source. These tests set the window on
// the engine under each parser they build, as a grammar built on jsonic
// sets it on its own, and hold the composition to it: every row of every
// fixture that parses its input parses the same from a reader, in pieces of
// 1, 3 and 64 bytes, under windows of a few UTF-16 units and the engine's
// own, as from its text; and so do long documents and tokens longer than
// the window.

const { describe, it } = require('node:test')
const assert = require('node:assert')
const Fs = require('node:fs')
const Path = require('node:path')

const { findSpecDir, loadSpec } = require('@tabnas/support')

const { Jsonic, Tabnas, jsonic } = require('..')

const SPEC = findSpecDir(__dirname)

// A reader that hands its bytes over at most `size` at a time.
function pieces(text, size) {
  const bytes = Buffer.from(text, 'utf8')
  let at = 0
  return {
    read(buf) {
      const n = Math.min(size, buf.length, bytes.length - at)
      buf.set(bytes.subarray(at, at + n))
      at += n
      return n
    },
  }
}

// The engine under a jsonic instance, with `lex.window` set, as a grammar
// built on jsonic sets it on the whole parser it makes.
function windowed(instance) {
  const tn = instance.internal().tabnas
  tn.options({ lex: { window: true } })
  return tn
}

// What a parse shows: its value, or its error's code, position, row,
// column and token text. An error's excerpt is left out: a parse from a
// reader shows a long line only as far back as the window reaches
// (tabnas/parser DIVERGENCE.md), the one difference allowed.
function seen(parse) {
  try {
    return { value: parse() }
  } catch (e) {
    if (null == e || 'function' !== typeof e.toJSON) {
      return { error: 'throw ' + (e && e.name) + ' ' + (e && e.message) }
    }
    const d = e.toJSON()
    return {
      error:
        d.code + ' at ' + d.pos + ' ' + d.row + ':' + d.col + ' ' +
        JSON.stringify(d.token.src),
    }
  }
}

const show = (o) =>
  undefined !== o.error
    ? 'error ' + o.error
    : 'value ' + require('node:util').inspect(o.value, { depth: 4 })

// The pieces a reader hands its bytes over in, and the windows (margin and
// block) it is read under besides the engine's own: a few units, so that
// every matcher meets the window's edge.
const PIECES = [1, 3, 64]
const WINDOWS = [[1, 1], [3, 2], [64, 64]]

// `input` parsed by `tn` whole, and from a reader in every pieces size,
// under the engine's window and each of WINDOWS: how many reader parses
// were compared, and where they differ.
function sameFromReader(tn, at, input, failures) {
  const whole = seen(() => tn.parse(input))
  let compared = 0
  for (const size of PIECES) {
    const reads = [['', () => tn.parseReader(pieces(input, size))]]
    for (const [margin, block] of WINDOWS) {
      reads.push([
        ', margin ' + margin + ', block ' + block,
        () =>
          tn.parseReaderWithSizes(pieces(input, size), {
            margin,
            block,
            chunk: size,
          }),
      ])
    }
    for (const [how, parse] of reads) {
      const read = seen(parse)
      compared++
      let same = read.error === whole.error
      if (same && undefined === whole.error) {
        try {
          assert.deepStrictEqual(read.value, whole.value)
        } catch (e) {
          same = false
        }
      }
      if (!same) {
        failures.push(
          at + ': ' + JSON.stringify(input) + ' from a reader in pieces of ' +
          size + how + '\n  whole:  ' + show(whole) + '\n  reader: ' + show(read),
        )
      }
    }
  }
  return compared
}

// Every fixture file whose rows are parsed, with the options its runner
// builds the parser from (alignment.test.js, feature.test.js and the
// rest); 'json' is the strict variant. lex.tsv's inputs are lexed by its
// own runner, and are parsed here by a stock parser, which reaches the
// same matchers.
const STOCK = [
  'alignment-empty',
  'alignment-errors',
  'alignment-map-merge',
  'alignment-number-prefix-separator',
  'alignment-number-text',
  'alignment-safe-key',
  'alignment-structure',
  'alignment-utf8',
  'alignment-values',
  'comma-implicit-comma',
  'comma-optional-comma',
  'feature-debug-cases',
  'feature-implicit-map',
  'feature-implicit-object',
  'feature-nested-space-pairs',
  'fv-arrays',
  'fv-comma',
  'fv-deep',
  'fv-drop-outs',
  'fv-numbers',
  'fv-subobj',
  'fv-types',
  'fv-works',
  'happy',
  'jsonic-basic-array-tree',
  'jsonic-basic-json',
  'jsonic-basic-mixed-tree',
  'jsonic-basic-object-tree',
  'jsonic-funky-keys',
  'jsonic-process-array',
  'jsonic-process-implicit-object',
  'jsonic-process-mixed-nodes',
  'jsonic-process-object-tree',
  'jsonic-process-scalars',
  'jsonic-process-text',
  'jsonic-process-whitespace',
  'lex-errors',
  'lex',
].map((name) => [name, {}])

const OPTIONED = [
  ['lex-errors', { rule: { exclude: 'jsonic,imp' } }],
  ['lex-errors', { rule: { exclude: 'jsonic,imp,comma' } }],
  ['exclude-strict-json', { rule: { exclude: 'jsonic,imp' } }],
  ['exclude-strict-json-errors', { rule: { exclude: 'jsonic,imp' } }],
  ['exclude-comma', { rule: { exclude: 'comma' } }],
  ['exclude-comma-errors', { rule: { exclude: 'comma' } }],
  ['rule-finish-errors', { rule: { finish: false } }],
  ['string-allow-control', { string: { allowControl: true } }],
  ['include-json', { rule: { include: 'json' } }],
  ['include-json-errors', { rule: { include: 'json' } }],
  ['alignment-strict-json-mode', 'json'],
  ['alignment-strict-json-mode-errors', 'json'],
  [
    'feature-comment-suffix-line',
    {
      comment: {
        def: {
          hash: { line: true, start: '#', lex: true, suffix: '@@' },
          line: { line: true, start: '//', lex: true },
          block: { line: false, start: '/*', end: '*/', lex: true },
        },
      },
    },
  ],
  [
    'feature-comment-suffix-block',
    {
      comment: {
        def: {
          hash: { line: true, start: '#', lex: true },
          line: { line: true, start: '//', lex: true },
          block: { line: false, start: '/*', end: '*/', lex: true, suffix: '!!' },
        },
      },
    },
  ],
  [
    'feature-comment-def-add',
    { comment: { def: { semi: { line: true, start: ';', lex: true } } } },
  ],
  ['feature-comment-def-tweak', { comment: { def: { hash: { eatline: true } } } }],
  ['feature-comment-def-remove', { comment: { def: { hash: null } } }],
  ['feature-comment-def-remove-errors', { comment: { def: { hash: null } } }],
  [
    'feature-comment-def-block-conv',
    { comment: { def: { hash: { line: false, start: '#', end: '@@', lex: true } } } },
  ],
  [
    'feature-comment-def-nolex-errors',
    { comment: { def: { semi: { line: true, start: ';' } } } },
  ],
  ['feature-list-pair', { list: { pair: true } }],
  ['feature-map-child', { map: { child: true } }],
  ['feature-map-child-deep', { map: { child: true }, list: { child: true } }],
  ['feature-list-child', { list: { child: true } }],
  ['feature-list-child-deep', { list: { child: true } }],
  ['feature-list-child-pair', { list: { child: true, pair: true } }],
  ['feature-list-child-pair-deep', { list: { child: true, pair: true } }],
]

// The fixtures that are not parses of their input, with why.
const EXEMPT = {
  divergent: 'each row builds its own parser from its opts column; run below',
  'utility-deep': 'util.deep, not a parse',
  'utility-modlist': 'util.modlist, not a parse',
  'utility-str': 'util.str, not a parse',
  'utility-strinject': 'util.strinject, not a parse',
}

// A long text of records, each line one, its tokens of every kind.
function records(count) {
  const out = []
  for (let i = 0; i < count; i++) {
    out.push(
      '  r' + i + ': { id: ' + i + ', name: "ñ' + i + '\\u00e9", ' +
      "tags: [a, 'b', `c`], n: -1.5e3, x: 0x1F }, # row " + i,
    )
  }
  return '{\n' + out.join('\n') + '\n}\n'
}

describe('window', () => {
  it('jsonic-declares-no-window', () => {
    // jsonic is the base the relaxed grammars build on: the window is
    // declared by each of them on its own whole parser, never here.
    const plugin = new Tabnas().use(jsonic)
    for (const [name, tn] of [
      ['Jsonic', Jsonic.internal().tabnas],
      ['Jsonic.make()', Jsonic.make().internal().tabnas],
      ["Jsonic.make('json')", Jsonic.make('json').internal().tabnas],
      ['new Tabnas().use(jsonic)', plugin],
    ]) {
      assert.ok(!tn.internal().config.lex.window, name + ' declares lex.window')
    }
  })

  it('every-fixture-parses-the-same-from-a-reader', () => {
    const runs = STOCK.concat(OPTIONED)
    const listed = new Set(runs.map(([name]) => name))
    const failures = []
    let compared = 0
    runs.forEach(([name, options], index) => {
      const tn = windowed(Jsonic.make(options))
      const spec = loadSpec(Path.join(SPEC, name + '.tsv'))
      for (const row of spec.rows) {
        compared += sameFromReader(
          tn, name + ':' + row.line, row.unescNamed('input'), failures)
      }
      if (0 === (index + 1) % 10 || index + 1 === runs.length) {
        console.log(
          'window fixtures: ' + (index + 1) + ' of ' + runs.length +
          ' files (' + Math.floor(((index + 1) * 100) / runs.length) +
          '%), ' + compared + ' reader parses',
        )
      }
    })
    assert.ok(10000 < compared, '' + compared)
    assert.deepEqual(failures, [])

    // Every fixture is read from a reader, or exempt with a reason.
    const missing = Fs.readdirSync(SPEC)
      .filter((file) => file.endsWith('.tsv'))
      .map((file) => file.slice(0, -4))
      .filter((name) => !listed.has(name) && undefined === EXEMPT[name])
    for (const name of Object.keys(EXEMPT)) {
      if (!Fs.existsSync(Path.join(SPEC, name + '.tsv'))) {
        missing.push(name + ' (exempt, but there is no such fixture)')
      }
    }
    assert.deepEqual(missing, [])
  })

  it('every-ledger-row-parses-the-same-from-a-reader', () => {
    const spec = loadSpec(Path.join(SPEC, 'divergent.tsv'))
    const failures = []
    let compared = 0
    for (const row of spec.rows) {
      const raw = row.named('opts')
      const options = '-' === raw || '' === raw ? {} : JSON.parse(raw)
      compared += sameFromReader(
        windowed(Jsonic.make(options)),
        row.named('name'),
        row.unescNamed('input'),
        failures,
      )
    }
    assert.ok(0 < compared)
    assert.deepEqual(failures, [])
  })

  it('tokens-longer-than-the-window-parse-the-same-from-a-reader', () => {
    // The engine's window reaches 64 KiB past the cursor before each
    // token: a token longer than that meets its edge in a parse from a
    // reader under the engine's own sizes, every kind of token here.
    const long = (64 << 10) + 2048
    const pad = (c, n) => c.repeat(n)
    const docs = [
      pad('t', long),
      '1' + pad('0', long),
      '[1.' + pad('5', long) + 'e3, 2]',
      '"' + pad('s', long) + '\\u{1F600}"',
      "'" + pad('\\n', long / 2) + "'",
      '`' + pad('m\n', long / 2) + '`',
      '"' + pad('a', long) + '\u0001"',
      '"' + pad('a', long),
      'a: 1 // ' + pad('c', long) + '\nb: 2',
      'a: 1 /* ' + pad('b\n', long / 2) + ' */ b: 2',
      'a: 1' + pad(' ', long) + ', b: 2',
      'a: 1' + pad('\n', long) + 'b: 2',
      '/* ' + pad('u', long),
    ]
    const tn = windowed(Jsonic.make())
    const strict = windowed(Jsonic.make('json'))
    const failures = []
    for (const [n, doc] of docs.entries()) {
      for (const parser of [tn, strict]) {
        const whole = seen(() => parser.parse(doc))
        for (const size of [4096, (64 << 10) + 3]) {
          const read = seen(() => parser.parseReader(pieces(doc, size)))
          let same = read.error === whole.error
          if (same && undefined === whole.error) {
            try {
              assert.deepStrictEqual(read.value, whole.value)
            } catch (e) {
              same = false
            }
          }
          if (!same) {
            failures.push(
              'document ' + n + ' (' + JSON.stringify(doc.slice(0, 12)) +
              '...) in pieces of ' + size + ': whole ' +
              show(whole).slice(0, 200) + ', reader ' + show(read).slice(0, 200),
            )
          }
        }
      }
    }
    assert.deepEqual(failures, [])
  })

  it('a-long-document-parses-the-same-from-a-reader', () => {
    // From a reader in pieces of 4 KiB, under the engine's window and
    // under one of 256 units, which moves on thousands of times before
    // the end; with an error at the end too, which the reader places
    // where the whole text does.
    const text = records(4000)
    assert.ok(256 << 10 < text.length, '' + text.length)
    const broken = text.slice(0, -2) + 'z: "tail\u0001"\n}\n'
    const tn = windowed(Jsonic.make())
    for (const [n, doc] of [text, broken].entries()) {
      const whole = seen(() => tn.parse(doc))
      assert.equal(undefined === whole.error, 0 === n, show(whole).slice(0, 200))
      for (const sizes of [undefined, { margin: 256, block: 256, chunk: 4096 }]) {
        const read = seen(() =>
          undefined === sizes
            ? tn.parseReader(pieces(doc, 4096))
            : tn.parseReaderWithSizes(pieces(doc, 4096), sizes),
        )
        assert.equal(read.error, whole.error)
        if (undefined === whole.error) {
          assert.deepStrictEqual(read.value, whole.value)
        }
      }
      console.log('window long document: ' + (n + 1) + ' of 2 (' + (n + 1) * 50 + '%)')
    }
  })
})
