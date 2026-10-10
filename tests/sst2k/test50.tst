#!/bin/sh
# Include the error and exit status; unbuffered output keeps their order stable.
python3 -u ../sst -r test50.log 2>&1
printf 'Exit status: %s\n' "$?"
