#!/bin/sh
# Replay mode stops after one game; feed stdin to exercise the play-again loop.
sed '/^#/d' test61.log | python3 -u ../sst -t -s 1 2>&1
printf 'Exit status: %s\n' "$?"
