#!/bin/sh
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path

source = Path('../sst')
ns = {'__name__': 'sst_supernova_supercommander_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']
lines = Path('test74.log').read_text().splitlines()


def setup():
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    ns['rnd'] = ns['randomizer']()
    ns['scanner'] = ns['sstscanner']()
    ns['logfp'] = None
    ns['replayfp'] = io.StringIO('')
    game.options = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']
    for line in lines:
        if line.startswith('# seed '):
            ns['rnd'].seed(int(line.split()[2]))
        elif line.startswith('# arguments '):
            ns['scanner'].inqueue = line.split()[2:]
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
    return game


for case in lines:
    if not case or case.startswith('#'):
        continue
    game = setup()
    destination = copy.copy(game.state.kscmdr)
    assert destination == Coord(2, 1) and game.state.nscrem == 1
    origin = copy.copy(game.quadrant)
    with contextlib.redirect_stdout(io.StringIO()):
        game.quadrant = copy.copy(destination)
        ns['newqad']()
        assert sum(enemy.type == 'S' for enemy in game.enemies) == 1
        game.quadrant = origin
        ns['newqad']()
    # Let the real RNG select the Supercommander's quadrant for scheduled events.
    for i, j, quadrant in game.state.traverse():
        if Coord(i, j) != destination:
            quadrant.stars = 0
    total = game.remkl()
    destroyed = game.state.galaxy[destination.i][destination.j].klingons
    game.isatb = 1
    ns['schedule'](ns['FSCDBAS'], 1)
    assert ns['is_scheduled'](ns['FSCMOVE'])
    if case == 'radio-damaged':
        game.damage[ns['DRADIO']] = 5
    with contextlib.redirect_stdout(io.StringIO()):
        ns['supernova'](destination if case == 'induced' else None)
    assert game.state.nscrem == 0
    assert not game.state.kscmdr.valid_quadrant(), 'Dead Supercommander still has a quadrant'
    assert not ns['is_scheduled'](ns['FSCMOVE'])
    assert not ns['is_scheduled'](ns['FSCDBAS'])
    assert game.isatb == 0 and not game.iscate
    assert game.state.galaxy[destination.i][destination.j].supernova
    assert game.remkl() == total - destroyed
    if case == 'stale-location':
        # Older saved games can still carry the obsolete a1 coordinates.
        game.state.kscmdr = Coord(0, 0)
    game.quadrant = Coord(0, 0)
    with contextlib.redirect_stdout(io.StringIO()):
        ns['newqad']()
    klingons = [enemy for enemy in game.enemies if enemy.type in ('K', 'C', 'S')]
    assert sorted(enemy.type for enemy in klingons) == ['C', 'K', 'K', 'K', 'K']
    assert game.klhere == game.state.galaxy[0][0].klingons == len(klingons)
    assert game.state.nscrem == 0 and not game.iscate
    target = next(enemy for enemy in klingons if enemy.type == 'K')
    with contextlib.redirect_stdout(io.StringIO()):
        ns['deadkl'](target.location, target.type, game.sector)
    assert game.state.nscrem == 0 and game.remkl() == total - destroyed - 1
    assert game.klhere == game.state.galaxy[0][0].klingons == 4
    assert Coord(0, 0) in game.state.kcmdr
    print(f'{case}: Supercommander removed; a1 keeps ordinary ships and its commander; counts agree.')

# Row and column zero are valid locations for a living Supercommander.
game = setup()
old = game.state.kscmdr
game.state.galaxy[old.i][old.j].klingons -= 1
game.state.kscmdr = Coord(0, 0)
game.state.galaxy[0][0].klingons += 1
game.quadrant = Coord(0, 0)
with contextlib.redirect_stdout(io.StringIO()):
    ns['newqad']()
assert game.state.nscrem == 1
assert sum(enemy.type == 'S' for enemy in game.enemies) == 1
assert sum(enemy.type == 'C' for enemy in game.enemies) == 1
print('Living Supercommander at a1 is still placed alongside its commander.')
PY
