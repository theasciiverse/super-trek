#!/bin/sh
# Include stderr and exit status as well as the retreat's galaxy-chart update.
python3 -u ../sst -r test71.log 2>&1
printf 'Exit status: %s\n' "$?"
