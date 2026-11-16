# Super Trek

A Rust port of Super Star Trek, the 1970s teletype classic, for the
Asciiverse catalog, under the name Super Trek. Historic preservation: the
game's rules, text and behaviour are kept exactly; the port adds a native
binary on every platform and a Ratatui interface around the classic typed
commands.

Status (2026-11-16): the whole game is translated and plays, version
0.5.0. All 69 of SST2K's recorded games that run the program replay byte
for byte (debug and release), and the differential fuzzer
(`tools/fuzz.py`) finds no difference in about 2,300 random games. The
Ratatui bridge is in. See "State of the port" below for what remains.

## Lineage

- 1974: UT Super Star Trek, FORTRAN, by David Matuszek and Paul Reynolds,
  with modifications by Don Smith.
- 1995: Tom Almy's C translation ("resurrected by Tom Almy"), maintained to
  2019 at https://almy.us/sst.html. Its CAPTURE command comes from NetBSD
  trek (UC Berkeley, BSD licence).
- 2004 to 2026: Eric S. Raymond's SST2K, built on Almy's C (its first
  commit, 2004-10-31, is "Initial commit of Tom Almy's version plus
  sst.doc"), translated to Python in 2006 and developed since, with Dave
  Matuszek involved. Version 2.10 (2026-09-05) is what this port follows.
  Source: https://gitlab.com/esr/super-star-trek, cloned at
  `../../references/super-star-trek`.

This port translates SST2K 2.10 (`sst`, about 8,200 lines of Python). The
2006 C version in SST2K's `historic/c-version` is a second reference.

## Licence and terms

Every layer is honoured; see `NOTICE`:

- **Matuszek, Reynolds, Smith and Almy's permission** (from Almy's
  `sst.doc`, in every archive he has shipped since at least 1999): copying,
  distribution, modification and use for recreational purposes, with all
  author credits kept; never for sale, promotion or profit. So this title
  is free forever, whatever Asciiverse does about paid titles later.
- **ESR's BSD licence** for SST2K's own work (copyright notice and
  disclaimer kept, including its note on the Star Trek trademarks).
- **UC Berkeley's BSD licence** for the CAPTURE code from NetBSD trek.
- **Name: Super Trek** (David, 2026-10-10). Keeps the game's identity,
  drops Paramount's "Star Trek" trademark. The only earlier "Super Trek"
  found is a 1987 Atari ST type-in listing in Antic magazine. Inside the
  game, the original text stays as it is (the ship, the Klingons, the
  "-SUPER- STAR TREK" title), since that is the preservation.

## Fidelity: ESR's recorded games are the test suite

SST2K's `tests/` holds 83 recorded games: each `testNN.log` has the random
seed, the command-line arguments and every line the player typed, and
`testNN.chk` is the exact output the game printed (some use a `.tst` script
instead). The port must reproduce every `.chk` byte for byte. This is
possible because the game's randomness is one small generator:

```text
x' = (1093 * x + 221587) mod 1048576      random() = x / 1048576
```

(a linear congruential generator "tested against Knuth vol. 2 by the
authors of ADVENT", `randomizer` in `sst`). `integer`, `real` and
`withprob` are thin wrappers, ported exactly in `src/rng.rs`.

Order of work:

1. Generator (`src/rng.rs`), checked against values from the Python.
2. Test harness: replay a `.log` through the port's line-mode interface and
   diff against the `.chk`, all 83 as `cargo test`.
3. Translate the game, file by file in the order the Python reads (data
   types, setup, movement, combat, events, scoring, reports), until every
   recorded game matches.
4. Only then the Ratatui interface: chart, status and log windows (SST2K's
   three-window mode), keeping typed commands as the way to play.

## House conventions

- Session logging (`log` + `env_logger`, see `../../CLAUDE.md`).
- Saves and SST2K's input log (which it writes to `/tmp/sst-input.log` for
  bug reports) go to the platform's data and cache folders, not `/tmp` or
  the install folder.
- Release builds by the same GitHub workflow as Greed
  (`../greed/.github/workflows/release.yml`).

## Port conventions (how the Rust maps the Python)

One `Sst` struct (`src/sst.rs`) owns what the Python keeps in globals:
`game` (Gamestate, including the RNG state `lcg_x`), `thing`, `scanner`
and `io`. Every Python top-level function is a method on it, in the module
PLAN.md's table names, keeping its name (`consumeTime` is `consume_time`,
`listCommands` is `list_commands`).

- Output: `self.prout(&s)`, `self.proutn(&s)`, `self.skip(n)`,
  `self.prouts(&s)`. Python `%` formatting is `pf!("fmt", args...)` with
  the format string copied verbatim (gettext's `_()` dropped). A Coord
  argument prints as Python's `str()`; a float with `%s` prints Python's
  float repr ("5.0").
- Numbers: Python ints are `i64`, floats `f64`; keep each value's Python
  type. `int(x)` is `trunc(x)`, `round(x)` is `pyround(x)` (ties to even),
  every `**` and `math.pow` is `pypow` (never `powi`, `*` or `mul_add`),
  `//` and `%` on negatives use `div_euclid`/`rem_euclid` or
  `pyfloordiv`/`pymod`.
- RNG: `rnd_real()`, `rnd_real1(n)`, `rnd_real2(lo, hi)`,
  `rnd_integer(n)`, `rnd_integer2(lo, hi)`, `withprob(p)`, called in
  exactly the Python's order, short-circuits included.
- Coordinates: `Coord` (integers; `Coord::none()` is Python's None and
  `Coord()`), `Vec2` (floats, course arithmetic). `Course` is `course`.
- Enemies: a grow-only arena `game.pool`, the live list `game.enemies` of
  ids, `game.tholian` an id. `Enemy(...)` is `new_enemy`, `enemy.move(loc)`
  is `enemy_move`. Loops over `game.enemies` that can remove entries are
  index loops that reproduce Python's skip-the-next-element behaviour.
- Planets: indices into `game.state.planets` (`Quadrant.planet`,
  `game.iplnet`).
- `TrekError` is a `Result`; `SystemExit` unwinds to `main` with
  `resume_unwind(Box::new(SystemExit(code)))`.
- Python crash paths are fixed and marked `// DEVIATION:` (listed below).

## Decisions (2026-11-16)

Taken for PLAN.md section 10, each a conservative choice, David may
overrule:

1. **Exact on Linux, near-exact elsewhere**: no port of glibc's maths;
   macOS and Windows may differ in rare last-bit cases. Conservative
   choice, David may overrule.
2. **Python crash paths are fixed**, each marked `// DEVIATION:` in the
   source and listed in PATCHES.md with the game that shows it.
   Conservative choice, David may overrule.
3. **Known SST2K bugs are kept verbatim** (PLAN 4.15: "were playing", the
   dead DEBUG command, BSDTREK on the PLANETS bit, the Tholian internal
   error, `hold` at a1, the trailing blank cell in the command list).
   Conservative choice, David may overrule.
4. **UI: a line-mode core with a Ratatui shell** (PLAN 9). The game runs
   with SST2K's teletype semantics on its own thread; the bridge draws
   short-range scan, status, long-range scan and (40 rows or more) the star
   chart from a copy taken at each prompt, reading only what the ship
   knows, so a game played there replays exactly. `--tty`, `-t`, `-r`, or
   input/output that is not a terminal give the plain line mode.
   Conservative choice, David may overrule.
5. **Saves: own versioned JSON** in `$XDG_STATE_HOME/super-trek` (else
   `~/.local/state/super-trek`), `.trk` names and SST2K's messages kept.
   Conservative choice, David may overrule.
6. **Help: `sst.doc` generated once** with upstream's own xmlto and
   makehelp.py pipeline (`tools/make-sst-doc.sh`) and embedded
   (`assets/sst.doc`). Conservative choice, David may overrule.
7. **Wayback quirks kept verbatim**. Conservative choice, David may
   overrule.

## Deviations

Every difference from SST2K 2.10 is in `PATCHES.md`: the crash and hang
paths fixed (decision 2), saves, input log, help, screen and command line.
New ones go there and get a `// DEVIATION:` comment.

## Testing

- `cargo test` (and `cargo test --release`; PLAN 4.8 needs both): unit
  tests and the replay ratchet (`tests/replay.rs`, `tests/passing.txt`).
  `SUPER_TREK_REPLAY=test07 cargo test --test replay -- --nocapture` runs
  one game and writes its output for a diff.
- `tools/rngdiff.sh testNN`: SST2K's and the port's random-number call
  order side by side; the first difference names the diverging module.
- `tools/fuzz.py --games 500 --jobs 12`: random games through both; any
  differing game is kept as a ready replay log.
- Build, tests and recordings under
  `systemd-run --user --scope -p MemoryMax=4G`, `-j4`.
- `RUST_LOG=debug super-trek 2>log` logs every input line and key, each
  command, condition changes, quadrant entries and a state line at every
  prompt.
- `SUPER_TREK_FAST=1` turns off the slow typing of `prouts` (tests);
  `SUPER_TREK_YEAR` and `SUPER_TREK_DATE` fix the clock.
