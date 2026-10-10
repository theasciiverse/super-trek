#!/bin/sh
# Exercise base arrival and the real destruction event in a late-mission state.
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path

source = Path('../sst')
ns = {'__name__': 'sst_supercommander_base_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
Coord = ns['Coord']

for case in Path('test72.log').read_text().splitlines():
    if case.startswith('# seed '):
        seed = int(case.split()[2])
    if not case or case.startswith('#'):
        continue
    ns['game'] = game = ns['Gamestate']()
    ns['rnd'] = ns['randomizer']()
    ns['rnd'].seed(seed)
    ns['logfp'] = None
    ns['replayfp'] = io.StringIO('')
    game.options = ns['OPTION_TTY'] | ns['OPTION_ALPHAMERIC']
    game.skill = ns['SKILL_GOOD']
    game.inkling, game.incom, game.inscom = 10, 3, 1
    game.intime = 7
    game.indate = 100
    game.state.date = 103.1
    game.state.remres = 140
    game.state.crew = ns['FULLCREW']
    game.ship, game.alive, game.condition = 'E', True, 'green'
    game.energy, game.shield, game.torps = 5000, 2500, 10
    game.quadrant, game.sector = Coord(7, 7), Coord(4, 4)
    game.quad = [['.'] * 10 for _ in range(10)]
    game.quad[4][4] = 'E'
    game.state.kscmdr = Coord(2, 3)
    game.state.nscrem = game.state.galaxy[2][3].klingons = 1
    base = Coord(3, 3)
    game.state.baseq = [copy.copy(base)]
    game.battle = Coord()
    for event in range(ns['NEVENTS']):
        ns['unschedule'](event)
    ns['schedule'](ns['FSCMOVE'], 0.2777)
    if case == 'radio-damaged':
        game.damage[ns['DRADIO']] = 10
    elif case == 'passive':
        game.state.date = 101
    elif case.startswith('commander-'):
        game.battle = copy.copy(base) if case == 'commander-same-base' else Coord(0, 0)
        game.state.kcmdr = [copy.copy(game.battle)]
        game.state.galaxy[game.battle.i][game.battle.j].klingons = 1
        game.state.baseq.append(Coord(4, 3) if case == 'commander-same-base'
                                else copy.copy(game.battle))
        ns['schedule'](ns['FCDBAS'], 0.5)
    for location in game.state.baseq:
        game.state.galaxy[location.i][location.j].starbase = True
    battle = copy.copy(game.battle)
    commander_deadline = ns['scheduled'](ns['FCDBAS'])
    with contextlib.redirect_stdout(io.StringIO()) as output:
        ns['supercommander']()
    if case in ('commander-same-base', 'passive'):
        if case == 'commander-same-base':
            assert game.state.kscmdr == base, 'Expected passage through the besieged base'
        assert game.isatb == 0 and not ns['is_scheduled'](ns['FSCDBAS'])
        assert game.battle == battle
        assert ns['scheduled'](ns['FCDBAS']) == commander_deadline
        print(f'{case}: no Supercommander attack scheduled.')
        continue
    assert game.state.kscmdr == base
    assert game.isatb == 1, 'Supercommander reached the base without attacking'
    assert ns['is_scheduled'](ns['FSCDBAS'])
    assert game.battle == battle
    assert ns['scheduled'](ns['FCDBAS']) == commander_deadline
    delay = ns['scheduled'](ns['FSCDBAS']) - game.state.date
    extra = 0.5 if case == 'commander-elsewhere' else 0
    assert 1 + extra <= delay < 3 + extra
    warned = 'under attack from the Klingon Super-commander' in output.getvalue()
    assert warned == (case != 'radio-damaged')
    assert game.iseenit == warned
    # Run normal movement ticks and destruction events through the deadline.
    game.optime = delay + 0.001
    with contextlib.redirect_stdout(io.StringIO()):
        ns['events']()
    assert not game.alldone
    assert game.state.kscmdr == base and game.state.nscrem == 1
    assert not game.state.galaxy[base.i][base.j].starbase
    assert base not in game.state.baseq
    assert game.isatb == 0 and not ns['is_scheduled'](ns['FSCDBAS'])
    if case == 'commander-elsewhere':
        assert not ns['is_scheduled'](ns['FCDBAS'])
        assert not game.state.galaxy[battle.i][battle.j].starbase
    print(f'{case}: attack scheduled; warning {"sent" if warned else "suppressed"}; base destroyed.')
PY
