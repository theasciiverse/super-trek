#!/bin/sh
python3 - 2>&1 <<'PY'
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from unittest.mock import Mock, patch

source = Path('../sst')
ns = {'__name__': 'sst_request_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
game = ns['game'] = ns['Gamestate']()
scanner = ns['scanner'] = ns['sstscanner']()
Coord = ns['Coord']
ns['logfp'] = None
ns['rows'] = 0
game.options = ns['OPTION_TTY'] | ns['OPTION_ALPHAMERIC']
game.alive = True
game.quadrant, game.sector = Coord(1, 2), Coord(4, 5)
game.state.date, game.state.remtime = 2456.7, 4.25
game.warpfac, game.energy, game.torps = 6.5, 1234.5, 7
game.shield, game.inshld, game.shldup = 1250.0, 2500.0, True
game.state.galaxy[0][0].klingons = 3
game.state.galaxy[2][2].klingons = 4
game.inbase = 5
game.state.baseq = [Coord(0, 0), Coord(2, 2), Coord(3, 3)]

def request(*tokens):
    scanner.inqueue = [*tokens, '\n']
    output = StringIO()
    with redirect_stdout(output):
        ns['request']()
    return output.getvalue()

def field(label, value):
    return f'{label:<14}{value}\n'

count = 0
for line in Path('test81.log').read_text().splitlines():
    if not line or line.startswith('#'):
        continue
    name, shortest, expected = line.split(' ', 2)
    label, value = expected.split('|')
    for length in range(len(shortest), len(name) + 1):
        for token in (name[:length], name[:length].upper()):
            actual = request(token)
            assert actual == field(label, value), (token, actual)
            count += 1
print(f'{count} lowercase and uppercase REQUEST prefixes return the correct single field.')

invalid = ('b', 'system', 'datejunk', 'stardate', '1', '?')
for token in invalid:
    output = request(token)
    assert output.startswith('UNRECOGNIZED REQUEST. Legal requests are:\n'), output
    assert 'time, bases.' in output and 'system' not in output
    assert request('e') == field('Energy', '1234.50')
assert request('\n', 'energy') == 'Information desired? ' + field('Energy', '1234.50')
print('Invalid items list the documented names; missing items prompt and input recovers.')

game.state.remtime = 2.5
assert request('ti') == field('Time Left', '2.50')
game.state.baseq.pop()
assert request('ba') == field('Bases Left', '2')
game.state.baseq.clear()
assert request('bases') == field('Bases Left', '0')
game.state.galaxy[0][0].klingons = 0
assert request('k') == field('Klingons Left', '4')
game.sandbox = True
assert request('time') == field('Time Left', 'Unlimited')
assert request('date') == field('Stardate', '2456.7')
print('Time and enemy/base counts follow current state; Sandbox reports unlimited time.')

# Exercise command dispatch while stubbing only terminal rendering and input.
for mode, inputs in (
    (ns['OPTION_CURSES'], ['request energy\n', 'req ti\n', 'quit\n']),
    (ns['OPTION_TTY'], ['curses\n', 'request energy\n', 'req ti\n', 'quit\n']),
):
    game.options, game.alldone = mode, False
    request_spy = Mock()
    output = []
    noop = lambda *args: None
    with patch.dict(ns, {
        'drawmaps': noop, 'setwnd': noop, 'clrscr': noop, 'skip': noop,
        'iostart': noop, 'prout': output.append, 'proutn': output.append,
        'cgetline': Mock(side_effect=inputs), 'request': request_spy,
    }):
        ns['makemoves']()
    request_spy.assert_not_called()
    assert output.count('Beg your pardon, Captain?') == 2, output
    assert game.options & ns['OPTION_CURSES']
    assert not game.options & ns['OPTION_TTY']
print('Screen mode suppresses REQUEST and REQ, including after switching from TTY mode.')
PY
