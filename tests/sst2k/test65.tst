#!/bin/sh
# Exercise strategic movement directly so tactical retreats cannot hide a ghost.
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path

source = Path('../sst')
ns = {'__name__': 'sst_supercommander_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']


def setup():
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    ns['rnd'] = ns['randomizer']()
    ns['scanner'] = ns['sstscanner']()
    ns['logfp'] = None
    ns['replayfp'] = io.StringIO('')
    game.options = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']
    for line in Path('test65.log').read_text().splitlines():
        if line.startswith('# seed '):
            ns['rnd'].seed(int(line.split()[2]))
        elif line.startswith('# arguments '):
            ns['scanner'].inqueue = line.split()[2:]
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
        game.quadrant = copy.copy(game.state.kscmdr)
        ns['newqad']()
    game.justin = False
    return game


for condition in ('green', 'red', 'docked'):
    game = setup()
    source_quad = copy.copy(game.quadrant)
    sc = next(enemy for enemy in game.enemies if enemy.type == 'S')
    old_sector = copy.copy(sc.location)
    others = []
    if condition == 'red':
        others.append(ns['newkling']())
        game.klhere += 1
        game.state.galaxy[source_quad.i][source_quad.j].klingons += 1
    if condition == 'docked':
        game.state.baseq.append(copy.copy(source_quad))
        game.state.galaxy[source_quad.i][source_quad.j].starbase = True
        game.base = ns['dropin']('B')
    game.condition = 'docked' if condition == 'docked' else 'red'
    game.ientesc = True
    ns['schedule'](ns['FSCDBAS'], 1.0)
    total = game.remkl()
    with contextlib.redirect_stdout(io.StringIO()):
        if condition == 'docked':
            assert ns['movescom'](Coord(6, 6), False)
        else:
            ns['supercommander']()
    destination = copy.copy(game.state.kscmdr)
    assert destination != source_quad, 'Supercommander did not leave'
    assert sc not in game.enemies, 'Departed Supercommander remains a target'
    assert game.quad[old_sector.i][old_sector.j] == '.'
    assert game.enemies == others, 'Departure changed other targets'
    assert game.klhere == len(others)
    assert game.state.galaxy[source_quad.i][source_quad.j].klingons == len(others)
    assert game.state.galaxy[destination.i][destination.j].klingons == 1
    assert game.remkl() == total and game.state.nscrem == 1
    assert game.condition == condition
    assert not game.iscate and not game.ientesc and game.isatb == 0
    assert not ns['is_scheduled'](ns['FSCDBAS'])
    print(f'Departure: no ghost, counts agree, condition {condition}.')

    # The ship can only be killed after pursuing it into its new quadrant.
    game.quadrant = copy.copy(destination)
    with contextlib.redirect_stdout(io.StringIO()):
        ns['newqad']()
        sc = next(enemy for enemy in game.enemies if enemy.type == 'S')
        ns['deadkl'](sc.location, sc.type, game.sector)
    assert game.remkl() == total - 1
    assert game.state.nscrem == 0
    assert game.klhere == 0 and not game.enemies
    assert game.state.galaxy[destination.i][destination.j].klingons == 0
    assert all(q.klingons >= 0 for _, _, q in game.state.traverse())
    print('Pursuit: one kill, no negative galaxy counts.')

game = setup()
total = game.remkl()
enemies = list(game.enemies)
sector_map = copy.deepcopy(game.quad)
assert not ns['movescom'](copy.copy(game.quadrant), False)
assert game.enemies == enemies and game.quad == sector_map
assert game.remkl() == total and game.klhere == 1
print('Stationary local move rejected without removing the Supercommander.')

game = setup()
origin = copy.copy(game.state.kscmdr)
game.quadrant = Coord(6, 6)
with contextlib.redirect_stdout(io.StringIO()):
    ns['newqad']()
game.justin = False
enemies = list(game.enemies)
sector_map = copy.deepcopy(game.quad)
local_count = game.klhere
total = game.remkl()
assert not ns['movescom'](copy.copy(game.quadrant), False)
assert game.state.kscmdr == origin and game.remkl() == total
assert game.enemies == enemies and game.quad == sector_map
assert game.klhere == local_count
print('Move into the Enterprise quadrant rejected without changing counts.')

destination = Coord(6, 5)
previous_count = game.state.galaxy[destination.i][destination.j].klingons
with contextlib.redirect_stdout(io.StringIO()):
    assert ns['movescom'](destination, False)
assert game.state.kscmdr == destination and game.remkl() == total
assert game.state.galaxy[origin.i][origin.j].klingons == 0
assert game.state.galaxy[destination.i][destination.j].klingons == previous_count + 1
assert game.enemies == enemies and game.quad == sector_map
assert game.klhere == local_count
print('Remote movement updates galaxy counts without changing local targets.')
PY
