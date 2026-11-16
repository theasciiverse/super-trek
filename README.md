# Super Trek

*Stardate 3188. Fifteen Klingon battle cruisers are loose in the galaxy, a
Super-Commander among them, and you have seven stardates to stop them. Four
starbases, ten photon torpedoes, five thousand units of energy. Good luck.
You'll need it.*

You command the U.S.S. Enterprise across an eight by eight galaxy of
quadrants, each a ten by ten grid of sectors where stars, black holes,
starbases, planets and enemy ships sit. You hunt by long-range scan, jump by
warp, and fight sector by sector with phasers and torpedoes, while the clock
runs: Commanders tractor-beam you into ambushes, stars go supernova, bases
fall under siege and call for help, Romulans guard their Neutral Zone, a
Tholian spins its web around you, and somewhere out there drifts something
that should not be poked. Every order is a typed command, as it was on a
teletype in 1974, and every one of them costs time, energy or both.

## Where it comes from

In 1973, at the University of Texas at Austin, David Matuszek and Paul
Reynolds were playing Grady Hicks' Star Trek in BASIC on the campus CDC 6600
and grew tired of how long it took to load and how slowly it ran, so they
wrote their own, in FORTRAN, through 1973 and 1974. It grew ten by ten
sectors, command words in place of numbers, and later, from Don Smith,
Marc Newman and others, black holes, the Tholian web, the Super-Commander
and Emeritus play. Its last FORTRAN banner read "Latest update-21 Sept 78".
That game travelled: Eric Allman's BSD Trek descends from it, and so does
this one.

Around 1977 Tom Almy got a copy, squeezed it onto a 56k PDP-11 with
overlays, fixed its loopholes and ran tournaments. In 1995, missing an old
friend, he spent more than a year turning his saved listing into portable
C. Eric S. Raymond built on Almy's C from 2004 as SST2K, translated it to
Python in 2006 with Dave Matuszek involved, and has kept it going since,
with planets and dilithium mining, deep-space probes, cloaking, capture and
inhabited worlds folded in over the years.

Super Trek is a faithful Rust port of SST2K 2.10: the same rules, the same
random numbers, the same words, checked against ESR's own recorded games
byte for byte. It adds a bridge screen around the classic typed commands.

## Playing

```
cargo run --release
```

In a terminal of 80 by 24 or more the game opens on the bridge: short-range
scan, status and long-range scan across the top (and the star chart on tall
terminals), the ship's log and your command line below. Type commands as
the 1970s player did: `help` lists them, `help <command>` explains one.
PgUp and PgDn scroll the log, the arrow keys recall earlier commands, and
Ctrl-C leaves.

`super-trek --tty` (or `-t`) plays in the original plain line mode, and so
does any run whose input or output is not a terminal.

Saved games (`save`, `freeze`) go to `~/.local/state/super-trek` unless you
name a folder. Every session's typed input is kept in
`~/.cache/super-trek/input.log`, which replays the game exactly with
`super-trek -r`.

## Credits

- David Matuszek and Paul Reynolds, the original UT Super Star Trek
  (FORTRAN, 1973 to 1978), with modifications by Don Smith and others.
- Tom Almy, the C translation (1995 on) and many fixes and features.
- Eric S. Raymond, SST2K (2004 to 2026), the version this port follows.
- Stas Sergeev, base shields and the black-hole time warp in SST2K.
- The CAPTURE command comes from BSD trek (University of California,
  Berkeley).

The original authors' terms: copying, distribution, modification and use
for recreational purposes, with these credits kept, and never for sale or
profit. Super Trek is free and always will be. See `NOTICE` for the full
terms.
