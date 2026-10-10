#!/bin/sh
# Bound a regressed header loop and include its diagnostic and exit status.
python3 - 2>&1 <<'PY'
import subprocess
import sys

try:
    result = subprocess.run(
        [sys.executable, '-u', '../sst', '-r', 'test63.log'],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        timeout=5,
    )
except subprocess.TimeoutExpired:
    sys.exit('Replay did not reject the incomplete header within five seconds.')
sys.stdout.write(result.stdout)
print('Exit status:', result.returncode)
PY
