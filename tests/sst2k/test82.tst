#!/bin/sh
# Exercise the real victory handler, input reader, score calculation, and plaque writer.
python3 - 2>&1 <<'PY'
from contextlib import redirect_stdout
from io import StringIO
from pathlib import Path
from tempfile import TemporaryDirectory

source = Path('../sst')
ns = {'__name__': 'sst_citation_test'}
exec(compile(source.read_text(), str(source), 'exec'), ns)
ns['logfp'] = None
ns['rows'] = 0

with TemporaryDirectory(prefix='sst-citation-') as tmp:
    for line in Path('test82.log').read_text().splitlines():
        if not line or line.startswith('#'):
            continue
        case, skill, elapsed, alive, thawed, debug, answer, expected = line.split()
        game = ns['game'] = ns['Gamestate']()
        ns['scanner'] = ns['sstscanner']()
        game.options = ns['OPTION_TTY']
        game.skill = ns['SKILL_' + skill.upper()]
        game.ship = 'E'
        game.alive, game.thawed, game.idebug = map(lambda x: x == '1',
                                                 (alive, thawed, debug))
        game.indate = 1000.0
        game.state.date = game.indate + float(elapsed)
        # All 20 initial enemies have been destroyed; no penalties incurred.
        game.inkling, game.incom, game.inscom = 17, 2, 1
        filename = Path(tmp) / (case + '.txt')
        captain = 'Captain ' + case
        ns['replayfp'] = StringIO(f'{answer}\n{filename}\n{captain}\n')
        output = StringIO()
        with redirect_stdout(output):
            ns['finish'](ns['FWON'])
        transcript = output.getvalue()
        prompted = 'Do you want your Commodore Emeritus Citation printed?'
        assert game.gamewon and game.alldone, case
        assert (prompted in transcript) == (expected in ('printed', 'declined')), case
        assert ('You cannot get a citation, so...' in transcript) == (expected == 'blocked'), case
        assert filename.exists() == (expected == 'printed'), case
        assert ('File or device name for your plaque:' in transcript) == (expected == 'printed'), case

        if expected in ('printed', 'declined', 'blocked'):
            promotion = ('promotes you to Commodore Emeritus.' if skill == 'expert'
                         else 'Now you can retire and write your own Star Trek game!')
            assert promotion in transcript, case
            if expected != 'blocked':
                assert transcript.index(promotion) < transcript.index(prompted), case
                assert transcript.index(prompted) < transcript.index('Your score --'), case
        if expected == 'printed':
            plaque = filename.read_text()
            assert captain in plaque and 'Commodore Emeritus' in plaque, case
            assert ('Expert level' if skill == 'expert' else 'Emeritus level') in plaque, case
            assert f'Your score:  {int(game.score)}' in plaque, case
            assert f'Klingons per stardate:  {game.perdate:.2f}' in plaque, case
            assert transcript.index('Your score --') < transcript.index('File or device name'), case
            assert ns['replayfp'].read() == '', case
        elif expected == 'declined':
            assert ns['replayfp'].read() == f'{filename}\n{captain}\n', case
        else:
            assert ns['replayfp'].tell() == 0, case
        print(f'{case}: citation {expected}.')
PY
