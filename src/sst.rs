//! The `Sst` struct that owns everything the Python keeps in globals, and
//! the small helpers every module uses (enemy objects, event schedule,
//! names of things).

use crate::consts::*;
use crate::coord::Coord;
use crate::io::Io;
use crate::py::pyfmt;
use crate::scanner::Scanner;
use crate::state::*;

/// `raise SystemExit(code)` / `sys.exit(code)`: unwinds to `main`.
#[derive(Debug, Clone, Copy)]
pub struct SystemExit(pub i32);

/// `raise TrekError` (bad course input).
#[derive(Debug, Clone, Copy)]
pub struct TrekError;

pub struct Sst {
    pub game: Gamestate,
    /// outside `game`: never saved, survives thaw and new games
    pub thing: Thingy,
    pub scanner: Scanner,
    pub io: Io,
}

impl Default for Sst {
    fn default() -> Self {
        Self::new()
    }
}

impl Sst {
    pub fn new() -> Self {
        Sst { game: Gamestate::default(), thing: Thingy::default(), scanner: Scanner::new(), io: Io::default() }
    }

    /// Keep the copy of the options that Coord's Display reads current.
    pub fn sync_options(&self) {
        crate::coord::OPTIONS.with(|o| o.set(self.game.options));
    }

    pub fn set_options(&mut self, v: i64) {
        self.game.options = v;
        self.sync_options();
    }

    pub fn damaged(&self, dev: usize) -> bool {
        self.game.damaged(dev)
    }

    pub fn communicating(&self) -> bool {
        self.game.communicating()
    }

    // ---- Enemy objects (522-555) ----

    /// `Enemy(etype, loc, power)`: creates it, places it, appends it to
    /// `game.enemies`.
    pub fn new_enemy(&mut self, etype: char, loc: Option<Coord>, power: f64) -> EnemyId {
        let id = self.game.pool.len();
        self.game.pool.push(Enemy {
            etype,
            location: Coord::none(),
            kdist: f64::NAN,
            kavgd: f64::NAN,
            pending_remove: false,
            power: f64::NAN,
        });
        if let Some(l) = loc {
            self.enemy_move(id, Some(l));
        }
        self.game.pool[id].power = power;
        self.game.enemies.push(id);
        id
    }

    /// `Enemy.move(loc)`; `None` removes it from the quadrant and the list.
    pub fn enemy_move(&mut self, id: EnemyId, loc: Option<Coord>) -> bool {
        let old = self.game.pool[id].location;
        let motion = match loc {
            Some(l) => l != old,
            None => true, // `loc != self.location` with loc None
        };
        if !old.is_none() && motion {
            let ch = if self.game.pool[id].etype == 'T' { '#' } else { '.' };
            self.game.set_q(old, ch);
        }
        match loc {
            Some(l) => {
                let et = self.game.pool[id].etype;
                self.game.pool[id].location = l;
                self.game.set_q(l, et);
                let d = self.game.sector.sub(l).distance();
                self.game.pool[id].kdist = d;
                self.game.pool[id].kavgd = d;
            }
            None => {
                self.game.pool[id].location = Coord::none();
                self.game.pool[id].kdist = f64::NAN;
                self.game.pool[id].kavgd = f64::NAN;
                if let Some(pos) = self.game.enemies.iter().position(|&e| e == id) {
                    self.game.enemies.remove(pos);
                }
            }
        }
        motion
    }

    /// `repr(enemy)`: "<T,a1,123.000000>".
    pub fn enemy_repr(&self, id: EnemyId) -> String {
        let e = &self.game.pool[id];
        pyfmt("<%s,%s,%f>", &[e.etype.into(), e.location.into(), e.power.into()])
    }

    /// `sortenemies()`: stable sort by distance.
    pub fn sortenemies(&mut self) {
        let pool = &self.game.pool;
        self.game.enemies.sort_by(|a, b| pool[*a].kdist.partial_cmp(&pool[*b].kdist).unwrap_or(std::cmp::Ordering::Equal));
    }

    // ---- Event schedule (2550-2576) ----

    pub fn unschedule(&mut self, ev: usize) {
        self.game.future[ev].date = FOREVER;
    }
    pub fn is_scheduled(&self, ev: usize) -> bool {
        self.game.future[ev].date != FOREVER
    }
    pub fn scheduled(&self, ev: usize) -> f64 {
        self.game.future[ev].date
    }
    pub fn schedule(&mut self, ev: usize, offset: f64) {
        self.game.future[ev].date = self.game.state.date + offset;
    }
    pub fn postpone(&mut self, ev: usize, offset: f64) {
        self.game.future[ev].date += offset;
    }

    // ---- Names (7777-7841) ----

    pub fn crmshp(&self) -> String {
        match self.game.ship {
            'E' => "Enterprise".into(),
            'F' => "Faerie Queene".into(),
            _ => "Ship???".into(),
        }
    }

    /// `expran(avrage)`
    pub fn expran(&mut self, avrage: f64) -> f64 {
        let r = self.rnd_real();
        -avrage * (1e-7 + r).ln()
    }

    /// `randplace(size)`
    pub fn randplace(&mut self, size: i64) -> Coord {
        let i = self.rnd_integer(size as f64);
        let j = self.rnd_integer(size as f64);
        Coord::new(i, j)
    }

    /// `Coord.scatter()`
    pub fn scatter(&mut self, c: Coord) -> Coord {
        let i = c.i + self.rnd_integer2(-1.0, 2.0);
        let j = c.j + self.rnd_integer2(-1.0, 2.0);
        Coord::new(i, j)
    }

    /// Log a one-line snapshot of the state (session logging convention).
    pub fn log_state(&self, why: &str) {
        if !log::log_enabled!(log::Level::Debug) {
            return;
        }
        let g = &self.game;
        log::debug!(target: "state",
            "{why}: date={:.2} quad={} sect={} cond={} energy={:.1} shield={:.1} up={} torps={} kl_here={} enemies={} remkl={} remtime={:.2} rng={}",
            g.state.date, g.quadrant, g.sector, g.condition, g.energy, g.shield, g.shldup, g.torps, g.klhere,
            g.enemies.len(), g.remkl(), g.state.remtime, g.lcg_x);
    }
}

/// `cramen(ch)`: the name of an enemy or feature.
pub fn cramen(ch: char) -> &'static str {
    match ch {
        'R' => "Romulan",
        'K' => "Klingon",
        'C' => "Commander",
        'S' => "Super-commander",
        '*' => "Star",
        'P' => "Planet",
        'B' => "Starbase",
        ' ' => "Black hole",
        'T' => "Tholian",
        '#' => "Tholian web",
        '?' => "Stranger",
        '@' => "Inhabited World",
        _ => "Unknown??",
    }
}

/// `crmena(loud, enemy, loctype, w)` where `w` is given already as its
/// `repr()` text (a Coord's `to_string()`, or `enemy_repr` for the one call
/// that passes an Enemy).
pub fn crmena(loud: bool, enemy: char, loctype: &str, w: &str) -> String {
    let mut buf = String::new();
    if loud {
        buf.push_str("***");
    }
    buf.push_str(cramen(enemy));
    buf.push_str(" at ");
    if loctype == "quadrant" {
        buf.push_str("Quadrant ");
    } else if loctype == "sector" {
        buf.push_str("Sector ");
    }
    buf + w
}
