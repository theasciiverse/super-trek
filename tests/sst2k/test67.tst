#!/bin/sh
# Include stderr and exit status to catch invalid replacement-ship placement.
python3 -u ../sst -r test67.log 2>&1
printf 'Exit status: %s\n' "$?"
