#!/bin/sh
# Regenerate assets/sst.doc, SST2K's on-line help file, from ESR's DocBook
# source, the same way upstream's doc/Makefile does:
#
#   xmlto -m sst-layer.xsl txt sst-doc.xml    (DocBook XSL to HTML, w3m -dump)
#   ./makehelp.py >sst.doc
#
# Usage: tools/make-sst-doc.sh [SST2K-doc-dir]
#   SST2K-doc-dir defaults to ../../references/super-star-trek/doc, the
#   upstream checkout's doc/ folder (sst-doc.xml, sst-layer.xsl, makehelp.py).
#
# Needs xsltproc, the DocBook XSL stylesheets (docbook-xsl), xmlto, w3m and
# python3. Without root, xmlto and w3m can be unpacked into a scratch folder:
#   apt-get download xmlto w3m && for d in *.deb; do dpkg -x "$d" root; done
#   PATH=$PWD/root/usr/bin:$PATH FORMAT_DIR=$PWD/root/usr/share/xmlto/format \
#     tools/make-sst-doc.sh
# (FORMAT_DIR tells an unpacked xmlto where its format scripts are.)
#
# Three steps beyond upstream's, all mechanical:
#
# 1. The CLOAK entry's "Mnemonic:" line is indented in sst-doc.xml, which
#    defeats makehelp.py's title removal and left the section title "Cloak"
#    at the end of the PHASERS entry; that one line is unindented first.
# 2. makehelp.py was written for Python 2, where it read the UTF-8 text as
#    bytes; its fixes for non-breaking spaces, curly quotes and dashes match
#    UTF-8 byte sequences. Under Python 3 those matches fail (it would turn
#    "SRSCAN" into "RSCAN" and glue words together), so it is run unchanged
#    with its file and stdout read and written as Latin-1, which gives it the
#    Python 2 byte view again.
# 3. The result is folded to plain ASCII: table rules drawn by w3m become
#    + - |, bullets become *, and any en or em dash becomes "-".
set -eu

here=$(cd "$(dirname "$0")/.." && pwd)
src=${1:-$here/../../references/super-star-trek/doc}
src=$(cd "$src" && pwd)
out=$here/assets/sst.doc

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cp "$src/sst-doc.xml" "$src/sst-layer.xsl" "$src/makehelp.py" "$work/"
cd "$work"

xmlto --skip-validation -m sst-layer.xsl txt sst-doc.xml

# Step 1: unindent the one indented Mnemonic line (CLOAK); w3m indents it
# with non-breaking spaces.
python3 -I - sst-doc.txt <<'EOF'
import re, sys
p = sys.argv[1]
t = open(p, encoding="utf-8").read()
t = re.sub("^[ " + chr(0xA0) + "]+(Mnemonic:)", r"\1", t, flags=re.M)
open(p, "w", encoding="utf-8").write(t)
EOF

# Step 2: run makehelp.py with Python 2 byte semantics.
python3 -I - makehelp.py >sst.doc.raw <<'EOF'
import builtins, io, runpy, sys
_open = builtins.open
builtins.open = lambda f, m="r", *a, **k: _open(f, m, encoding="latin-1", newline="")
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding="latin-1", newline="")
runpy.run_path(sys.argv[1], run_name="__main__")
sys.stdout.flush()
EOF

# Step 3: fold what is left to ASCII, and fail if anything non-ASCII remains.
python3 -I - sst.doc.raw "$out" <<'EOF'
import sys
text = open(sys.argv[1], encoding="utf-8").read()
# Box-drawing rules and corners, bullets, en and em dashes, non-breaking
# spaces and curly quotes, by code point.
fold = {0x2500: "-", 0x2502: "|", 0x2022: "*", 0x25CF: "*",
        0x2013: "-", 0x2014: "-", 0x00A0: " ",
        0x201C: '"', 0x201D: '"', 0x2018: "'", 0x2019: "'"}
for c in (0x250C, 0x252C, 0x2510, 0x251C, 0x253C, 0x2524, 0x2514, 0x2534, 0x2518):
    fold[c] = "+"
fold = {chr(k): v for k, v in fold.items()}
text = "".join(fold.get(c, c) for c in text)
bad = sorted({c for c in text if ord(c) > 127})
if bad:
    sys.exit("non-ASCII left in sst.doc: " + " ".join("U+%04X" % ord(c) for c in bad))
open(sys.argv[2], "w", encoding="ascii", newline="\n").write(text)
EOF

echo "wrote $out"
