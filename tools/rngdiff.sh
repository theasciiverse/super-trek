#!/bin/sh
# Compare a recorded game between SST2K (Python) and the port: the order of
# random-number calls (the first differing call names where a module
# diverges) and the output.
#   tools/rngdiff.sh testNN [path-to-super-trek-binary]
# Needs ../../references/super-star-trek (SST2K 2.10) and python3.
set -e
here=$(cd "$(dirname "$0")/.." && pwd)
ref=$here/../../references/super-star-trek
bin=${2:-$here/target/debug/super-trek}
work=${TMPDIR:-/tmp}/super-trek-rngdiff
mkdir -p "$work"
python3 - "$ref/sst" "$work/sst-trace" <<'PY'
import sys
s = open(sys.argv[1]).read()
s = s.replace("        old_x = game.lcg_x\n",
              "        old_x = game.lcg_x\n        sys.stderr.write('RNG %r\\n' % (float(old_x),))\n", 1)
open(sys.argv[2], "w").write(s)
PY
cd "$ref/tests"
TMPDIR=$work python3 "$work/sst-trace" -r "$1.log" 2>"$work/py.rng" >"$work/py.out" || true
SUPER_TREK_FAST=1 SUPER_TREK_RNG_TRACE=1 XDG_CACHE_HOME=$work "$bin" -r "$1.log" 2>"$work/rs.rng" >"$work/rs.out" || true
echo "RNG calls: python $(wc -l <"$work/py.rng"), port $(wc -l <"$work/rs.rng")"
diff "$work/py.rng" "$work/rs.rng" | head -3 || true
diff "$work/py.out" "$work/rs.out" | head -40 || true
