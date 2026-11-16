//! Enemy movement: welcoming is in setup.rs; tryexit, movebaddy, moveklings, movescom, supercommander, movetholian (`sst` 704-1218).

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::py::*;
use crate::sst::{Sst, crmena};
use crate::state::*;

/// `Coord.sgn()` of a Coord whose parts are `motion * m` (float or int):
/// 0 when zero (also -0.0), -1 when negative, else 1.
fn sgnf(v: f64) -> i64 {
    if v == 0.0 {
        0
    } else if v < 0.0 {
        -1
    } else {
        1
    }
}

impl Sst {
    /// `tryexit(enemy, look, irun, defer_remove)`: a bad guy attempts to bug out.
    pub fn tryexit(&mut self, enemy: EnemyId, look: Coord, irun: bool, defer_remove: bool) -> Vec<EnemyMove> {
        // Zero-based sectors cross a quadrant edge at -1 or QUADSIZE.
        let iq = Coord::new(
            self.game.quadrant.i + look.i.div_euclid(QUADSIZE),
            self.game.quadrant.j + look.j.div_euclid(QUADSIZE),
        );
        if !self.welcoming(iq) {
            return vec![];
        }
        let etype = self.game.pool[enemy].etype;
        if etype == 'R' {
            return vec![]; // Romulans cannot escape!
        }
        if !irun {
            // avoid intruding on another commander's territory
            if etype == 'C' {
                if self.game.state.kcmdr.contains(&iq) {
                    return vec![];
                }
                // refuse to leave if currently attacking starbase
                if self.game.battle == self.game.quadrant {
                    return vec![];
                }
            }
            // don't leave if over 1000 units of energy
            if self.game.pool[enemy].power > 1000.0 {
                return vec![];
            }
        }
        let oldloc = self.game.pool[enemy].location;
        // handle local matters related to escape
        if defer_remove {
            if !oldloc.is_none() {
                if etype == 'T' {
                    self.game.set_q(oldloc, '#');
                } else {
                    self.game.set_q(oldloc, '.');
                }
            }
            let e = &mut self.game.pool[enemy];
            e.location = Coord::none();
            e.kdist = f64::NAN;
            e.kavgd = f64::NAN;
            e.pending_remove = true;
        } else {
            self.enemy_move(enemy, None);
        }
        self.game.klhere -= 1;
        if self.game.condition != "docked" {
            self.newcnd();
        }
        // Handle global matters related to escape
        let q = self.game.quadrant;
        self.game.state.gal_mut(q).klingons -= 1;
        self.game.state.gal_mut(iq).klingons += 1;
        if etype == 'S' {
            self.game.iscate = false;
            self.game.ientesc = false;
            self.game.isatb = 0;
            self.schedule(FSCMOVE, 0.2777);
            self.unschedule(FSCDBAS);
            self.game.state.kscmdr = iq;
        } else if etype == 'C' && self.game.state.kcmdr.contains(&q) {
            // Relocate the commander without changing its index or the total count.
            let idx = self.game.state.kcmdr.iter().position(|c| *c == q).unwrap();
            self.game.state.kcmdr[idx] = iq;
        }
        // report move out of quadrant (kdist is None by now in the Python too)
        let olddist = self.game.pool[enemy].kdist;
        vec![EnemyMove { left: true, enemy, oldloc, olddist, goto: iq }]
    }

    /// `movebaddy(enemy)`: tactical movement for the bad guys.
    pub fn movebaddy(&mut self, enemy: EnemyId) -> Vec<EnemyMove> {
        let mut irun = false;
        let incmdr = self.game.state.kcmdr.contains(&self.game.quadrant) as i64;
        let isscom = (self.game.state.kscmdr == self.game.quadrant) as i64;
        let nbaddys: i64 = if self.game.skill >= SKILL_EXPERT {
            trunc(
                ((incmdr * 2 + isscom * 2) as f64 + self.game.klhere as f64 * 1.23 + self.game.irhere as f64 * 1.5)
                    / 2.0,
            )
        } else {
            incmdr + isscom
        };
        let old_dist = self.game.pool[enemy].kdist;
        let mdist = trunc(old_dist + 0.5); // Nearest integer distance
        let etype = self.game.pool[enemy].etype;
        let power = self.game.pool[enemy].power;
        // motion holds the Python value, int or float; only its sign, its
        // truncation and %d of it are ever observed.
        let mut motion: f64;
        // If SC, check with spy to see if should hi-tail it
        if etype == 'S' && (power <= 500.0 || (self.game.condition == "docked" && !self.damaged(DPHOTON))) {
            irun = true;
            motion = -(QUADSIZE as f64);
        } else {
            // decide whether to advance, retreat, or hold position
            let mut forces = power + 100.0 * self.game.enemies.len() as f64 + (400 * (nbaddys - 1)) as f64;
            if !self.game.shldup {
                forces += 1000.0; // Good for enemy if shield is down!
            }
            if !self.damaged(DPHASER) || !self.damaged(DPHOTON) {
                if self.damaged(DPHASER) {
                    // phasers damaged
                    forces += 300.0;
                } else {
                    forces -= 0.2 * (self.game.energy - 2500.0);
                }
                if self.damaged(DPHOTON) {
                    // photon torpedoes damaged
                    forces += 300.0;
                } else {
                    forces -= 50.0 * self.game.torps as f64;
                }
            } else {
                // phasers and photon tubes both out!
                forces += 1000.0;
            }
            motion = 0.0;
            if forces <= 1000.0 && self.game.condition != "docked" {
                // Typical situation
                let r = self.rnd_real1(200.0);
                motion = ((forces + r) / 150.0) - 5.0;
            } else {
                if forces > 1000.0 {
                    // Very strong -- move in for kill
                    let r = self.rnd_real();
                    motion = pypow(1.0 - r, 2.0) * old_dist + 1.0;
                }
                if self.game.condition == "docked" && (self.game.options & OPTION_BASE) != 0 {
                    // protected by base -- back off !
                    let skill = self.game.skill as f64;
                    let r = self.rnd_real();
                    motion -= skill * (2.0 - pypow(r, 2.0));
                }
            }
            if self.game.idebug {
                let s = pf!("=== MOTION = %d, FORCES = %1.2f, ", motion, forces);
                self.proutn(&s);
            }
            // don't move if no motion
            if motion == 0.0 {
                return vec![];
            }
            // Limit motion according to skill
            if motion.abs() > self.game.skill as f64 {
                if motion < 0.0 {
                    motion = -self.game.skill as f64;
                } else {
                    motion = self.game.skill as f64;
                }
            }
        }
        // calculate preferred number of steps
        let mut nsteps = trunc(motion).abs();
        if motion > 0.0 && nsteps > mdist {
            nsteps = mdist; // don't overshoot
        }
        nsteps = nsteps.min(QUADSIZE); // This shouldn't be necessary
        nsteps = nsteps.max(1); // This shouldn't be necessary
        if self.game.idebug {
            let s = pf!("NSTEPS = %d:", nsteps);
            self.proutn(&s);
        }
        // Compute preferred values of delta X and Y
        let eloc = self.game.pool[enemy].location;
        let mut m = self.game.sector.sub(eloc);
        if 2.0 * (m.i.abs() as f64) < m.j.abs() as f64 {
            m.i = 0;
        }
        if 2.0 * (m.j.abs() as f64) < (self.game.sector.i - eloc.i).abs() as f64 {
            m.j = 0;
        }
        m = Coord::new(sgnf(motion * m.i as f64), sgnf(motion * m.j as f64));
        let mut goto = eloc;
        // main move loop
        for ll in 0..nsteps {
            if self.game.idebug {
                let s = pf!(" %d", ll + 1);
                self.proutn(&s);
            }
            // Check if preferred position available
            let mut look = goto.add(m);
            let mut krawli: i64 = if m.i < 0 { 1 } else { -1 };
            let mut krawlj: i64 = if m.j < 0 { 1 } else { -1 };
            let mut success = false;
            let mut attempts = 0; // Settle mysterious hang problem
            while attempts < 20 && !success {
                attempts += 1;
                if look.i < 0 || look.i >= QUADSIZE {
                    if motion < 0.0 {
                        return self.tryexit(enemy, look, irun, true);
                    }
                    if krawli == m.i || m.j == 0 {
                        break;
                    }
                    look.i = goto.i + krawli;
                    krawli = -krawli;
                } else if look.j < 0 || look.j >= QUADSIZE {
                    if motion < 0.0 {
                        return self.tryexit(enemy, look, irun, true);
                    }
                    if krawlj == m.j || m.i == 0 {
                        break;
                    }
                    look.j = goto.j + krawlj;
                    krawlj = -krawlj;
                } else if (self.game.options & OPTION_RAMMING) != 0 && self.game.q(look) != '.' {
                    // See if enemy should ram ship
                    if self.game.q(look) == self.game.ship && (etype == 'C' || etype == 'S') {
                        self.collision(true, enemy);
                        return vec![];
                    }
                    if krawli != m.i && m.j != 0 {
                        look.i = goto.i + krawli;
                        krawli = -krawli;
                    } else if krawlj != m.j && m.i != 0 {
                        look.j = goto.j + krawlj;
                        krawlj = -krawlj;
                    } else {
                        break; // we have failed
                    }
                } else {
                    success = true;
                }
            }
            if success {
                goto = look;
                if self.game.idebug {
                    let s = goto.to_string();
                    self.proutn(&s);
                }
            } else {
                break; // done early
            }
        }
        if self.game.idebug {
            self.skip(1);
        }
        // Enemy moved, but is still in sector
        let oldloc = self.game.pool[enemy].location;
        vec![EnemyMove { left: false, enemy, oldloc, olddist: old_dist, goto }]
    }

    /// `moveklings()`: sequence Klingon tactical movement.
    pub fn moveklings(&mut self) -> Vec<EnemyMove> {
        if self.game.idebug {
            self.prout("== MOVCOM");
        }
        // Figure out which Klingon is the commander (or Supercommander)
        // and do move. The loops follow Python's list iterator over the
        // live list: a collision can remove an entry mid-loop.
        let mut tacmoves = Vec::new();
        if self.game.state.kcmdr.contains(&self.game.quadrant) {
            let mut k = 0;
            while k < self.game.enemies.len() {
                let e = self.game.enemies[k];
                k += 1;
                if self.game.pool[e].etype == 'C' {
                    let mv = self.movebaddy(e);
                    tacmoves.extend(mv);
                }
            }
        }
        if self.game.state.kscmdr == self.game.quadrant {
            let mut k = 0;
            while k < self.game.enemies.len() {
                let e = self.game.enemies[k];
                k += 1;
                if self.game.pool[e].etype == 'S' {
                    let mv = self.movebaddy(e);
                    tacmoves.extend(mv);
                    break;
                }
            }
        }
        // If skill level is high, move other Klingons and Romulans too!
        // Move these last so they can base their actions on what the
        // commander(s) do.
        if self.game.skill >= SKILL_EXPERT && (self.game.options & OPTION_MVBADDY) != 0 {
            let mut k = 0;
            while k < self.game.enemies.len() {
                let e = self.game.enemies[k];
                k += 1;
                let t = self.game.pool[e].etype;
                if t == 'K' || t == 'R' {
                    let mv = self.movebaddy(e);
                    tacmoves.extend(mv);
                }
            }
        }
        let pool = &self.game.pool;
        if self.game.enemies.iter().any(|&e| pool[e].pending_remove) {
            self.game.enemies.retain(|&e| !pool[e].pending_remove);
        }
        tacmoves
    }

    /// `movescom(iq, avoid)`: Supercommander movement helper.
    pub fn movescom(&mut self, iq: Coord, avoid: bool) -> bool {
        // Strategic movement must not enter the Enterprise's quadrant.
        if iq == self.game.quadrant {
            return false;
        }
        // Avoid quadrants with bases if we want to avoid Enterprise
        if !self.welcoming(iq) || (avoid && self.game.state.baseq.contains(&iq)) {
            return false;
        }
        if self.game.justin && !self.game.iscate {
            return false;
        }
        // Remember the source before updating the Supercommander's galaxy location.
        let was_here = self.game.state.kscmdr == self.game.quadrant;
        let old = self.game.state.kscmdr;
        self.game.state.gal_mut(old).klingons -= 1;
        // Python makes kscmdr the caller's `iq` object; supercommander()
        // never mutates iq after a successful move, so a copy is the same.
        self.game.state.kscmdr = iq;
        self.game.state.gal_mut(iq).klingons += 1;
        if was_here {
            // SC has scooted, remove him from current quadrant
            self.game.iscate = false;
            self.game.isatb = 0;
            self.game.ientesc = false;
            self.unschedule(FSCDBAS);
            let found = self.game.enemies.iter().copied().find(|&e| self.game.pool[e].etype == 'S');
            if let Some(e) = found {
                self.enemy_move(e, None);
            }
            self.game.klhere -= 1;
            if self.game.condition != "docked" {
                self.newcnd();
            }
            self.sortenemies();
        }
        // check for a helpful planet
        for i in 0..self.game.inplan.max(0) as usize {
            let kscmdr = self.game.state.kscmdr;
            let p = &self.game.state.planets[i];
            if p.quadrant == kscmdr && p.crystals == "present" {
                // destroy the planet
                self.game.state.planets[i].pclass = Some("destroyed".into());
                self.game.state.gal_mut(kscmdr).planet = None;
                if self.communicating() {
                    self.announce();
                    self.prout("Lt. Uhura-  \"Captain, Starfleet Intelligence reports");
                    let s = pf!("   a planet in Quadrant %s has been destroyed", kscmdr);
                    self.prout(&s);
                    self.prout("   by the Super-commander.\"");
                }
                break;
            }
        }
        true // looks good!
    }

    /// `supercommander()`: move the Super Commander.
    pub fn supercommander(&mut self) {
        let mut idelta: Coord;
        let mut basetbl: Vec<(usize, f64)> = Vec::new();
        if self.game.idebug {
            self.prout("== SUPERCOMMANDER");
        }
        // Decide on being active or passive
        let skill = self.game.skill as f64;
        let avoid = ((self.game.incom - self.game.state.kcmdr.len() as i64 + self.game.inkling - self.game.remkl())
            as f64
            / (self.game.state.date + 0.01 - self.game.indate)
            < 0.1 * skill * (skill + 1.0))
            || (self.game.state.date - self.game.indate) < 3.0;
        if !self.game.iscate && avoid {
            // compute move away from Enterprise
            idelta = self.game.state.kscmdr.sub(self.game.quadrant);
            if idelta.distance() > 2.0 {
                // circulate in space
                idelta.i = self.game.state.kscmdr.j - self.game.quadrant.j;
                idelta.j = self.game.quadrant.i - self.game.state.kscmdr.i;
            }
        } else {
            // compute distances to starbases
            if self.game.state.baseq.is_empty() {
                // nothing left to do
                self.unschedule(FSCMOVE);
                return;
            }
            let sc = self.game.state.kscmdr;
            for (i, base) in self.game.state.baseq.iter().enumerate() {
                basetbl.push((i, base.sub(sc).distance()));
            }
            if self.game.state.baseq.len() > 1 {
                // stable, like Python's sort
                basetbl.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            }
            // look for nearest base without a commander, no Enterprise, and
            // without too many Klingons, and not already under attack.
            let mut ifindit = 0;
            let mut iwhichb = 0usize;
            for i2 in 0..self.game.state.baseq.len() {
                let base = self.game.state.baseq[i2];
                let i = basetbl[i2].0; // bug in original had it not finding nearest
                if base == self.game.quadrant || base == self.game.battle || !self.welcoming(base) {
                    continue;
                }
                // if there is a commander, and no other base is appropriate,
                // we will take the one with the commander
                let mut broke = false;
                for cmdr in &self.game.state.kcmdr {
                    if base == *cmdr && ifindit != 2 {
                        ifindit = 2;
                        iwhichb = i;
                        broke = true;
                        break;
                    }
                }
                if !broke {
                    // no commander -- use this one
                    ifindit = 1;
                    iwhichb = i;
                    break;
                }
            }
            if ifindit == 0 {
                return; // Nothing suitable -- wait until next time
            }
            let ibq = self.game.state.baseq[iwhichb];
            // decide how to move toward base
            idelta = ibq.sub(self.game.state.kscmdr);
        }
        // Maximum movement is 1 quadrant in either or both axes
        idelta = idelta.sgn();
        // try moving in both x and y directions
        let mut iq = self.game.state.kscmdr.add(idelta);
        if !self.movescom(iq, avoid) {
            // failed -- try some other maneuvers
            if idelta.i == 0 || idelta.j == 0 {
                // attempt angle move
                if idelta.i != 0 {
                    iq.j = self.game.state.kscmdr.j + 1;
                    if !self.movescom(iq, avoid) {
                        iq.j = self.game.state.kscmdr.j - 1;
                        self.movescom(iq, avoid);
                    }
                } else if idelta.j != 0 {
                    iq.i = self.game.state.kscmdr.i + 1;
                    if !self.movescom(iq, avoid) {
                        iq.i = self.game.state.kscmdr.i - 1;
                        self.movescom(iq, avoid);
                    }
                }
            } else {
                // try moving just in x or y
                iq.j = self.game.state.kscmdr.j;
                if !self.movescom(iq, avoid) {
                    iq.j = self.game.state.kscmdr.j + idelta.j;
                    iq.i = self.game.state.kscmdr.i;
                    self.movescom(iq, avoid);
                }
            }
        }
        // check for a base
        if self.game.state.baseq.is_empty() {
            self.unschedule(FSCMOVE);
        } else {
            for k in 0..self.game.state.baseq.len() {
                let ibq = self.game.state.baseq[k];
                if ibq == self.game.state.kscmdr && self.game.state.kscmdr != self.game.battle {
                    // Attack unless a commander is already attacking this base.
                    if avoid {
                        return; // no, don't attack base!
                    }
                    self.game.iseenit = false;
                    self.game.isatb = 1;
                    let r = self.rnd_real2(1.0, 3.0);
                    self.schedule(FSCDBAS, r);
                    if self.is_scheduled(FCDBAS) {
                        let d = self.scheduled(FCDBAS) - self.game.state.date;
                        self.postpone(FSCDBAS, d);
                    }
                    if !self.communicating() {
                        return; // no warning
                    }
                    self.game.iseenit = true;
                    self.announce();
                    let s = pf!("Lt. Uhura-  \"Captain, the starbase in Quadrant %s", self.game.state.kscmdr);
                    self.prout(&s);
                    self.prout("   reports that it is under attack from the Klingon Super-commander.");
                    let s = pf!("   It can survive until stardate %d.\"", trunc(self.scheduled(FSCDBAS)));
                    self.prout(&s);
                    if !self.game.resting {
                        return;
                    }
                    self.prout("Mr. Spock-  \"Captain, shall we cancel the rest period?\"");
                    if !self.ja() {
                        return;
                    }
                    self.game.resting = false;
                    self.game.optime = 0.0; // actually finished
                    return;
                }
            }
        }
        // Check for intelligence report
        if !self.game.idebug
            && (self.withprob(0.8)
                || !self.communicating()
                || !self.game.state.gal(self.game.state.kscmdr).charted)
        {
            return;
        }
        self.announce();
        self.prout("Lt. Uhura-  \"Captain, Starfleet Intelligence reports");
        let s = pf!("   the Super-commander is in Quadrant %s.", self.game.state.kscmdr);
        self.prout(&s);
    }

    /// `movetholian()`: move the Tholian.
    pub fn movetholian(&mut self) {
        let Some(t) = self.game.tholian else {
            return;
        };
        if self.game.justin {
            return;
        }
        let loc = self.game.pool[t].location;
        let last = QUADSIZE - 1;
        let tid = if loc.i == 0 && loc.j == 0 {
            Coord::new(0, last)
        } else if loc.i == 0 && loc.j == last {
            Coord::new(last, last)
        } else if loc.i == last && loc.j == last {
            Coord::new(last, 0)
        } else if loc.i == last && loc.j == 0 {
            Coord::new(0, 0)
        } else {
            // something is wrong! (known SST2K quirk, kept: a Tholian
            // removed by move(None) leaves game.tholian set, with no location)
            self.enemy_move(t, None);
            self.prout("***Internal error: Tholian in a bad spot.");
            return;
        };
        // do nothing if we are blocked
        let c = self.game.q(tid);
        if c != '.' && c != '#' {
            return;
        }
        let mut here = loc;
        let delta = tid.sub(loc).sgn();
        // move in x axis
        while here.i != tid.i {
            here.i += delta.i;
            if self.game.q(here) == '.' {
                self.enemy_move(t, Some(here));
            }
        }
        // move in y axis
        while here.j != tid.j {
            here.j += delta.j;
            if self.game.q(here) == '.' {
                self.enemy_move(t, Some(here));
            }
        }
        // check to see if all holes plugged
        let q = |s: &Self, i: usize, j: usize| s.game.quad[i][j];
        for i in 0..QS {
            if q(self, 0, i) != '#' && q(self, 0, i) != 'T' {
                return;
            }
            if q(self, QS - 1, i) != '#' && q(self, QS - 1, i) != 'T' {
                return;
            }
            if q(self, i, 0) != '#' && q(self, i, 0) != 'T' {
                return;
            }
            if q(self, i, QS - 1) != '#' && q(self, i, QS - 1) != 'T' {
                return;
            }
        }
        // All plugged up -- Tholian splits
        let tl = self.game.pool[t].location;
        self.game.set_q(tl, '#');
        self.dropin(Some(' '));
        let s = crmena(true, 'T', "sector", &self.enemy_repr(t)) + " completes web.";
        self.prout(&s);
        self.enemy_move(t, None);
    }
}
