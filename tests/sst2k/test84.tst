#!/bin/sh
python3 - 2>&1 <<'PY'
from pathlib import Path
import pickle
import subprocess
import tempfile

def run(args, commands=''):
    result = subprocess.run(['../sst', *args], input=commands, text=True,
                            capture_output=True, timeout=15, check=True)
    assert not result.stderr, result.stderr
    return result.stdout

class Payload:
    def __reduce__(self):
        return print, ('SST_PICKLE_EXECUTED',)

with tempfile.TemporaryDirectory(prefix='sst-replay-load-') as directory:
    root = Path(directory)
    savefile = root / 'saved.trk'
    commands = []
    for line in Path('test84.log').read_text().splitlines():
        if line.startswith('# seed '):
            seed = line.split()[2]
        if line and not line.startswith('#'):
            commands.append(line.format(savefile=savefile))
    run(['-t', '-s', seed, 'regular', 'short', 'good', '2026'],
        '\n'.join(commands) + '\nn\nn\n')
    assert savefile.exists()
    replay = root / 'resume.log'
    replay.write_text(f'# seed {seed}\n# arguments saved {savefile}\n#\nstatus\nquit\n')
    output = run(['-r', str(replay)])
    assert 'Warp Factor   6.0' in output
    assert 'invalid or unsafe saved game' not in output
    assert 'May the Great Bird of the Galaxy' in output
    print('A CLI save resumes through replay with its saved warp factor and normal commands.')

    for protocol in (0, 4, 5):
        savefile.write_bytes(pickle.dumps(Payload(), protocol=protocol))
        replay.write_text(f'# seed {seed}\n# arguments saved {savefile}\n#\n'
                          'regular short good 2026\nstatus\nquit\n')
        output = run(['-r', str(replay)])
        assert 'invalid or unsafe saved game' in output
        assert 'SST_PICKLE_EXECUTED' not in output.splitlines()
        assert 'Warp Factor   5.0' in output
    print('Executable saved-game payloads are rejected; a new game can still be started.')

    # Original single-file exploit: pickle instructions precede the replay headers.
    replay.write_bytes(pickle.dumps(Payload(), protocol=0) +
                       f'\n# seed {seed}\n# arguments saved {replay}\n#\n'
                       'regular short good 2026\nquit\n'.encode())
    output = run(['-r', str(replay)])
    assert 'invalid or unsafe saved game' in output
    assert 'SST_PICKLE_EXECUTED' not in output.splitlines()
    assert 'May the Great Bird of the Galaxy' in output
    print('The single-file saved-replay exploit is rejected without executing its marker.')
PY
