#!/bin/sh
# Call the plaque writer with real replay input and a fixed wall-clock date.
python3 - 2>&1 <<'PY'
import io
import os
from pathlib import Path
import sys
import tempfile
from unittest.mock import patch

source = Path('../sst')
commands = Path('test62.log').read_text()
ns = {'__name__': 'sst_plaque_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
ns['game'] = game = ns['Gamestate']()
game.options = ns['OPTION_TTY']
game.score = 1234
game.perdate = 12.34
ns['logfp'] = None
ns['replayfp'] = io.StringIO(commands)

with tempfile.TemporaryDirectory(prefix='sst-plaque-') as tmp:
    original_directory = Path.cwd()
    try:
        os.chdir(tmp)
        with patch.object(ns['time'], 'ctime', return_value='Sat Sep  5 12:34:56 2026'):
            for skill, filename in [(ns['SKILL_EXPERT'], 'expert.txt'),
                                    (ns['SKILL_EMERITUS'], 'emeritus.txt')]:
                game.skill = skill
                ns['plaque']()
                print('\n--- ' + filename + ' ---')
                sys.stdout.write(Path(filename).read_text())
    finally:
        os.chdir(original_directory)
PY
