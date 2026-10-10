#!/bin/sh
python3 - 2>&1 <<'PY'
import contextlib
import io
from pathlib import Path
import sys
import tempfile
import types

source = Path('../sst')
module = types.ModuleType('sst_wayback_restart_test')
sys.modules[module.__name__] = module
ns = module.__dict__
exec(compile(source.read_text(), str(source), 'exec'), ns)
ns['rnd'] = ns['randomizer']()
ns['scanner'] = ns['sstscanner']()
ns['logfp'] = None
defaults = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']


def fresh(options=defaults):
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    game.options = options
    return game


def start(year):
    ns['rnd'].seed(seed)
    ns['scanner'].chew()
    ns['scanner'].inqueue = ['regular', 'short', 'good', str(year)]
    ns['replayfp'] = io.StringIO('captain\n')
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()


def options(command):
    ns['scanner'].chew()
    ns['scanner'].inqueue = command.split() + ['\n']
    with contextlib.redirect_stdout(io.StringIO()):
        ns['goptions']()


years = []
for line in Path('test77.log').read_text().splitlines():
    if line.startswith('# seed '):
        seed = int(line.split()[2])
    if line and not line.startswith('#'):
        years.append(int(line))

# Compare reused game state with an independently initialized game for each year.
expected = {}
for year in years:
    reference = fresh()
    start(year)
    expected[year] = (reference.options, reference.state.nscrem,
                      reference.inplan, reference.remkl(), ns['rnd'].getrngstate())
game = fresh()
for year in years:
    start(year)
    assert ns['game'] is game
    actual = (game.options, game.state.nscrem, game.inplan,
              game.remkl(), ns['rnd'].getrngstate())
    assert actual == expected[year], f'Wayback {year} differs from a fresh game'
    assert game.options & ns['OPTION_IOMODES'] == ns['OPTION_TTY']
    print(f'Wayback {year}: options, generated galaxy, and RNG match a fresh game.')

# A player preference that predates Wayback must not be restored automatically.
game = fresh(defaults & ~ns['OPTION_COLOR'])
start(1973)
start(2026)
assert game.options == defaults & ~ns['OPTION_COLOR']
assert game.options & ns['OPTION_ALPHAMERIC']
print('Initial color preference preserved while modern features return.')

# Clearing an already hidden feature is still an explicit preference.
game = fresh()
start(1973)
options('clear color alphameric')
start(2026)
without_display = defaults & ~(ns['OPTION_COLOR'] | ns['OPTION_ALPHAMERIC'])
assert game.options == without_display
options('set alphameric')
start(1973)
start(2026)
assert game.options == defaults & ~ns['OPTION_COLOR']
print('OPTIONS changes survive Wayback switches, including changes to hidden features.')

with tempfile.TemporaryDirectory(prefix='sst-wayback-test-') as directory:
    game = fresh(defaults & ~ns['OPTION_COLOR'])
    start(1973)
    restricted = game.options
    filename = str(Path(directory) / 'wayback.trk')
    ns['scanner'].chew()
    ns['scanner'].inqueue = [filename]
    with contextlib.redirect_stdout(io.StringIO()):
        ns['freeze'](False)
    start(2026)
    ns['scanner'].chew()
    ns['scanner'].inqueue = ['saved', filename]
    ns['replayfp'] = None
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
    game = ns['game']
    assert game.thawed and game.options == restricted
    start(2026)
    assert game.options == defaults & ~ns['OPTION_COLOR']
    print('Saved historical game keeps its rules and restores modern options for the next game.')

    # Older saves lack the suppression mask; their active choices remain valid.
    options('clear alphameric')
    legacy_options = game.options
    del game.wayback_disabled
    ns['scanner'].inqueue = [filename]
    with contextlib.redirect_stdout(io.StringIO()):
        ns['freeze'](False)
    fresh()
    ns['scanner'].inqueue = ['saved', filename]
    ns['replayfp'] = None
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
    game = ns['game']
    assert game.thawed and game.options == legacy_options
    start(2026)
    assert game.options == legacy_options
    print('Legacy saves load and preserve their recorded option choices.')
PY
