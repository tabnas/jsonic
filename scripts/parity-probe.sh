#!/usr/bin/env bash
# parity-probe — run candidate sources through BOTH ports and report whether
# they agree, in the column format test/spec/*.tsv wants.
#
# Usage:  scripts/parity-probe.sh <file-of-sources>
#         scripts/parity-probe.sh -            # read sources from stdin
#
# One source per line, using the corpus's own escapes (\n, \t, \\) so a
# multi-line source stays one line. Output, one line per candidate:
#
#   AGREE   <src>\t<value>        -> paste straight into a fixture
#   DIFFER  <src>\t<go>\t<ts>     -> a real divergence; adjudicate it, and if
#                                    it is deliberate, add it to
#                                    test/spec/divergent.tsv with a reason
#
# WHY THIS EXISTS
#
# A fixture row is only trustworthy if BOTH engines were asked. Rows written
# by hand pin whatever the author believed, and that has gone wrong here more
# than once: a base-prefix overflow row was authored as 2^72 when the literal
# is 2^76 (the parser was right, the arithmetic was not), and a "failing"
# fixture turned out to be an author comparing JSON strings where the runners
# compare parsed values. Both were caught by luck rather than by process.
#
# Downstream (boru) adopted exactly this after finding 15 of 254 rows in a
# supposed parity corpus encoded one port's behaviour only.
#
# So: probe first, paste second. Never author an expected value.
set -uo pipefail

repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
src_arg="${1:?usage: parity-probe.sh <file-of-sources|->}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

if [ "$src_arg" = "-" ]; then
  cat > "$tmp/src.txt"
else
  cp "$src_arg" "$tmp/src.txt"
fi

# Both probes render an outcome the same way — ERROR:<code> or the value as
# JSON — but through each port's OWN parser and OWN JSON encoder. Nothing is
# shared between them except the source list and that convention; a shared
# renderer would be able to hide a divergence inside itself.
( cd "$repo/go" && PARITY_PROBE_IN="$tmp/src.txt" \
    go test -run TestParityProbe -count=1 -v . >"$tmp/go.txt" 2>"$tmp/go.err" ) || {
  echo "go probe failed:" >&2; cat "$tmp/go.err" >&2; exit 1; }
# $'\t' (ANSI-C quoting), not '\t': inside plain single quotes that is a
# literal backslash-t and BRE never matches a tab, which silently yielded an
# empty probe on the first run. The length guard below caught it.
grep $'^PROBE\t' "$tmp/go.txt" | cut -f2- > "$tmp/go.out" || true

( cd "$repo/ts" && node "$repo/scripts/parity-probe.mjs" "$tmp/src.txt" > "$tmp/ts.out" ) || {
  echo "ts probe failed (is ts/ built? run: npm --prefix ts run build)" >&2; exit 1; }

golines=$(wc -l < "$tmp/go.out")
tslines=$(wc -l < "$tmp/ts.out")
srclines=$(wc -l < "$tmp/src.txt")
if [ "$golines" != "$srclines" ] || [ "$tslines" != "$srclines" ]; then
  echo "probe output length mismatch: src=$srclines go=$golines ts=$tslines" >&2
  echo "(a probe that drops or adds lines would misalign every row)" >&2
  exit 1
fi

# Compare by VALUE, not by the two renderings' bytes.
#
# Each port still renders with its OWN encoder — that is the point of the
# design above, and a shared renderer could hide a divergence inside itself.
# But the two encoders disagree about things that are not parse differences,
# and comparing their bytes reported those as divergences:
#
#   {a:"x<U+2028>y"}   go {"a":"x\u2028y"}   ts {"a":"x<U+2028>y"}
#
# Identical strings. Go's json.Marshal escapes U+2028 and U+2029 (they are
# line terminators in JavaScript, so escaping them keeps the output safe to
# embed in a script); JSON.stringify leaves them raw. A DIFFER there is an
# encoder artifact, and acting on it would have put a row in the divergence
# ledger recording a disagreement that does not exist — which is the one
# thing this probe must never cause, because the ledger is what people
# trust instead of re-measuring.
#
# So both renderings are parsed and re-encoded once, canonically, before
# comparison. A real value difference survives that; an encoder difference
# does not. Same approach as css/scripts/divergence-probe.sh.
paste -d'\t' "$tmp/src.txt" "$tmp/go.out" "$tmp/ts.out" > "$tmp/joined.tsv"

node -e '
const fs = require("fs")

// Sorted keys so object order — which is outside the value contract,
// ADR-15 — cannot register as a divergence either.
const canon = (v) => Array.isArray(v) ? v.map(canon)
  : (v && "object" === typeof v)
    ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, canon(v[k])]))
    : v

// An ERROR:<code> line is not JSON; compare those as the strings they are.
// A rendering this cannot parse is compared as a string too, rather than
// being silently called equal.
const norm = (cell) => {
  if (cell.startsWith("ERROR")) return cell
  try { return JSON.stringify(canon(JSON.parse(cell))) }
  catch { return cell }
}

for (const line of fs.readFileSync(process.argv[1], "utf8").split("\n")) {
  if (!line) continue
  const [src, go, ts] = line.split("\t")
  if (norm(go) === norm(ts)) {
    process.stdout.write("AGREE\t" + src + "\t" + go + "\n")
  } else {
    process.stdout.write("DIFFER\t" + src + "\t" + go + "\t" + ts + "\n")
  }
}
' "$tmp/joined.tsv"
