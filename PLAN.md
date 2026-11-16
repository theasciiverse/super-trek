# Super Trek: port plan

Plan for porting SST2K 2.10 (`references/super-star-trek/sst`, 8,176 lines
of Python) to Rust with byte-for-byte identical output, made 2026-10-10 from
a full read of `sst`, its tests and docs, and a survey of the wider Trek
family. Line numbers refer to `sst`. Decisions still open for David are
collected at the end.

## 1. Where SST2K sits in the family

```
Mayfield STTR1, HP BASIC (1971/72)
 ├─ Leedom "Super Star Trek", BASIC (1974) -> BASIC Computer Games (1978)   [different game]
 └─ Hicks and Korp UT Trek, BASIC (1973)
     └─ UT FORTRAN Super Star Trek (Matuszek, Reynolds; Smith, Newman, Cohen; 1973-78)
         ├─ Prime FTN port, rev 19.1 (1983)   [source survives: mipmip.org/prime_games/startrek.tgz]
         ├─ BSD trek, Eric Allman (1976+)      [own catalog title: games/bsdtrek]
         └─ PDP-11 FORTRAN -> Tom Almy C (1995-2019)
              └─ ESR SST2K C (2004) -> Python (2006) -> 2.10 (2026-09-05)   <- this port
```

- The "recreational, no profit, keep the credits" notice is the **UT authors'**:
  it is in the FORTRAN header and in Almy's `sst.doc`. Super Trek stays free.
- SST2K already contains the best of the other branches: CAPTURE, CLOAK,
  weighted damage, crew, inhabited worlds and nova black holes (from BSD trek,
  via Almy); SCORE, the death-ray upgrade, Tholians, ramming, probes (Almy);
  the screen mode, base shields, black-hole time warp (Stas Sergeev). Nothing
  else needs merging for a faithful port. Almy's 2015 to 2019 fixes that may
  be missing from SST2K stay out (SST2K 2.10 is the reference) and are listed
  as known differences.
- **Wayback** (the fourth new-game question) is a timeline of feature
  switches, each option tagged with a year; it is not an emulation of old
  versions, and its docs say 1973 is approximate. It is ported as is.
- Reference material: the Prime `startrek.ftn` (4,991 lines) and its manual
  are the second historical source for 1970s text; SST2K's
  `historic/c-version` is ESR's 2006 C, context only (it uses the C library's
  `Rand()` and some formulas differ).

## 2. Architecture

One struct owns everything; every Python top-level function becomes a method
on it, in the module listed below. Rust allows `impl Sst` in many files, so
the translation stays function for function.

```rust
pub struct Sst {
    pub game: Gamestate, // includes rng (lcg_x), so saves restore the RNG as Python does
    pub thing: Thingy,   // outside game: never saved, survives thaw and new games
    pub scanner: Scanner,
    pub io: Io,          // output sink, replay reader, rows/linecount, input log, flags
}
```

| Python unit (`sst` lines) | Rust module |
|---|---|
| randomizer (50-106) | `rng.rs` (state becomes f64, see 4.13) |
| constants, options table, devices, events, finish codes, tables | `consts.rs` |
| Coord, letterize (164-279) | `coord.rs`: `Coord` (grid) and `Vec2` (course math) |
| Thingy, Planet, Quadrant, Page (one chart cell), Snapshot, Event, Enemy, Gamestate (301-675) | `state.rs` |
| I/O layer (3891-4315) | `io.rs` (TTY now, a sink trait for the UI later) |
| scanner, ja, huh (7844-7961) | `scanner.rs` |
| setup, choose, newqad, dropin, ... (6883-7453) | `setup.rs` |
| imove, warp, impulse, dock, course (4317-4940) | `moving.rs` |
| atover, timwrp, probe, mayday, abandon (4943-5313) | `rescue.rs` |
| phasers, torps, shields, capture, cloak | `battle_player.rs` |
| attack, torpedo, fry, deadkl, collision | `battle.rs` |
| tryexit, movebaddy, moveklings, super-commander, Tholian (704-1218) | `ai.rs` |
| schedule, events, wait (2550-3100) | `events.rs` |
| nova, supernova (3103-3356) | `nova.rs` |
| orbit, beam, mine, crystals, shuttle, deathray (5319-5804) | `planets.rs` |
| reports, scans, chart, status, request (5810-6440) | `reports.rs` |
| finish, score, plaque, kaboom (3362-3888) | `finish.rs` |
| freeze, thaw (6458-6807) | `save.rs` (own format) |
| helpme (7509-7578) | `help.rs` + embedded `sst.doc` |
| makemoves command loop (7581-7774) | `commands.rs` |
| getopt, replay header, play-again loop (8050-8176) | `cli.rs`, `main.rs` |

Rules that keep it mechanical:

- `game.x` -> `self.game.x`; `prout(..)` -> `self.prout(..)`; never hold a
  borrow of `self.game` across a `self.*` call (copy the Coord or id out).
- Enemies live in a grow-only arena (`pool: Vec<Enemy>`, `enemies: Vec<EnemyId>`,
  `tholian: Option<EnemyId>`): Python keeps references after removal, and that
  is observable (the Tholian "Internal error" quirk).
- Planets are indices; events are a `Vec` indexed by event code.
- One `Sst` across games, with exactly the resets `setup`/`choose` do: some
  fields deliberately survive a new game (tests 61, 75-77 depend on it).

## 3. Control flow

- `TrekError` -> `Result` from `getcourse`; `JumpOut` and `for/else` ->
  labelled blocks.
- `sys.exit` deep inside input reading -> `resume_unwind(SystemExit(code))`
  caught in `main`, which flushes, writes the input log, restores the
  terminal and exits with the code.
- Loops that remove from `game.enemies` while iterating keep Python's
  skip-the-next-element behaviour: index loops, never iterators.

## 4. Fidelity hazards (the main risk)

1. **`%` formatting**: a small runtime Python-`%` interpreter (`pf!`) with
   format strings copied verbatim; `%d` of a float truncates. Rust rounds
   formatting ties to even like Python; pinned by tests generated from Python.
2. **`str`/`repr`**: Coord `__str__`, Python string repr for "Can't interpret".
3. **`round()` is banker's rounding** -> `round_ties_even`, never `.round()`.
4. **`int()` truncates** -> `as i64`, clamping parsed numbers first.
5. **`//` and `%` on negatives** -> `div_euclid`/`rem_euclid`; the wraparound
   is observable (displaced enemies land on the far edge).
6. **Negative list indexing** (3236) -> a `py_index` helper.
7. **int/float/bool mixing** -> f64 for anything ever a float, explicit casts.
8. **`**` and math**: `x**2` differs from `x*x` in real cases, and LLVM turns
   `pow(x, 2.0)` into `x*x` in release builds. One helper,
   `pypow(x, y) = x.powf(black_box(y))`, for every `**`. Never `powi`, never
   `mul_add`. Tests run in debug and release.
9. **Evaluation order** copied verbatim (float association, RNG call order).
10. **Sorting/min**: stable sorts, first minimum (`min_by`, never `max_by`).
11. **Dict order** never reaches output or the RNG.
12. **Scanner**: Python `split()` and `float()` quirks (underscores, Unicode
    whitespace), case rules, prefix matching copied exactly.
13. **RNG state as f64**: tournament play seeds with a float, which Python
    keeps fractional; integer seeds stay bit exact.
14. **Clock**: only the Wayback default year reads it; injectable for tests.
15. **Known bugs kept verbatim**: "were playing" in report, dead DEBUG
    command, BSDTREK mapped to the PLANETS bit (an extra RNG call in novas),
    the Tholian internal error, `hold` starting at a1, the trailing blank cell
    in the command list.
16. **Coord aliasing**: every in-place mutation was traced; Rust `Copy`
    Coords do not change behaviour (recheck in review).
17. **Python crash paths** (5258, 3043/3047, 3236, 668, 2659/2885): see
    decisions.

## 5. I/O and replay

The recorded games run with `-r`; to match their output the port must:
emulate getopt (`cr:s:txV`); print each header line stripped (so
`# arguments ` loses its trailing space); seed and queue the header
arguments; take `rows` from **stdout only** (pipes never paginate; do not use
`crossterm::terminal::size`, which falls back to /dev/tty); echo every replay
line including comments; after replay EOF continue from stdin; stop after one
game with the "Great Bird" farewell; send nothing to stderr but header
errors; exit 0. Two flags, `replay_started` and `replay_open`, because Python's
closed file stays truthy.

## 6. Saves and help

- Saves: own versioned JSON format (serde) with a port of SST2K's
  validation; keep `.trk` and the error messages; no pickle compatibility.
- Help: SST2K reads a generated `sst.doc` (from `sst-doc.xml` via xmlto and
  `makehelp.py`); it is not in the repo. Generate it once, commit it as an
  asset, embed it. No test covers HELP.

## 7. Tests

- 59 plain replays + 6 flag-running scripts + 3 subprocess Python tests
  (63, 78, 80) = **68 compared through the binary**.
- 14 Python unit tests ported as Rust unit tests (plaque, super-commander,
  setup per skill, tryexit, base attacks, supernova, play again, options,
  request, citations, saves): test62 in finish.rs, test83 in save.rs, the
  rest in src/unit_tests.rs, each compared with its .chk.
- Uncovered by any recording (needs review and newly recorded games):
  impulse, time warp, engine damage above warp 6, supernova escape, Tholian
  web completion, a commander destroying a base, enslavement and
  reproduction, crystals, shield malfunction, misfire, many endings,
  tournament games, saves in replay, OPTIONS/HELP/COMMANDS/PLANETS,
  pagination. New games are recorded from the Python with a scratch TMPDIR;
  rare events found by seed search.

## 8. Order of work

| Step | Content | Rust lines | First tests passing |
|---|---|---|---|
| 0 | Tooling: harness fixes, RNG trace mode in both Python and Rust, golden vectors | ~300 | |
| 1 | Foundations: pyfmt, pymath, coord, rng (f64), io, scanner, cli | ~1,300 | 50, 63, format units |
| 2 | State, setup, command loop, chart; stubs for everything else | ~1,600 | **test01** |
| 3 | Instant commands: status, request, scans, report, score, help | ~900 | 12, 22, 59-61, 80 |
| 4 | Time core and enemy side: events, nova, attack, AI, finish | ~3,000 | 07, 16, 26, 31, 68 |
| 5 | Player weapons and movement | ~2,500 | most of the rest |
| 6 | Planets | ~450 | 05, 08, 13, 34-36, 39, 55, 56 |
| 7 | Saves, tournament, plaque, help asset, unit-test ports | ~700 + tests | 62, 73, 76, 82-84, 79 |
| 8 | Coverage push with newly recorded games | | |
| 9 | Ratatui interface | ~1,500-2,500 | |

About 9,000 to 10,000 lines of Rust plus 1,500 of tests. Steps 4 to 6 call
each other, so most recorded games pass only once 4 and 5 are both in.

**Parallel work** is safe after steps 1 and 2, which one agent does, since
they fix every shared type and stub every function with its final
signature. Then one module per agent (reports and finish; events and nova;
AI and enemy battle; player battle; movement; planets; saves, help and unit
tests), with state.rs frozen. Referees: the replay ratchet; RNG trace diffs
that name the module where a game first diverges; a check that every output
string in `sst` appears verbatim in the Rust; a per-function RNG call audit;
and a second agent reviewing each function against the Python, which matters
most for the code no recording reaches.

## 9. The Ratatui phase

SST2K's screen mode redraws scan, status, long-range scan and (above 40
rows) the chart before every command, and those redraws have side effects
on what the ship knows. Recommended: a **line-mode core with a UI shell**.
The core always runs with teletype semantics on its own thread; the UI draws
the panels from read-only snapshots taken at each prompt; input goes back
through the same input log. A game played in the UI then replays exactly
under `-r`.

## 10. Decisions for David

1. **Cross-platform exactness.** On Linux the port matches Python bit for
   bit. macOS and Windows maths libraries differ in rare last-bit cases,
   which can change a game. Accept "exact on Linux, near-exact elsewhere",
   or port glibc's maths routines.
2. **Python crash paths** (for example ABANDON next to an uninhabited planet
   crashes SST2K): reproduce the crash, or fix it and record the deviation?
3. **Known bugs** (section 4.15): keep verbatim (assumed) or fix?
4. **UI mode**: line-mode core with a UI shell (replays exactly), or a
   screen mode faithful to SST2K's side effects (does not replay in TTY)?
5. **Saves**: location (platform data folder vs the current directory) and
   name rules.
6. **Help asset**: generate `sst.doc` once with xmlto and embed it (help then
   always works, unlike SST2K run outside its install).
7. **Wayback quirks**: keep the BSDTREK/PLANETS mix-up and the sandbox docs
   mismatch, or fix them?
