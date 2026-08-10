const { readFileSync, existsSync } = require('fs')
const { join } = require('path')

function unescape(str) {
  return str.replace(/\\r\\n|\\n|\\r/g, (m) => {
    if (m === '\\r\\n') return '\r\n'
    if (m === '\\n') return '\n'
    if (m === '\\r') return '\r'
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
      const cols = line.split('\t').map(unescape)
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
