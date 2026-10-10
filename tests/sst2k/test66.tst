#!/bin/sh
# Include stderr and exit status to catch a crash after the orbit request.
python3 -u ../sst -r test66.log 2>&1
printf 'Exit status: %s\n' "$?"
