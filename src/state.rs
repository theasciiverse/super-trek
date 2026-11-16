//! Game state: `Thingy`, `Planet`, `Quadrant`, `Page`, `Snapshot`, `Event`,
//! `Enemy`, `course` and `Gamestate` from `sst` (301-675, 4680-4727).
//!
//! Python object references become indices:
//! - planets: `Snapshot.planets[k]`; `Quadrant.planet` and `game.iplnet`
//!   hold `Some(k)`. A deep copy of a snapshot keeps the mapping, as
//!   Python's `deepcopy` keeps shared references.
//! - enemies: a grow-only arena `game.pool`; `game.enemies` lists the ids
//!   of the live list in order, and `game.tholian` is an id. Python keeps
//!   enemy objects alive after removal from the list, and that is
//!   observable, so ids stay valid for the whole game.

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Thingy {
    pub location: Coord,
}

impl Thingy {
    pub fn at(&self, q: Coord) -> bool {
        q == self.location
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Planet {
    pub name: Option<String>,
    pub quadrant: Coord,
    /// "M", "N", "O" or "destroyed" (None before setup)
    pub pclass: Option<String>,
    /// "mined", "present", "absent"
    pub crystals: String,
    /// "unknown", "known", "shuttle_down"
    pub known: String,
    pub inhabited: bool,
}

impl Default for Planet {
    fn default() -> Self {
        Planet {
            name: None,
            quadrant: Coord::none(),
            pclass: None,
            crystals: "absent".into(),
            known: "unknown".into(),
            inhabited: false,
        }
    }
}

impl Planet {
    pub fn pclass(&self) -> &str {
        self.pclass.as_deref().unwrap_or("None")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quadrant {
    pub stars: i64,
    /// index into `Snapshot.planets`
    pub planet: Option<usize>,
    pub starbase: bool,
    pub klingons: i64,
    pub romulans: i64,
    pub supernova: bool,
    pub charted: bool,
    /// "secure", "distressed", "enslaved"
    pub status: String,
}

impl Default for Quadrant {
    fn default() -> Self {
        Quadrant {
            stars: 0,
            planet: None,
            starbase: false,
            klingons: 0,
            romulans: 0,
            supernova: false,
            charted: false,
            status: "secure".into(),
        }
    }
}

/// One cell of the star chart.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Page {
    pub stars: Option<i64>,
    pub starbase: bool,
    pub klingons: Option<i64>,
    pub supernova: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub snap: bool,
    pub crew: i64,
    pub nscrem: i64,
    pub starkl: i64,
    pub basekl: i64,
    pub nromrem: i64,
    pub nplankl: i64,
    pub nworldkl: i64,
    pub planets: Vec<Planet>,
    pub date: f64,
    pub remres: f64,
    pub remtime: f64,
    pub baseq: Vec<Coord>,
    pub kcmdr: Vec<Coord>,
    pub kscmdr: Coord,
    pub galaxy: Vec<Vec<Quadrant>>,
    pub chart: Vec<Vec<Page>>,
}

impl Default for Snapshot {
    fn default() -> Self {
        Snapshot {
            snap: false,
            crew: 0,
            nscrem: 0,
            starkl: 0,
            basekl: 0,
            nromrem: 0,
            nplankl: 0,
            nworldkl: 0,
            planets: Vec::new(),
            date: 0.0,
            remres: 0.0,
            remtime: 0.0,
            baseq: Vec::new(),
            kcmdr: Vec::new(),
            kscmdr: Coord::none(),
            galaxy: vec![vec![Quadrant::default(); GS]; GS],
            chart: vec![vec![Page::default(); GS]; GS],
        }
    }
}

impl Snapshot {
    /// `traverse()`: every quadrant in row order.
    pub fn traverse(&self) -> impl Iterator<Item = (usize, usize, &Quadrant)> {
        (0..GS).flat_map(move |i| (0..GS).map(move |j| (i, j, &self.galaxy[i][j])))
    }
    pub fn gal(&self, c: Coord) -> &Quadrant {
        &self.galaxy[c.i as usize][c.j as usize]
    }
    pub fn gal_mut(&mut self, c: Coord) -> &mut Quadrant {
        &mut self.galaxy[c.i as usize][c.j as usize]
    }
    pub fn page(&self, c: Coord) -> &Page {
        &self.chart[c.i as usize][c.j as usize]
    }
    pub fn page_mut(&mut self, c: Coord) -> &mut Page {
        &mut self.chart[c.i as usize][c.j as usize]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Event {
    /// NaN stands for Python's None (before the first setup).
    pub date: f64,
    /// `Coord::none()` stands for None as well as for `Coord()`.
    pub quadrant: Coord,
}

impl Default for Event {
    fn default() -> Self {
        Event { date: f64::NAN, quadrant: Coord::none() }
    }
}

pub type EnemyId = usize;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Enemy {
    /// 'K', 'C', 'S', 'R', 'T', '?'
    pub etype: char,
    pub location: Coord,
    /// NaN stands for None
    pub kdist: f64,
    pub kavgd: f64,
    pub pending_remove: bool,
    pub power: f64,
}

/// `EnemyMove`: (left the quadrant, enemy, old location, old distance, goto)
#[derive(Clone, Copy, Debug)]
pub struct EnemyMove {
    pub left: bool,
    pub enemy: EnemyId,
    pub oldloc: Coord,
    pub olddist: f64,
    pub goto: Coord,
}

/// `course` (4680-4727). Positions are absolute (quadrant*10 + sector).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Course {
    pub distance: f64,
    pub bearing: f64,
    pub angle: f64,
    pub increment: Vec2,
    pub moves: i64,
    pub step: i64,
    pub origin: Vec2,
    pub location: Vec2,
    pub final_: Coord,
    pub nextlocation: Option<Vec2>,
}

impl Course {
    /// `course(bearing, distance, origin)`; origin is `cartesian()` when None
    /// (the caller passes it).
    pub fn new(bearing: f64, distance: f64, origin: Vec2) -> Course {
        let mut bearing = bearing;
        if bearing < 0.0 {
            bearing += 12.0;
        }
        let angle = (15.0 - bearing) * 0.5235988;
        let mut increment = Vec2::new(-angle.sin(), angle.cos());
        let bigger = increment.i.abs().max(increment.j.abs());
        increment = increment.div(bigger);
        let moves = crate::py::pyround(10.0 * distance * bigger);
        let location = origin;
        let final_ = location.add(increment.scale(moves as f64)).roundtogrid();
        Course { distance, bearing, angle, increment, moves, step: 0, origin, location, final_, nextlocation: None }
    }
    pub fn reset(&mut self) {
        self.location = self.origin;
        self.step = 0;
    }
    pub fn arrived(&self) -> bool {
        self.location.roundtogrid() == self.final_
    }
    pub fn nextstep(&mut self) -> bool {
        self.step += 1;
        let next = self.location.add(self.increment);
        self.nextlocation = Some(next);
        let samequad = self.location.quadrant() == next.quadrant();
        self.location = next;
        samequad
    }
    pub fn quadrant(&self) -> Coord {
        self.location.quadrant()
    }
    pub fn sector(&self) -> Coord {
        self.location.sector()
    }
    /// `power(w)`: `distance * w**3 * (shldup + 1)`
    pub fn power(&self, w: f64, shldup: bool) -> f64 {
        self.distance * crate::py::pypow(w, 3.0) * (shldup as i64 + 1) as f64
    }
    /// `time(w)`: `10.0 * distance / w**2`
    pub fn time(&self, w: f64) -> f64 {
        10.0 * self.distance / crate::py::pypow(w, 2.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Gamestate {
    pub options: i64,
    pub wayback_disabled: i64,
    pub state: Snapshot,
    pub snapsht: Snapshot,
    pub quad: [[char; QS]; QS],
    pub damage: [f64; NDEVICES],
    pub future: Vec<Event>,
    pub passwd: Option<String>,
    /// arena of every Enemy created this game (see module doc)
    pub pool: Vec<Enemy>,
    pub enemies: Vec<EnemyId>,
    pub quadrant: Coord,
    pub sector: Coord,
    pub tholian: Option<EnemyId>,
    /// None in Python is `Coord::none()` here for base, battle, plnet
    pub base: Coord,
    pub battle: Coord,
    pub plnet: Coord,
    pub gamewon: bool,
    pub ididit: bool,
    pub alive: bool,
    pub justin: bool,
    pub shldup: bool,
    pub shldchg: bool,
    pub iscate: bool,
    pub ientesc: bool,
    pub resting: bool,
    pub icraft: bool,
    pub landed: bool,
    pub alldone: bool,
    pub neutz: bool,
    pub isarmed: bool,
    pub inorbit: bool,
    pub imine: bool,
    pub icrystl: bool,
    pub iseenit: bool,
    pub thawed: bool,
    pub sandbox: bool,
    /// "green", "yellow", "red", "docked", "dead" ("" for None)
    pub condition: String,
    /// "onship", "offship", "removed" ("" for None)
    pub iscraft: String,
    pub skill: i64,
    pub inkling: i64,
    pub inbase: i64,
    pub incom: i64,
    pub inscom: i64,
    pub inrom: i64,
    pub instar: i64,
    pub intorps: i64,
    pub torps: i64,
    /// 'E' Enterprise, 'F' Faerie Queene ('\0' for the initial 0)
    pub ship: char,
    pub abandoned: i64,
    pub length: i64,
    pub klhere: i64,
    pub casual: i64,
    pub nhelp: i64,
    pub nkinks: i64,
    /// index into `state.planets`
    pub iplnet: Option<usize>,
    pub inplan: i64,
    pub irhere: i64,
    pub isatb: i64,
    pub tourn: Option<i64>,
    pub nprobes: i64,
    pub inresor: f64,
    pub intime: f64,
    pub inenrg: f64,
    pub inshld: f64,
    pub inlsr: f64,
    pub indate: f64,
    pub energy: f64,
    pub shield: f64,
    pub warpfac: f64,
    pub lsupres: f64,
    pub optime: f64,
    pub damfac: f64,
    pub lastchart: f64,
    pub cryprob: f64,
    pub probe: Option<Course>,
    pub height: f64,
    pub score: f64,
    pub perdate: f64,
    pub idebug: bool,
    pub cdebug: bool,
    pub brigcapacity: i64,
    pub brigfree: i64,
    pub kcaptured: i64,
    pub iscloaked: bool,
    pub ncviol: i64,
    pub isviolreported: bool,
    pub lcg_x: f64,
}

impl Default for Gamestate {
    fn default() -> Self {
        Gamestate {
            options: 0,
            wayback_disabled: 0,
            state: Snapshot::default(),
            snapsht: Snapshot::default(),
            quad: [['.'; QS]; QS],
            damage: [0.0; NDEVICES],
            future: vec![Event::default(); NEVENTS],
            passwd: None,
            pool: Vec::new(),
            enemies: Vec::new(),
            quadrant: Coord::none(),
            sector: Coord::none(),
            tholian: None,
            base: Coord::none(),
            battle: Coord::none(),
            plnet: Coord::none(),
            gamewon: false,
            ididit: false,
            alive: false,
            justin: false,
            shldup: false,
            shldchg: false,
            iscate: false,
            ientesc: false,
            resting: false,
            icraft: false,
            landed: false,
            alldone: false,
            neutz: false,
            isarmed: false,
            inorbit: false,
            imine: false,
            icrystl: false,
            iseenit: false,
            thawed: false,
            sandbox: false,
            condition: String::new(),
            iscraft: String::new(),
            skill: SKILL_NONE,
            inkling: 0,
            inbase: 0,
            incom: 0,
            inscom: 0,
            inrom: 0,
            instar: 0,
            intorps: 0,
            torps: 0,
            ship: '\0',
            abandoned: 0,
            length: 0,
            klhere: 0,
            casual: 0,
            nhelp: 0,
            nkinks: 0,
            iplnet: None,
            inplan: 0,
            irhere: 0,
            isatb: 0,
            tourn: None,
            nprobes: 0,
            inresor: 0.0,
            intime: 0.0,
            inenrg: 0.0,
            inshld: 0.0,
            inlsr: 0.0,
            indate: 0.0,
            energy: 0.0,
            shield: 0.0,
            warpfac: 0.0,
            lsupres: 0.0,
            optime: 0.0,
            damfac: 0.0,
            lastchart: 0.0,
            cryprob: 0.0,
            probe: None,
            height: 0.0,
            score: 0.0,
            perdate: 0.0,
            idebug: false,
            cdebug: false,
            brigcapacity: 400,
            brigfree: 400,
            kcaptured: 0,
            iscloaked: false,
            ncviol: 0,
            isviolreported: false,
            lcg_x: 0.0,
        }
    }
}

impl Gamestate {
    /// `remkl()`: Klingons left in the galaxy.
    pub fn remkl(&self) -> i64 {
        self.state.traverse().map(|(_, _, q)| q.klingons).sum()
    }

    /// `recompute()`. Division by zero is a Python crash; the caller guards.
    pub fn recompute(&mut self) {
        if self.sandbox {
            self.state.remtime = 9999.0;
            return;
        }
        self.state.remtime = self.state.remres / (self.remkl() + 4 * self.state.kcmdr.len() as i64) as f64;
    }

    /// `unwon()`: Klingons remaining.
    pub fn unwon(&self) -> i64 {
        self.remkl()
    }

    pub fn damaged(&self, dev: usize) -> bool {
        self.damage[dev] != 0.0
    }

    pub fn communicating(&self) -> bool {
        !self.damaged(DRADIO) || self.condition == "docked"
    }

    pub fn e(&self, id: EnemyId) -> &Enemy {
        &self.pool[id]
    }
    pub fn e_mut(&mut self, id: EnemyId) -> &mut Enemy {
        &mut self.pool[id]
    }
    pub fn q(&self, c: Coord) -> char {
        self.quad[c.i as usize][c.j as usize]
    }
    pub fn set_q(&mut self, c: Coord, ch: char) {
        self.quad[c.i as usize][c.j as usize] = ch;
    }
    /// The quadrant the ship is in.
    pub fn here(&self) -> &Quadrant {
        self.state.gal(self.quadrant)
    }
    pub fn here_mut(&mut self) -> &mut Quadrant {
        let q = self.quadrant;
        self.state.gal_mut(q)
    }
    /// The Planet `game.iplnet` refers to.
    pub fn iplnet(&self) -> Option<&Planet> {
        self.iplnet.map(|k| &self.state.planets[k])
    }
    pub fn iplnet_mut(&mut self) -> Option<&mut Planet> {
        self.iplnet.map(move |k| &mut self.state.planets[k])
    }
}
