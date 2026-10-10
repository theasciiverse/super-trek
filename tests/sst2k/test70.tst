#!/bin/sh
# Exercise both immediate removal and the deferred removal used by movebaddy().
python3 - 2>&1 <<'PY'
import copy
from pathlib import Path

source = Path('../sst')
ns = {'__name__': 'sst_retreat_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']


def coord(text):
    return Coord(ord(text[0]) - ord('a'), int(text[1:]) - 1)


for line in Path('test70.log').read_text().splitlines():
    if not line or line.startswith('#'):
        continue
    origin, sector, row, column, destination = line.split()
    for etype in ('K', 'C', 'S'):
        for deferred in (False, True):
            ns['game'] = game = ns['Gamestate']()
            game.options = ns['OPTION_TTY'] | ns['OPTION_ALPHAMERIC']
            game.quadrant = coord(origin)
            game.sector = Coord(4, 4)
            game.ship = 'E'
            game.alive = True
            game.energy = 5000
            game.condition = 'red'
            game.quad = [['.'] * 10 for _ in range(10)]
            game.quad[4][4] = 'E'
            enemy = ns['Enemy'](etype, coord(sector), 100)
            others = []
            if etype == 'K':
                # An ordinary retreat must leave its local commander alone.
                others.append(ns['Enemy']('C', Coord(6, 6), 1200))
            if etype in ('K', 'C'):
                game.state.kcmdr = [copy.copy(game.quadrant)]
            else:
                game.state.kscmdr = copy.copy(game.quadrant)
                game.state.nscrem = 1
            game.klhere = len(game.enemies)
            source_quad = game.state.galaxy[game.quadrant.i][game.quadrant.j]
            source_quad.klingons = game.klhere
            total = game.remkl()
            old_map = copy.deepcopy(game.quad)
            moves = ns['tryexit'](enemy, Coord(int(row), int(column)), False,
                                  defer_remove=deferred)
            if destination == 'blocked':
                assert moves == [], (line, etype, deferred)
                assert game.quad == old_map
                assert game.enemies == [enemy] + others
                assert enemy.location == coord(sector) and not enemy.pending_remove
                assert game.klhere == total and source_quad.klingons == total
                expected = game.quadrant
            else:
                expected = coord(destination)
                assert len(moves) == 1 and moves[0][0]
                assert moves[0][4] == expected, (line, etype, deferred, moves[0][4])
                assert moves[0][2] == coord(sector)
                assert game.quad[coord(sector).i][coord(sector).j] == '.'
                assert game.klhere == len(others) and source_quad.klingons == len(others)
                assert game.state.galaxy[expected.i][expected.j].klingons == 1
                assert not enemy.location.valid_sector()
                assert enemy.pending_remove == deferred
                assert game.enemies == ([enemy] if deferred else []) + others
            assert game.remkl() == total
            assert all(q.klingons >= 0 for _, _, q in game.state.traverse())
            if etype == 'K':
                assert game.state.kcmdr == [game.quadrant]
            elif etype == 'C':
                assert game.state.kcmdr == [expected]
            else:
                assert game.state.kscmdr == expected and game.state.nscrem == 1
    print(f'{origin} {sector} via ({row},{column}): {destination}; counts and locations agree.')
PY
