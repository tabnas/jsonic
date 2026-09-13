
const Fs = require('node:fs')
const Path = require('node:path')

const REPO = Path.join(__dirname, '..', '..')

// The reader-facing set. Working documents (design notes, feasibility
// reports, ledgers) are deliberately out: see "The published set" in
// docs/STYLE-GUIDE.md.
const PAGES = [
  "ts/doc/api.md",
  "ts/doc/concepts.md",
  "ts/doc/guide.md",
  "ts/doc/options.md",
  "ts/doc/plugins.md",
  "ts/doc/syntax.md",
  "ts/doc/tutorial.md",
  "go/doc/api.md",
  "go/doc/concepts.md",
  "go/doc/differences.md",
  "go/doc/guide.md",
  "go/doc/options.md",
  "go/doc/plugins.md",
  "go/doc/syntax.md",
  "go/doc/tutorial.md",
  "README.md",
  "ts/README.md",
  "go/README.md"
]

const TUTORIALS = [
  "ts/doc/tutorial.md",
  "go/doc/tutorial.md"
]


function exists(rel) {
  return Fs.existsSync(Path.join(REPO, rel))
}


// Filtered to what is actually on disk, so a renamed page fails as a
// missing gate rather than as a crash.
function gatedDocs() {
  return PAGES.filter(exists)
}


function tutorials() {
  return TUTORIALS.filter(exists)
}


module.exports = { gatedDocs, tutorials, PAGES }

if (require.main === module) {
  process.stdout.write(gatedDocs().join('\n') + '\n')
}
