#!/bin/sh
python3 - 2>&1 <<'PY'
import contextlib
import copy
import copyreg
import io
from pathlib import Path
import pickle
import sys
import tempfile
import types

source = Path('../sst')
module = types.ModuleType('sst_restricted_save_test')
sys.modules[module.__name__] = module
ns = module.__dict__
exec(compile(source.read_text(), str(source), 'exec'), ns)
ns['rnd'] = ns['randomizer']()
ns['scanner'] = ns['sstscanner']()
ns['logfp'] = None

def same_state(left, right, seen=None):
    if seen is None:
        seen = ({}, {})
    assert type(left) is type(right)
    if type(left) in (type(None), bool, int, float, str):
        assert left == right
        return
    forward, reverse = seen
    if id(left) in forward:
        assert forward[id(left)] == id(right)
        return
    assert id(right) not in reverse
    forward[id(left)] = id(right)
    reverse[id(right)] = id(left)
    if type(left) is list:
        assert len(left) == len(right)
        for a, b in zip(left, right):
            same_state(a, b, seen)
    else:
        assert vars(left).keys() == vars(right).keys()
        for key in vars(left):
            same_state(vars(left)[key], vars(right)[key], seen)

def fresh(seed=1, skill='good', year=2026):
    ns['game'] = game = ns['Gamestate']()
    ns['thing'] = ns['Thingy']()
    game.options = ns['OPTION_ALL'] & ~ns['OPTION_CURSES']
    ns['rnd'].seed(seed)
    ns['scanner'].inqueue = ['regular', 'short', skill, str(year)]
    ns['replayfp'] = io.StringIO('captain\n')
    with contextlib.redirect_stdout(io.StringIO()):
        ns['setup']()
    return game

protocols = [int(line) for line in Path('test83.log').read_text().splitlines()
             if line and not line.startswith('#')]
game = fresh()
game.probe = ns['course'](1.0, 2.0)
game.tholian = ns['Enemy']('T', ns['dropin'](), 100.0)
game.iplnet = game.state.planets[0]
game.snapsht = copy.deepcopy(game.state)
game.state.snap = True
original = pickle.dumps(game, protocol=4)
for protocol in protocols:
    loaded = ns['load_game'](io.BytesIO(pickle.dumps(game, protocol=protocol)))
    assert type(loaded) is ns['Gamestate'] and loaded is not game
    same_state(game, loaded)
    assert loaded.tholian is loaded.enemies[-1]
    assert loaded.iplnet is loaded.state.planets[0]
    assert loaded.snapsht.planets[0] is not loaded.state.planets[0]
    assert type(loaded.probe) is ns['course']
    assert ns['game'] is game and pickle.dumps(game, protocol=4) == original
    print(f'Protocol {protocol}: complete state, shared identities, snapshot, enemy, and probe restored.')

legacy = copy.deepcopy(game)
del legacy.sandbox, legacy.wayback_disabled, legacy.tholian.pending_remove
legacy.statekscmdr = ns['Coord']()
legacy.state.galaxy[0][0].supernova = True
for snapshot in (legacy.state, legacy.snapsht):
    for row in snapshot.chart:
        for page in row:
            del page.supernova
loaded = ns['load_game'](io.BytesIO(pickle.dumps(legacy, protocol=0)))
assert not loaded.sandbox and loaded.wayback_disabled == 0
assert not loaded.tholian.pending_remove and not hasattr(loaded, 'statekscmdr')
assert loaded.state.chart[0][0].supernova
assert not loaded.snapsht.chart[0][0].supernova
print('Legacy fields migrate after validation, including independent snapshot charts.')

for skill in ('novice', 'fair', 'good', 'expert', 'emeritus'):
    for year in (1973, 2026):
        for seed in range(1, 6):
            game = fresh(seed, skill, year)
            loaded = ns['load_game'](io.BytesIO(pickle.dumps(game)))
            same_state(game, loaded)
print('50 games spanning all difficulties and old/modern Wayback settings round-trip.')

class Payload:
    def __init__(self, function, *args):
        self.function, self.args = function, args
    def __reduce__(self):
        return self.function, self.args

game = fresh()
original = pickle.dumps(game)
class_state = {cls: dict(vars(cls)) for cls in
               (*ns['SAVE_CLASSES'], ns['SavedRecord'], ns['SavedReconstructor'])}
rejected = 0
with tempfile.TemporaryDirectory(prefix='sst-load-test-') as directory:
    filename = Path(directory) / 'game.trk'
    def reject(data):
        global rejected
        filename.write_bytes(data)
        ns['scanner'].inqueue = [str(filename)]
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            assert ns['thaw'](), 'Hostile save accepted'
        assert 'invalid or unsafe saved game' in output.getvalue(), output.getvalue()
        assert 'SST_PICKLE_EXECUTED' not in output.getvalue()
        assert ns['game'] is game and pickle.dumps(game) == original
        assert all(dict(vars(cls)) == attrs for cls, attrs in class_state.items())
        rejected += 1

    for protocol in protocols:
        reject(pickle.dumps(Payload(print, 'SST_PICKLE_EXECUTED'), protocol=protocol))
        reject(pickle.dumps(Payload(eval, "print('SST_PICKLE_EXECUTED')"), protocol=protocol))
        # An allowed class must not invoke its constructor or mutate game.enemies.
        reject(pickle.dumps(Payload(ns['Enemy']), protocol=protocol))
    # No module imports or dotted attribute lookup may be requested by a pickle.
    reject(b'csubprocess\nPopen\n.')
    reject(b'c__main__\nGamestate.__init__\n.')
    # BUILD must not modify a shared loader function or a game class.
    reject(b'ccopy_reg\n_reconstructor\n}Vpoison\nI1\nsb.')
    reject(b'c__main__\nGamestate\n}Vpoison\nI1\nsb.')
    for code in (42, 300, 70000):
        copyreg.add_extension('builtins', 'print', code)
        try:
            copyreg._extension_cache[code] = print
            data = pickle.dumps(Payload(print, 'SST_PICKLE_EXECUTED'), protocol=2)
            reject(data)
            try:
                ns['GameUnpickler'](io.BytesIO(data)).load()
            except pickle.UnpicklingError:
                pass
            else:
                raise AssertionError('Direct unpickler bypassed extension rejection')
        finally:
            copyreg.remove_extension('builtins', 'print', code)
    print('Function payloads, constructor side effects, imports, and cached extensions blocked.')

    for change in (
        lambda g: setattr(g, 'state', None),
        lambda g: setattr(g, 'damage', ['bad'] * ns['NDEVICES']),
        lambda g: g.future.pop(),
        lambda g: g.state.galaxy.pop(),
        lambda g: setattr(g.quadrant, 'i', -1),
        lambda g: setattr(g.sector, 'j', 10),
        lambda g: setattr(g.state.galaxy[0][0], 'planet', 'bad'),
        lambda g: setattr(g, 'energy', float('nan')),
        lambda g: setattr(g, 'energy', float('inf')),
        lambda g: setattr(g, 'torps', 2**100),
        lambda g: setattr(g, 'warp', 'shadow a method'),
        lambda g: setattr(g, '__setstate__', 'shadow a hook'),
        lambda g: delattr(g, 'energy'),
    ):
        invalid = copy.deepcopy(game)
        change(invalid)
        reject(pickle.dumps(invalid))
    print('Wrong field types, dimensions, coordinates, nonfinite values, and injected fields rejected.')

    cycle = []
    cycle.append(cycle)
    deep = []
    for _ in range(110):
        deep = [deep]
    for data in (
        pickle.dumps(None), pickle.dumps(cycle), pickle.dumps(deep),
        b'', b'not a pickle', original[:-1], original + b'junk',
        b'x' * (ns['SAVE_MAX_BYTES'] + 1),
        b'N0' * ns['SAVE_MAX_OPS'] + b'N.',
        b'\x80\x02]r\xff\xff\xff\xff.',  # Huge memo allocation.
        b'\x80\x04\x95' + (2**40).to_bytes(8, 'little') + b'N.',
        b'Pexternal\n.', b'\x80\x02NQ.',
    ):
        reject(data)
    print('Cyclic, truncated, oversized, trailing, persistent-ID, and allocation-abuse inputs rejected.')
    print(f'{rejected} rejected saves leave the active game and password unchanged.')

    filename.write_bytes(original)
    ns['scanner'].inqueue = [str(filename)]
    with contextlib.redirect_stdout(io.StringIO()):
        assert not ns['thaw']()
    assert ns['game'] is not game
    same_state(game, ns['game'])
    print('A valid save loads successfully after rejected inputs.')
PY
