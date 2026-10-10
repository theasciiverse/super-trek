#!/bin/sh
# Keep all stars in the target quadrant so the real RNG selects it naturally.
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path
import sys
import tempfile
import types

source = Path('../sst')
module = types.ModuleType('sst_supernova_radio_test')
sys.modules[module.__name__] = module
ns = module.__dict__
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']


def chart_text():
    with contextlib.redirect_stdout(io.StringIO()) as output:
        ns['chart']()
    return output.getvalue()


def roundtrip():
    with tempfile.TemporaryDirectory(prefix='sst-supernova-test-') as directory:
        filename = str(Path(directory) / 'game.trk')
        with contextlib.redirect_stdout(io.StringIO()):
            ns['scanner'].inqueue = [filename]
            ns['freeze'](False)
            ns['game'] = ns['Gamestate']()
            ns['game'].options = ns['OPTION_TTY']
            ns['scanner'].inqueue = [filename]
            assert not ns['thaw']()
    return ns['game']


for case in Path('test73.log').read_text().splitlines():
    if case.startswith('# seed '):
        seed = int(case.split()[2])
    if not case or case.startswith('#'):
        continue
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    ns['rnd'] = ns['randomizer']()
    ns['rnd'].seed(seed)
    ns['scanner'] = ns['sstscanner']()
    ns['logfp'] = None
    ns['replayfp'] = None
    game.options = ns['OPTION_TTY'] | ns['OPTION_ALPHAMERIC']
    game.ship, game.alive, game.energy = 'E', True, 1000
    # Keep the focused galaxy fixture, but supply a valid ship for save/load checks.
    game.skill, game.length, game.passwd = ns['SKILL_GOOD'], 1, 'captain'
    game.inshld, game.warpfac = 2500.0, 5.0
    game.quad = [['.'] * ns['QUADSIZE'] for _ in range(ns['QUADSIZE'])]
    game.condition = 'docked' if case == 'docked' else 'green'
    game.quadrant, game.sector = Coord(7, 0), Coord(4, 4)
    game.damage[ns['DRADIO']] = 0 if case == 'radio-working' else 5
    destination = Coord(1, 6)
    q = game.state.galaxy[destination.i][destination.j]
    page = game.state.chart[destination.i][destination.j]
    q.stars, q.starbase, q.klingons, q.romulans = 3, True, 2, 1
    game.state.baseq = [copy.copy(destination)]
    game.state.kcmdr = [copy.copy(destination)]
    game.state.nromrem = 1
    planet = ns['Planet']()
    planet.quadrant, planet.pclass = copy.copy(destination), 'M'
    q.planet = planet
    game.state.planets, game.inplan = [planet], 1
    # Leave another enemy alive so destruction cannot finish the game.
    game.state.galaxy[7][7].klingons = 1
    q.charted = case != 'unexplored-base'
    page.starbase = True
    if q.charted:
        page.klingons, page.stars = 2, 3
    before = chart_text()
    with contextlib.redirect_stdout(io.StringIO()) as output:
        ns['supernova'](destination if case == 'induced' else None)
    assert q.supernova, 'Radio damage prevented recording the physical supernova'
    assert not q.starbase and destination not in game.state.baseq
    assert q.planet is None and planet.pclass == 'destroyed'
    assert q.klingons == q.romulans == game.state.nromrem == 0
    assert destination not in game.state.kcmdr and game.remkl() == 1
    assert not game.alldone
    informed = case in ('radio-working', 'docked')
    assert ('Supernova in Quadrant b7' in output.getvalue()) == informed
    if informed:
        assert '***' in chart_text()
    else:
        assert chart_text() == before, 'Chart revealed an unreported supernova'
    if case == 'radio-damaged':
        game = roundtrip()
        assert game.state.galaxy[destination.i][destination.j].supernova
        assert chart_text() == before, 'Loading a save revealed an unreported supernova'
    if case == 'sensor-discovery':
        game.quadrant = Coord(2, 6)
        with contextlib.redirect_stdout(io.StringIO()) as output:
            ns['lrscan'](silent=False)
        assert '***' in output.getvalue() and '***' in chart_text()
    elif case == 'quadrant-entry':
        game.quadrant = copy.copy(destination)
        with contextlib.redirect_stdout(io.StringIO()):
            ns['newqad']()
        assert game.base is None and game.iplnet is None
        assert all(cell == '.' for row in game.quad for cell in row)
        assert '***' in chart_text()
    else:
        game.damage[ns['DRADIO']] = 0
        assert '***' in chart_text(), 'Repaired radio did not update the chart'
        game.quadrant = copy.copy(destination)
        with contextlib.redirect_stdout(io.StringIO()):
            ns['newqad']()
        assert game.base is None and game.iplnet is None
        assert all(cell == '.' for row in game.quad for cell in row)
    expected_kills = (3, 1, 1) if case == 'induced' else (0, 0, 0)
    assert (game.state.starkl, game.state.basekl, game.state.nplankl) == expected_kills
    print(f'{case}: quadrant destroyed; chart respects discovery; no base reappears.')

# Simulate saves predating the separate chart flag, including time-warp history.
game.damage[ns['DRADIO']] = 5
game.snapsht.galaxy[2][2].supernova = True
for snapshot in (game.state, game.snapsht):
    for i, j, quadrant in snapshot.traverse():
        del snapshot.chart[i][j].supernova
game = roundtrip()
for snapshot in (game.state, game.snapsht):
    for i, j, quadrant in snapshot.traverse():
        assert snapshot.chart[i][j].supernova == quadrant.supernova
assert '***' in chart_text()
print('Legacy save: known supernovae preserved in the current state and time-warp history.')
PY
