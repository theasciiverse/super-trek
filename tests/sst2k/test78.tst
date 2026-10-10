#!/bin/sh
# Fail promptly on crashes or hangs and retain the complete CLI transcript.
python3 - 2>&1 <<'PY'
import subprocess
import sys

result = subprocess.run(['../sst', '-r', 'test78.log'], capture_output=True,
                        text=True, timeout=15, check=True)
assert not result.stderr, result.stderr
sys.stdout.write(result.stdout)
PY
