#!/bin/sh
# Records asciiverse/super-trek.cast, the catalog preview (100x30), with
# hub's castrec (../../hub). Run from the repo after cargo build --release.
# Seed 136, a Good game: two Klingons fall to the opening phaser volley,
# a torpedo spread takes two more, the last two go to a second volley, the
# long-range scan points the way, a warp into the next quadrant meets a
# Commander's ambush, and the chart shows what the ship has seen.
set -eu
hub=${HUB:-../../hub}
home=$(mktemp -d)
s="w3.5;sphasers;kenter;w0.8;sa;kenter;w0.8;s900;kenter;w4"
s="$s;storpedo 2 a7 b9;kenter;w5"
s="$s;sphasers;kenter;w0.8;sa;kenter;w0.8;s1000;kenter;w5"
s="$s;slrscan;kenter;w3"
s="$s;smove a g3 e5;kenter;w6"
s="$s;sphasers;kenter;w0.8;sa;kenter;w0.8;s800;kenter;w4"
s="$s;schart;kenter;w5"
CAST_COLS=100 CAST_ROWS=30 HOME="$home" XDG_CACHE_HOME="$home/cache" XDG_STATE_HOME="$home/state" \
  python3 "$hub/tools/castrec.py" asciiverse/super-trek.cast . "$s" -- \
  target/release/super-trek -s 136 regular short good 2026
rm -rf "$home"
