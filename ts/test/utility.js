const { readFileSync, existsSync } = require('fs')
const { join } = require('path')

// Decode the escapes the fixture format allows in the INPUT column. `\t` is
// included to match the Go loader's preprocessEscapes and the format
// documented in test/AGENTS.md; without it a `\t` row silently fed the
// parser a literal backslash-t.
function unescape(str) {
  return str.replace(/\\r\\n|\\n|\\r|\\t/g, (m) => {
    if (m === '\\r\\n') return '\r\n'
    if (m === '\\n') return '\n'
    if (m === '\\r') return '\r'
    if (m === '\\t') return '\t'
    return m
  })
}

function loadTSV(name) {
  // The fixtures live at the repo root (test/spec), above both runtimes, so
  // go/ runs the same files — the @tabnas/parser convention.
  const specPath = join(__dirname, '..', '..', 'test', 'spec', name + '.tsv')

  if (!existsSync(specPath)) {
    throw new Error('spec file not found: ' + specPath)
  }

  const lines = readFileSync(specPath, 'utf8').split(/\r?\n/).filter(Boolean)
  return lines
    .slice(1)
    .map((line, i) => {
      // Only the INPUT column is escape-decoded. The Go runner already works
      // this way (preprocessEscapes on cols[0], parseExpected on the raw
      // cols[1]), and test/AGENTS.md documents it as the format — but this
      // loader used to decode EVERY column, so an `expected` value holding a
      // JSON string escape (`"ab\n😀"`) was turned into a raw control
      // character and failed JSON.parse. Decoding the input alone makes the
      // two runtimes read the same file the same way.
      const cols = line.split('\t')
      if (0 < cols.length) {
        cols[0] = unescape(cols[0])
      }
      return { cols, row: i + 1 }
    })
    // A `#`-leading line with no tab is a comment. The Go runners already
    // skip these (any row with fewer than two columns is ignored there),
    // so without this filter a documented fixture ran clean in Go and
    // crashed the TS runner on JSON.parse(undefined) — the two loaders
    // must agree on what a row IS before the rows can pin agreement on
    // anything else. A `#`-leading source with a tab is still data, so a
    // C-preprocessor-style input column works.
    .filter((e) => !(1 === e.cols.length && e.cols[0].startsWith('#')))
}

module.exports = { loadTSV }
