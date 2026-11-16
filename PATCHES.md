# Super Trek: differences from SST2K 2.10

Super Trek follows SST2K 2.10 (`sst`, Python) exactly: every recorded game
in SST2K's test suite that runs the program, and every one of the random
games `tools/fuzz.py` plays through both, gives byte-for-byte the same
output. This file lists every place where the port does something else, and
why.

## Places where SST2K crashes or hangs

SST2K stops with a Python traceback (or loops forever) in the situations
below. The port carries on in the most natural way instead; each spot is
marked `// DEVIATION:` in the source.

| Where | SST2K | Super Trek |
|---|---|---|
| `imove`, edge of the galaxy (`moving.rs`) | a course ending exactly on the far edge (sector 80) reflects onto itself in the negative-energy-barrier loop and the game hangs | the end point is moved one sector inside; found by `tools/fuzz.py` (games 15, 84, 101 and others) |
| `imove`, a course of zero moves with enemies present (`moving.rs`) | `move m 0.01` and the like: TypeError on `w - enemy.location` | the ship's own sector stands in, as it has not moved |
| `abandon` next to an uninhabited planet with WORLDS on (`rescue.rs`, `sst` 5258) | TypeError: an uninhabited planet has no name | the crew beam down with the WORLDS-off wording; fuzz game 120 |
| `destroybase` after ABANDON to a base under attack (`events.rs`) | the ship's quadrant is the same object as the base under attack and is invalidated with it; the game crashes at the next command (fuzz game 243) | only the battle marker is cleared |
| events with a missing starbase, base or planet name (`events.rs`, `sst` 2660, 2875, 2885, 3043, 3047) | TypeError | the event finds no base; a missing planet name prints "None" |
| probe event with no probe (`events.rs`) | AttributeError | the event is dropped |
| `recompute()` with no Klingons and no Commanders left (`events.rs`) | ZeroDivisionError | the float division gives infinity |
| nova buffeting the ship more than one sector (`nova.rs`, `sst` 3236) | IndexError in the course table | the direction of the push is used |
| a planet destroyed while `iplnet` is None (`nova.rs`, `battle.rs`) | AttributeError | the planet record is left alone |
| Tholian hit after it has gone (`battle.rs`) | AttributeError | nothing to move |
| `capture` when the weakest Klingon has no power left (`battle_player.rs`) | ZeroDivisionError | it surrenders |
| planet commands with no planet (`planets.rs`) | AttributeError | the planet reads as unknown; `mine` does nothing |
| `deathray` if an enemy outlives `deadkl` (`planets.rs`) | endless loop | the loop stops |
| `report` with an unknown game length or a missing probe (`reports.rs`) | KeyError, AttributeError | an empty length; the probe "Nowhere" |
| `chart` on a charted quadrant whose page was never filled (`reports.rs`) | TypeError | counts as 0 |
| `finish` when the initial resources or Klingons were 0 (`finish.rs`) | ZeroDivisionError (setup never allows it) | infinity |
| write errors while saving or writing a plaque (`save.rs`, `finish.rs`) | OSError traceback | "Can't freeze game as file ..."; the plaque is left incomplete |

## Deliberate differences

- **Saved games** are this port's own versioned JSON
  (`{"format": "super-trek-save", "version": 1, ...}`), not Python pickles,
  with SST2K's validation of the loaded game ported. SST2K's prompts, the
  `.trk` suffix rule and its messages are kept. A bare file name is saved
  in `$XDG_STATE_HOME/super-trek` (else `~/.local/state/super-trek`,
  `%LOCALAPPDATA%\super-trek\saves` on Windows); a name with a folder is
  used as given.
- **Input log**: SST2K writes `$TMPDIR/sst-input.log`; Super Trek writes
  `~/.cache/super-trek/input.log` (`$XDG_CACHE_HOME`), same format, and it
  replays with `super-trek -r`. Its "recorded by" line names the user and
  Super Trek, not the host.
- **Help**: SST2K looks for a generated `sst.doc` in `.`, `doc/` or
  `/usr/share/doc/sst/`; Super Trek has it built in (made with upstream's
  own xmlto and makehelp.py pipeline, `tools/make-sst-doc.sh`), so HELP
  always finds it. Its text is ESR's, with upstream's doubled `%%` and its
  `%% OP` heading for OPTIONS kept as they are.
- **Screen**: SST2K's curses mode is replaced by the Ratatui bridge, which
  runs the teletype game unchanged and draws its panels without the side
  effects SST2K's curses redraws have. The CURSES command does nothing.
  The panels read only what the ship knows (the star chart).
- **`-V`** prints "Super Trek <version> (SST2K 2.10)". `--tty` is new.
  An invalid `-s` seed is an error message, not a traceback.
- **Seeds and numbers**: Python accepts arbitrarily large integers and
  Unicode digits in a replay's `# seed`; the port reduces any ASCII
  integer exactly but does not read non-ASCII digits.
- **Platforms**: on Linux the port uses the same C maths library as
  Python and matches bit for bit; on macOS and Windows a rare last-bit
  difference in `sin`, `cos`, `atan2`, `log` or `pow` could steer a game
  differently from SST2K there.
