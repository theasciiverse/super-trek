#!/bin/sh
# Reuse the same game and snapshot, as the play-again loop does.
python3 - 2>&1 <<'PY'
import contextlib
import copy
import io
from pathlib import Path

source = Path('../sst')
ns = {'__name__': 'sst_supercommander_restart_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)


def start(seed, skill, year):
    ns['rnd'].seed(seed)
    ns['scanner'].chew()
    ns['scanner'].inqueue = ['regular', 'short', skill, str(year)]
    ns['replayfp'] = io.StringIO('captain\n')
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()


def visit(location):
    ns['game'].quadrant = copy.copy(location)
    with contextlib.redirect_stdout(io.StringIO()):
        ns['newqad']()


for line in Path('test75.log').read_text().splitlines():
    if line.startswith('# seed '):
        initial_seed = int(line.split()[2])
    if not line or line.startswith('#'):
        continue
    seed, skill, year = line.split()
    ns['game'] = game = ns['Gamestate']()
    snapshot = game.state
    ns['thing'] = ns['Thingy']()
    ns['rnd'] = ns['randomizer']()
    ns['scanner'] = ns['sstscanner']()
    ns['logfp'] = None
    game.options = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']
    start(initial_seed, 'good', 2026)
    previous = copy.copy(game.state.kscmdr)
    assert game.state.nscrem == 1 and previous.valid_quadrant()
    assert ns['is_scheduled'](ns['FSCMOVE'])
    visit(previous)
    assert sum(enemy.type == 'S' for enemy in game.enemies) == 1

    start(int(seed), skill, year)
    assert ns['game'] is game and game.state is snapshot
    assert game.inscom == game.state.nscrem == 0
    assert not game.state.kscmdr.valid_quadrant(), 'New game retained the old Supercommander location'
    assert not ns['is_scheduled'](ns['FSCMOVE'])
    assert not ns['is_scheduled'](ns['FSCDBAS'])
    assert game.isatb == 0 and not game.iscate
    total = game.remkl()
    assert total == game.inkling + game.incom
    visit(previous)
    assert all(enemy.type != 'S' for enemy in game.enemies)
    quadrant = game.state.galaxy[previous.i][previous.j]
    if skill == 'fair':
        assert quadrant.klingons > 0, 'Reproduction needs Klingons at the old location'
    local_klingons = sum(enemy.type in ('K', 'C', 'S') for enemy in game.enemies)
    assert local_klingons == game.klhere == quadrant.klingons
    assert game.remkl() == total and game.state.nscrem == 0
    assert not game.iscate
    print(f'Good -> {skill} {year}: location cleared; no Supercommander or scheduled attacks; counts agree.')

    if year == '2026':
        start(initial_seed, 'good', 2026)
        assert game.state.nscrem == 1 and game.state.kscmdr.valid_quadrant()
        assert ns['is_scheduled'](ns['FSCMOVE'])
        visit(game.state.kscmdr)
        assert sum(enemy.type == 'S' for enemy in game.enemies) == 1
        print(f'{skill} -> Good: a new Supercommander is allocated and placed.')
PY
