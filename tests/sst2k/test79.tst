#!/bin/sh
python3 - 2>&1 <<'PY'
import math
from pathlib import Path
import subprocess

source = Path('../sst')
ns = {'__name__': 'sst_finite_input_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
scanner = ns['sstscanner']()
counts = {'accept': 0, 'reject': 0}
for line in Path('test79.log').read_text().splitlines():
    if line.startswith('# seed '):
        seed = int(line.split()[2])
    if not line or line.startswith('#'):
        continue
    mode, token, *expected = line.split()
    scanner.inqueue = [token, '0.25', '\n']
    kind = scanner.nexttok()
    if mode == 'reject':
        assert kind == scanner.type == 'IHALPHA', f'Accepted {token} as numeric'
        assert scanner.real is None
        assert scanner.token == token.lower()
    else:
        assert kind == scanner.type == 'IHREAL'
        assert math.isfinite(scanner.real) and scanner.real == float(expected[0])
        assert math.copysign(1, scanner.real) == math.copysign(1, float(expected[0]))
    assert scanner.nexttok() == 'IHREAL' and scanner.real == 0.25
    assert scanner.nexttok() == 'IHEOL'
    counts[mode] += 1
print(f'{counts["reject"]} nonfinite spellings rejected; following input remains readable.')
print(f'{counts["accept"]} finite values preserved, including signed zero and the float limit.')

for token in ('nan', 'inf', '1e999'):
    result = subprocess.run(['../sst', '-t', '-s', str(seed), 'tournament', token],
                            input='1\nshort good 2026\nreport\nquit\nn\nn\n',
                            capture_output=True, text=True, timeout=15, check=True)
    assert not result.stderr, result.stderr
    assert 'Beg your pardon, Captain?' in result.stdout
    assert 'This is tournament game 1.' in result.stdout
    assert 'May the Great Bird of the Galaxy' in result.stdout
print('Tournament prompt rejects nonfinite IDs and accepts a replacement.')

result = subprocess.run(['../sst', '-t', '-s', str(seed), 'tournament', 'nan'],
                        input='', capture_output=True, text=True, timeout=15, check=True)
assert not result.stderr, result.stderr
print('EOF at the tournament-number retry exits cleanly.')
PY
