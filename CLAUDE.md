# Super Trek

A Rust port of Super Star Trek, the 1970s teletype classic, for the
Asciiverse catalog, under the name Super Trek. Historic preservation: the
game's rules, text and behaviour are kept exactly; the port adds a native
binary on every platform and a Ratatui interface around the classic typed
commands.

Status: started 2026-10-10. Design, credits, random generator and test
harness first; the game itself is translated after that.

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
