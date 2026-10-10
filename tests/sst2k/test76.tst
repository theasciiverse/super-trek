#!/bin/sh
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path
import sys
import tempfile
import types

source = Path('../sst')
module = types.ModuleType('sst_mission_restart_test')
sys.modules[module.__name__] = module
ns = module.__dict__
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']
ns['rnd'] = ns['randomizer']()
ns['scanner'] = ns['sstscanner']()
ns['logfp'] = None


def fresh():
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    game.options = ns['OPTION_ALL'] & ~(ns['OPTION_CURSES'] | ns['OPTION_COLOR'])
    game.cdebug = True
    return game


def start(year):
    ns['rnd'].seed(seed)
    ns['scanner'].chew()
    ns['scanner'].inqueue = ['regular', 'short', 'good', str(year)]
    ns['replayfp'] = io.StringIO('captain\n')
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()


def score_text():
    with contextlib.redirect_stdout(io.StringIO()) as output:
        ns['score']()
    return output.getvalue()


def docking():
    game = ns['game']
    game.quadrant = copy.copy(game.state.baseq[0])
    with contextlib.redirect_stdout(io.StringIO()):
        ns['newqad']()
    # Position the Enterprise in an empty sector adjacent to the actual base.
    game.quad[game.sector.i][game.sector.j] = '.'
    adjacent = [Coord(i, j)
                for i in range(max(0, game.base.i - 1), min(ns['QUADSIZE'], game.base.i + 2))
                for j in range(max(0, game.base.j - 1), min(ns['QUADSIZE'], game.base.j + 2))
                if game.quad[i][j] == '.']
    assert adjacent
    game.sector = adjacent[0]
    game.quad[game.sector.i][game.sector.j] = 'E'
    with contextlib.redirect_stdout(io.StringIO()) as output:
        ns['dock'](True)
    assert game.condition == 'docked', 'New Enterprise could not dock'
    assert 'captured Klingons transferred' not in output.getvalue()
    assert game.brigfree == 400 and game.kcaptured == 0


for line in Path('test76.log').read_text().splitlines():
    if line.startswith('# seed '):
        seed = int(line.split()[2])
    if not line or line.startswith('#'):
        continue
    ship, capacity, free, year = line.split()
    baseline = fresh()
    start(year)
    expected_rng = ns['rnd'].getrngstate()
    expected_options = baseline.options
    expected_score = score_text()

    game = fresh()
    start(2026)
    game.ship = ship
    game.brigcapacity, game.brigfree = int(capacity), int(free)
    game.iscloaked = ship == 'E'
    game.isviolreported = True
    game.ncviol, game.kcaptured = 2, 42
    game.score, game.perdate = 9000.0, 2.5
    saved_fields = {name: getattr(game, name) for name in (
        'ship', 'brigcapacity', 'brigfree', 'iscloaked', 'isviolreported',
        'ncviol', 'kcaptured', 'score', 'perdate')}

    with tempfile.TemporaryDirectory(prefix='sst-mission-restart-') as directory:
        filename = str(Path(directory) / 'mission.trk')
        ns['scanner'].chew()
        ns['scanner'].inqueue = [filename]
        with contextlib.redirect_stdout(io.StringIO()):
            ns['freeze'](False)

        start(year)
        assert ns['game'] is game
        assert game.ship == 'E' and game.alive
        assert not game.iscloaked, 'New mission inherited the cloak'
        assert not game.isviolreported
        assert game.brigcapacity == game.brigfree == 400, 'New Enterprise inherited the old brig'
        assert game.kcaptured == game.ncviol == 0, 'New mission inherited score credits or penalties'
        assert game.score == game.perdate == 0.0
        assert game.options == expected_options and game.cdebug
        assert ns['rnd'].getrngstate() == expected_rng
        assert ns['badpoints']() == 0
        assert score_text() == expected_score
        docking()
        print(f'{ship} -> Enterprise {year}: uncloaked; empty 400-place brig; clean score; docking succeeds.')

        # The saved-game branch of setup must retain the resumed mission.
        ns['scanner'].chew()
        ns['scanner'].inqueue = ['saved', filename]
        ns['replayfp'] = None
        with contextlib.redirect_stdout(io.StringIO()):
            ns['setup']()
        game = ns['game']
        assert game.thawed
        for name, value in saved_fields.items():
            assert getattr(game, name) == value, f'Loading a saved game reset {name}'
        print(f'Saved {ship}: cloak, brig, and scoring state preserved.')
PY
