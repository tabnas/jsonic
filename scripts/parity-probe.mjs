// parity-probe, TypeScript side. Reads one source per line (corpus escapes)
// and prints one outcome per line: ERROR:<code> or the value as JSON.
//
// Deliberately shares NO code with the Go probe beyond that convention —
// a shared renderer could hide a divergence inside itself.
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const { Jsonic } = require('../ts')

const un = (s) =>
  s.replace(/\\n/g, '\n').replace(/\\t/g, '\t').replace(/\\r/g, '\r').replace(/\\\\/g, '\\')

const file = process.argv[2]
if (!file) {
  console.error('usage: parity-probe.mjs <file-of-sources>')
  process.exit(2)
}

// Split on \n and keep EVERY line, including blank ones: the caller pastes
// go/ts output side by side, so dropping a line would misalign every row
// after it.
const lines = readFileSync(file, 'utf8').split('\n')
if ('' === lines[lines.length - 1]) lines.pop()

for (const line of lines) {
  let out
  try {
    const v = Jsonic.make().parse(un(line))
    // Render non-finite numbers as the corpus's marker strings. Neither
    // JSON encoder can express them and they disagree about how to fail
    // (Go's Marshal errors, JSON.stringify yields null), so `1e400` would
    // report DIFFER while both parsers agree the value is +Inf — a
    // renderer artifact presented as a parser divergence.
    out =
      'number' === typeof v && !Number.isFinite(v)
        ? Number.isNaN(v)
          ? '"@@NaN"'
          : 0 < v
            ? '"@@Infinity"'
            : '"@@-Infinity"'
        : JSON.stringify(v)
  } catch (e) {
    out = 'ERROR:' + (e && e.code ? e.code : String(e && e.message).split('\n')[0])
  }
  console.log(undefined === out ? 'undefined' : out)
}
