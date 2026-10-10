#!/bin/sh
# Check actual generated enemy counts and elapsed time, including Sandbox.
python3 - 2>&1 <<'PY'
import contextlib
import io
import math
from pathlib import Path

source = Path('../sst')
code = compile(source.read_text(), str(source), 'exec')
for line in Path('test69.log').read_text().splitlines():
    if line.startswith('# seed '):
        seed = int(line.split()[2])
    if not line or line.startswith('#'):
        continue
    skill, length, year = line.split()
    ns = {'__name__': 'sst_resource_test'}
    exec(code, ns)
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    ns['rnd'] = ns['randomizer']()
    ns['scanner'] = ns['sstscanner']()
    ns['logfp'] = None
    ns['replayfp'] = io.StringIO('captain\n')
    game.options = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']
    ns['rnd'].seed(seed)
    ns['scanner'].inqueue = ['regular', length, skill, year]
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
    expected_time = {'short': 7, 'medium': 14, 'long': 28, 'sandbox': 9999}[length]
    total = game.remkl()
    assert total == game.inkling + game.incom + game.inscom
    assert game.state.remtime == expected_time
    initial_resources = game.state.remres
    assert game.inresor == initial_resources
    game.recompute()
    assert math.isclose(game.state.remtime, expected_time, abs_tol=1e-9)
    date = game.state.date
    game.optime = 0.01
    with contextlib.redirect_stdout(io.StringIO()):
        ns['events']()
    assert not game.alldone
    assert game.remkl() == total, 'Enemies changed during the timing check'
    assert math.isclose(game.state.date - date, 0.01, abs_tol=1e-9)
    if game.sandbox:
        assert game.state.remtime == 9999
        print(f'{line}: deadline remains unlimited.')
    else:
        assert math.isclose(game.state.remtime, expected_time - 0.01, abs_tol=1e-9)
        assert math.isclose(game.state.remres / initial_resources,
                            (expected_time - 0.01) / expected_time, abs_tol=1e-9)
        print(f'{line}: {expected_time:.2f} -> {game.state.remtime:.2f} stardates.')
PY
