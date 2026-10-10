#!/bin/sh
# Include stderr and exit status as well as the displayed mission time.
python3 -u ../sst -r test68.log 2>&1
printf 'Exit status: %s\n' "$?"
