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

paste -d'\t' "$tmp/src.txt" "$tmp/go.out" "$tmp/ts.out" |
  while IFS=$'\t' read -r src go ts; do
    if [ "$go" = "$ts" ]; then
      printf 'AGREE\t%s\t%s\n' "$src" "$go"
    else
      printf 'DIFFER\t%s\t%s\t%s\n' "$src" "$go" "$ts"
    fi
  done
