//! Movement: imove, dock, cartesian, getcourse, course, impulse, warp, setwarp (`sst` 4320-4940).

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, TrekError};
use crate::state::*;

impl Sst {
    /// `cartesian()`: the ship's absolute position.
    pub fn cartesian(&self) -> Coord {
        self.game.quadrant.mul(QUADSIZE).add(self.game.sector)
    }

    /// `course(bearing, distance)` with the default origin (the ship).
    pub fn course(&self, bearing: f64, distance: f64) -> Course {
        Course::new(bearing, distance, self.cartesian().to_vec2())
    }

    /// `imove(icourse, noattack)`: movement execution for warp, impulse,
    /// supernova, and tractor-beam events. The nested `newquadrant` and
    /// `check_collision` are the private helpers below; the locals they
    /// share (`w`, `trbeam`) are passed in.
    pub fn imove(&mut self, icourse: &mut Course, noattack: bool) {
        let mut w = Coord::none();
        let mut trbeam = false;
        if self.game.inorbit {
            self.prout("Helmsman Sulu- \"Leaving standard orbit.\"");
            self.game.inorbit = false;
        }
        // If tractor beam is to occur, don't move full distance
        if self.game.state.date + self.game.optime >= self.scheduled(FTBEAM) {
            if self.game.iscloaked {
                // We can't be tractor beamed if cloaked,
                // so move the event into the future
                if !self.game.state.kcmdr.is_empty() {
                    let n = self.game.state.kcmdr.len() as f64;
                    let e = self.expran(1.5 * self.game.intime / n);
                    self.postpone(FTBEAM, self.game.optime + e);
                } else {
                    self.unschedule(FTBEAM);
                }
            } else {
                trbeam = true;
                self.game.condition = "red".into();
                icourse.distance =
                    icourse.distance * (self.scheduled(FTBEAM) - self.game.state.date) / self.game.optime + 0.1;
                self.game.optime = self.scheduled(FTBEAM) - self.game.state.date + 1e-5;
            }
        }
        // Move out
        let s = self.game.sector;
        self.game.set_q(s, '.');
        for _m in 0..icourse.moves.max(0) {
            icourse.nextstep();
            w = icourse.sector();
            if icourse.origin.quadrant() != icourse.location.quadrant() {
                self.imove_newquadrant(icourse, w, trbeam, noattack);
                break;
            } else if self.imove_check_collision(icourse, w) {
                self.prout("Collision detected");
                break;
            } else {
                self.game.sector = w;
            }
        }
        // We're in destination quadrant -- compute new average enemy distances
        let s = self.game.sector;
        let ship = self.game.ship;
        self.game.set_q(s, ship);
        if !self.game.enemies.is_empty() {
            // DEVIATION: with a course of zero moves (a tiny distance such as
            // "move m 0.01") `w` is still Coord() here and Python crashes with
            // a TypeError on `w - enemy.location`; the ship has not moved, so
            // its own sector stands in for `w`.
            let wd = if w.is_none() { self.game.sector } else { w };
            for k in 0..self.game.enemies.len() {
                let id = self.game.enemies[k];
                let finald = wd.sub(self.game.pool[id].location).distance();
                let e = &mut self.game.pool[id];
                e.kavgd = 0.5 * (finald + e.kdist);
                e.kdist = finald;
            }
            self.sortenemies();
            if !self.game.here().supernova {
                self.attack(false);
            }
            for k in 0..self.game.enemies.len() {
                let id = self.game.enemies[k];
                let e = &mut self.game.pool[id];
                e.kavgd = e.kdist;
            }
        }
        self.newcnd();
        self.drawmaps(0);
    }

    /// imove's nested `newquadrant(noattack)`.
    fn imove_newquadrant(&mut self, icourse: &mut Course, w: Coord, trbeam: bool, noattack: bool) {
        // Leaving quadrant -- allow final enemy attack
        // Don't set up attack if being pushed by nova or cloaked
        if !self.game.enemies.is_empty() && !noattack && !self.game.iscloaked {
            self.newcnd();
            for k in 0..self.game.enemies.len() {
                let id = self.game.enemies[k];
                let finald = w.sub(self.game.pool[id].location).distance();
                let e = &mut self.game.pool[id];
                e.kavgd = 0.5 * (finald + e.kdist);
            }
            // Stas Sergeev added the condition
            // that attacks only happen if Klingons
            // are present and your skill is good.
            if self.game.skill > SKILL_GOOD && self.game.klhere > 0 && !self.game.here().supernova {
                self.attack(false);
            }
            if self.game.alldone {
                return;
            }
        }
        // check for edge of galaxy
        let mut kinks = 0;
        loop {
            let mut kink = false;
            if icourse.final_.i < 0 {
                icourse.final_.i = -icourse.final_.i;
                kink = true;
            }
            if icourse.final_.j < 0 {
                icourse.final_.j = -icourse.final_.j;
                kink = true;
            }
            if icourse.final_.i >= GALSIZE * QUADSIZE {
                icourse.final_.i = (GALSIZE * QUADSIZE * 2) - icourse.final_.i;
                // DEVIATION: a course ending exactly on the far edge (80)
                // reflects onto itself and SST2K loops forever here; step
                // it one sector inside instead.
                if icourse.final_.i == GALSIZE * QUADSIZE {
                    icourse.final_.i = GALSIZE * QUADSIZE - 1;
                }
                kink = true;
            }
            if icourse.final_.j >= GALSIZE * QUADSIZE {
                icourse.final_.j = (GALSIZE * QUADSIZE * 2) - icourse.final_.j;
                // DEVIATION: a course ending exactly on the far edge (80)
                // reflects onto itself and SST2K loops forever here; step
                // it one sector inside instead.
                if icourse.final_.j == GALSIZE * QUADSIZE {
                    icourse.final_.j = GALSIZE * QUADSIZE - 1;
                }
                kink = true;
            }
            if kink {
                kinks += 1;
            } else {
                break;
            }
        }
        if kinks != 0 {
            self.game.nkinks += 1;
            if self.game.nkinks == 3 {
                // Three strikes -- you're out!
                self.finish(FNEG3);
                return;
            }
            self.skip(1);
            self.prout("YOU HAVE ATTEMPTED TO CROSS THE NEGATIVE ENERGY BARRIER");
            self.prout("AT THE EDGE OF THE GALAXY.  THE THIRD TIME YOU TRY THIS,");
            self.prout("YOU WILL BE DESTROYED.");
        }
        // Compute final position in new quadrant
        if trbeam {
            // Don't bother if we are to be beamed
            return;
        }
        self.game.quadrant = icourse.final_.quadrant();
        self.game.sector = icourse.final_.sector();
        self.skip(1);
        let s = pf!("Entering Quadrant %s.", self.game.quadrant);
        self.prout(&s);
        let sec = self.game.sector;
        let ship = self.game.ship;
        self.game.set_q(sec, ship);
        self.newqad();
        if self.game.skill > SKILL_NOVICE {
            self.attack(false);
        }
    }

    /// imove's nested `check_collision(h)`.
    fn imove_check_collision(&mut self, icourse: &Course, h: Coord) -> bool {
        let iquad = self.game.q(h);
        if iquad != '.' {
            // object encountered in flight path
            let stopegy = 50.0 * icourse.distance / self.game.optime;
            if "TKCSR?".contains(iquad) {
                for k in 0..self.game.enemies.len() {
                    let id = self.game.enemies[k];
                    if self.game.pool[id].location == h {
                        self.collision(false, id);
                        return true;
                    }
                }
                // This should not happen
                self.prout("Which way did he go?");
                false
            } else if iquad == ' ' {
                self.skip(1);
                self.prouts("***RED ALERT!  RED ALERT!");
                self.skip(1);
                let s = "***".to_string() + &self.crmshp();
                self.proutn(&s);
                let s = pf!(" pulled into black hole at Sector %s", h);
                self.proutn(&s);
                // Getting pulled into a black hole was certain
                // death in Almy's original.  Stas Sergeev added a
                // possibility that you'll get timewarped instead.
                let mut n: i64 = 0;
                for m in 0..NDEVICES {
                    if self.game.damage[m] > 0.0 {
                        n += 1;
                    }
                }
                let probf = pypow(1.4, (self.game.energy + self.game.shield) / 5000.0 - 1.0)
                    * pypow(1.3, 1.0 / (n + 1) as f64 - 1.0);
                if (self.game.options & OPTION_BLKHOLE) != 0 && self.withprob(1.0 - probf) {
                    self.timwrp();
                } else {
                    self.finish(FHOLE);
                }
                true
            } else {
                // something else
                self.skip(1);
                let s = self.crmshp();
                self.proutn(&s);
                if iquad == '#' {
                    let s = pf!(" encounters Tholian web at %s;", h);
                    self.prout(&s);
                } else {
                    let s = pf!(" blocked by object at %s;", h);
                    self.prout(&s);
                }
                self.proutn("Emergency stop required ");
                let s = pf!("%2d units of energy.", trunc(stopegy));
                self.prout(&s);
                self.game.energy -= stopegy;
                if self.game.energy <= 0.0 {
                    self.finish(FNRG);
                }
                true
            }
        } else {
            false
        }
    }

    /// `dock(verbose)`: dock our ship at a starbase.
    pub fn dock(&mut self, verbose: bool) {
        self.scanner.chew();
        if self.game.condition == "docked" && verbose {
            self.prout("Already docked.");
            return;
        }
        if self.game.inorbit {
            self.prout("You must first leave standard orbit.");
            return;
        }
        if !self.game.base.valid_sector() {
            self.prout("No starbase available for docking in this quadrant.");
            return;
        }
        if (self.game.sector.i - self.game.base.i).abs() > 1 || (self.game.sector.j - self.game.base.j).abs() > 1 {
            let s = self.crmshp() + " not adjacent to base.";
            self.prout(&s);
            return;
        }
        if self.game.iscloaked {
            self.prout("You cannot dock while cloaked.");
            return;
        }
        self.game.condition = "docked".into();
        if verbose {
            self.prout("Docked.");
        }
        self.game.ididit = true;
        // max() keeps the first of equals
        if self.game.inenrg > self.game.energy {
            self.game.energy = self.game.inenrg;
        }
        self.game.shield = self.game.inshld;
        self.game.torps = self.game.intorps;
        self.game.lsupres = self.game.inlsr;
        self.game.state.crew = FULLCREW;
        if self.game.brigcapacity - self.game.brigfree > 0 {
            let s = pf!("%d captured Klingons transferred to base", self.game.brigcapacity - self.game.brigfree);
            self.prout(&s);
            self.game.kcaptured += self.game.brigcapacity - self.game.brigfree;
            self.game.brigfree = self.game.brigcapacity;
        }
        if self.communicating() && ((self.is_scheduled(FCDBAS) || self.game.isatb == 1) && !self.game.iseenit) {
            // get attack report from base
            self.prout("Lt. Uhura- \"Captain, an important message from the starbase:\"");
            self.attackreport(false);
            self.game.iseenit = true;
        }
    }

    /// `getcourse(isprobe)`: get a course and distance from the user.
    #[allow(unused_assignments)] // iprompt is set in the manual branch as in Python
    pub fn getcourse(&mut self, isprobe: bool) -> Result<Course, TrekError> {
        let mut key = Tok::None;
        let mut dquad = self.game.quadrant;
        let mut navmode = "unspecified";
        let mut itemp = "curt";
        // For a probe given only a quadrant, Python sets both parts to the
        // float 4.0 (QUADSIZE/2 - 1); it is never printed, and as an f64
        // in the delta arithmetic 4 is the same value.
        let mut dsect = Coord::none();
        let mut iprompt = false;
        if self.game.landed && !isprobe {
            self.prout("Dummy! You can't leave standard orbit until you");
            self.proutn("are back aboard the ship.");
            self.scanner.chew();
            return Err(TrekError);
        }
        while navmode == "unspecified" {
            if self.damaged(DNAVSYS) {
                if isprobe {
                    self.prout("Computer damaged; manual navigation only");
                } else {
                    self.prout("Computer damaged; manual movement only");
                }
                self.scanner.chew();
                navmode = "manual";
                key = Tok::Eol;
                break;
            }
            key = self.nexttok();
            if key == Tok::Eol {
                self.proutn("Manual or automatic- ");
                iprompt = true;
                self.scanner.chew();
            } else if key == Tok::Alpha {
                if self.scanner.sees("manual") {
                    navmode = "manual";
                    key = self.nexttok();
                    break;
                } else if self.scanner.sees("automatic") {
                    navmode = "automatic";
                    key = self.nexttok();
                    break;
                } else {
                    self.huh();
                    self.scanner.chew();
                    return Err(TrekError);
                }
            } else {
                // numeric
                if isprobe {
                    self.prout("(Manual navigation assumed.)");
                } else {
                    self.prout("(Manual movement assumed.)");
                }
                navmode = "manual";
                break;
            }
        }
        let mut delta = Vec2::new(0.0, 0.0);
        if navmode == "automatic" {
            while key == Tok::Eol {
                if isprobe {
                    self.proutn("Target quadrant or quadrant&sector- ");
                } else {
                    self.proutn("Destination sector or quadrant&sector- ");
                }
                self.scanner.chew();
                iprompt = true;
                key = self.nexttok();
            }
            let t = self.scanner.token.clone();
            self.scanner.push(&t); // Something IHREAL or IHALPHA awaits us
            let first = match self.getcoord() {
                Some(c) => c,
                None => return Err(TrekError),
            };
            self.nexttok();
            let second = if self.scanner.type_ == Tok::Eol {
                None
            } else {
                let t = self.scanner.token.clone();
                self.scanner.push(&t);
                match self.getcoord() {
                    Some(c) => Some(c),
                    None => return Err(TrekError),
                }
            };
            self.scanner.chew();
            if let Some(second) = second {
                dquad.i = first.i;
                dquad.j = first.j;
                dsect.i = second.i;
                dsect.j = second.j;
            } else {
                // only one pair of numbers was specified
                if isprobe {
                    // only quadrant specified -- go to center of dest quad
                    dquad.i = first.i;
                    dquad.j = first.j;
                    dsect.i = QUADSIZE / 2 - 1; // preserves 1-origin behavior
                    dsect.j = dsect.i;
                } else {
                    // only sector specified
                    dsect.i = first.i;
                    dsect.j = first.j;
                }
                itemp = "normal";
            }
            if !dquad.valid_quadrant() || !dsect.valid_sector() {
                self.huh();
                return Err(TrekError);
            }
            self.skip(1);
            if !isprobe {
                if itemp > "curt" {
                    if iprompt {
                        let s = pf!("Helmsman Sulu- \"Course locked in for Sector %s.\"", dsect);
                        self.prout(&s);
                    }
                } else {
                    self.prout("Ensign Chekov- \"Course laid in, Captain.\"");
                }
            }
            // the actual deltas get computed here
            delta.j = (dquad.j - self.game.quadrant.j) as f64
                + (dsect.j - self.game.sector.j) as f64 / (QUADSIZE as f64 * 1.0);
            delta.i = (self.game.quadrant.i - dquad.i) as f64
                + (self.game.sector.i - dsect.i) as f64 / (QUADSIZE as f64 * 1.0);
        } else {
            // manual
            while key == Tok::Eol {
                self.proutn("X and Y displacements- ");
                self.scanner.chew();
                iprompt = true;
                key = self.nexttok();
            }
            itemp = "verbose";
            if key == Tok::Real {
                delta.j = self.scanner.real;
            } else {
                self.huh();
                return Err(TrekError);
            }
            key = self.nexttok();
            if key == Tok::Real {
                delta.i = self.scanner.real;
            } else if key == Tok::Eol {
                delta.i = 0.0;
                self.scanner.push("\n");
            } else {
                self.huh();
                return Err(TrekError);
            }
        }
        // Check for zero movement
        if delta.i == 0.0 && delta.j == 0.0 {
            self.scanner.chew();
            return Err(TrekError);
        }
        if itemp == "verbose" && !isprobe {
            self.skip(1);
            self.prout("Helmsman Sulu- \"Aye, Sir.\"");
        }
        self.scanner.chew();
        Ok(self.course(delta.bearing(), delta.distance()))
    }

    /// `impulse()`: move under impulse power.
    pub fn impulse(&mut self) {
        self.game.ididit = false;
        if self.damaged(DIMPULS) {
            self.scanner.chew();
            self.skip(1);
            self.prout("Engineer Scott- \"The impulse engines are damaged, Sir.\"");
            return;
        }
        let mut icourse = None;
        let power;
        if self.game.energy > 30.0 {
            match self.getcourse(false) {
                Ok(c) => {
                    power = 20.0 + 100.0 * c.distance;
                    icourse = Some(c);
                }
                Err(TrekError) => return,
            }
        } else {
            power = 30.0;
        }
        if power >= self.game.energy {
            // Insufficient power for trip
            self.skip(1);
            self.prout("First Officer Spock- \"Captain, the impulse engines");
            self.prout("require 20.0 units to engage, plus 100.0 units per");
            if self.game.energy > 30.0 {
                let s = pf!(
                    "quadrant.  We can go, therefore, a maximum of %d",
                    trunc(0.01 * (self.game.energy - 20.0) - 0.05)
                );
                self.proutn(&s);
                self.prout(" quadrants.\"");
            } else {
                self.prout("quadrant.  They are, therefore, useless.\"");
            }
            self.scanner.chew();
            return;
        }
        // power < energy implies energy > 30, so a course was read
        let mut icourse = icourse.expect("impulse: course read");
        // Make sure enough time is left for the trip
        self.game.optime = icourse.distance / 0.095;
        if !self.game.sandbox && self.game.optime >= self.game.state.remtime {
            self.prout("First Officer Spock- \"Captain, our speed under impulse");
            self.prout("power is only 0.95 sectors per stardate. Are you sure");
            self.proutn("we dare spend the time?\" ");
            if !self.ja() {
                return;
            }
        }
        // Activate impulse engines and pay the cost
        self.imove(&mut icourse, false);
        self.game.ididit = true;
        if self.game.alldone {
            return;
        }
        let power = 20.0 + 100.0 * icourse.distance;
        self.game.energy -= power;
        self.game.optime = icourse.distance / 0.095;
        if self.game.energy <= 0.0 {
            self.finish(FNRG);
        }
    }

    /// `warp(wcourse, involuntary)`: move under warp drive.
    pub fn warp(&mut self, wcourse: Option<Course>, involuntary: bool) {
        let mut wcourse = wcourse;
        let mut blooey = false;
        let mut twarp = false;
        if !involuntary {
            // Not WARPX entry
            self.game.ididit = false;
            if self.game.iscloaked {
                self.scanner.chew();
                self.skip(1);
                self.prout("Engineer Scott- \"The warp engines can not be used while cloaked, Sir.\"");
                return;
            }
            if self.game.damage[DWARPEN] > 10.0 {
                self.scanner.chew();
                self.skip(1);
                self.prout("Engineer Scott- \"The warp engines are damaged, Sir.\"");
                return;
            }
            if self.damaged(DWARPEN) && self.game.warpfac > 4.0 {
                self.scanner.chew();
                self.skip(1);
                self.prout("Engineer Scott- \"Sorry, Captain. Until this damage");
                self.prout("  is repaired, I can only give you warp 4.\"");
                return;
            }
            // Read in course and distance
            if wcourse.is_none() {
                match self.getcourse(false) {
                    Ok(c) => wcourse = Some(c),
                    Err(TrekError) => return,
                }
            }
            let wc = wcourse.as_ref().unwrap();
            // Make sure starship has enough energy for the trip
            // Note: this formula is slightly different from the C version,
            // and lets you skate a bit closer to the edge.
            if wc.power(self.game.warpfac, self.game.shldup) >= self.game.energy {
                // Insufficient power for trip
                self.game.ididit = false;
                self.skip(1);
                self.prout("Engineering to bridge--");
                if !self.game.shldup || 0.5 * wc.power(self.game.warpfac, self.game.shldup) > self.game.energy {
                    let iwarp = pypow(self.game.energy / (wc.distance + 0.05), 0.333333333);
                    if iwarp <= 0.0 {
                        self.prout("We can't do it, Captain. We don't have enough energy.");
                    } else {
                        let s = pf!("We don't have enough energy, but we could do it at warp %d", iwarp);
                        self.proutn(&s);
                        if self.game.shldup {
                            self.prout(",");
                            self.prout("if you'll lower the shields.");
                        } else {
                            self.prout(".");
                        }
                    }
                } else {
                    self.prout("We haven't the energy to go that far with the shields up.");
                }
                return;
            }
            // Make sure enough time is left for the trip
            self.game.optime = wc.time(self.game.warpfac);
            if !self.game.sandbox && self.game.optime >= 0.8 * self.game.state.remtime {
                self.skip(1);
                self.prout("First Officer Spock- \"Captain, I compute that such");
                let s = pf!(
                    "  a trip would require approximately %2.0f",
                    100.0 * self.game.optime / self.game.state.remtime
                );
                self.proutn(&s);
                self.prout(" percent of our");
                self.proutn("  remaining time.  Are you sure this is wise?\" ");
                if !self.ja() {
                    self.game.ididit = false;
                    // Python sets the int 0 here
                    self.game.optime = 0.0;
                    return;
                }
            }
        }
        let mut wcourse = wcourse.expect("warp: no course");
        // Entry WARPX
        if self.game.warpfac > 6.0 {
            // Decide if engine damage will occur
            // ESR: Seems wrong. Probability of damage goes *down* with distance?
            let prob = wcourse.distance * pypow(6.0 - self.game.warpfac, 2.0) / 66.666666666;
            if prob > self.rnd_real() {
                blooey = true;
                wcourse.distance = self.rnd_real1(wcourse.distance);
            }
            // Decide if time warp will occur
            if 0.5 * wcourse.distance * pypow(7.0, self.game.warpfac - 10.0) > self.rnd_real() {
                twarp = true;
            }
            if self.game.idebug && self.game.warpfac == 10.0 && !twarp {
                blooey = false;
                self.proutn("=== Force time warp? ");
                if self.ja() {
                    twarp = true;
                }
            }
            if blooey || twarp {
                // If time warp or engine damage, check path
                // If it is obstructed, don't do warp or damage
                let mut look = wcourse.moves;
                while look > 0 {
                    look -= 1;
                    wcourse.nextstep();
                    let w = wcourse.sector();
                    if !w.valid_sector() {
                        break;
                    }
                    if self.game.q(w) != '.' {
                        blooey = false;
                        twarp = false;
                    }
                }
                wcourse.reset();
            }
        }
        // Activate Warp Engines and pay the cost
        self.imove(&mut wcourse, false);
        if self.game.alldone {
            return;
        }
        self.game.energy -= wcourse.power(self.game.warpfac, self.game.shldup);
        if self.game.energy <= 0.0 {
            self.finish(FNRG);
        }
        self.game.optime = wcourse.time(self.game.warpfac);
        if twarp {
            self.timwrp();
        }
        if blooey {
            self.game.damage[DWARPEN] = self.game.damfac * self.rnd_real2(1.0, 4.0);
            self.skip(1);
            self.prout("Engineering to bridge--");
            self.prout("  Scott here.  The warp engines are damaged.");
            self.prout("  We'll have to reduce speed to warp 4.");
        }
        self.game.ididit = true;
    }

    /// `setwarp()`: change the warp factor.
    pub fn setwarp(&mut self) {
        let mut key;
        loop {
            key = self.nexttok();
            if key != Tok::Eol {
                break;
            }
            self.scanner.chew();
            self.proutn("Warp factor- ");
        }
        if key != Tok::Real {
            self.huh();
            return;
        }
        if self.game.damage[DWARPEN] > 10.0 {
            self.prout("Warp engines inoperative.");
            return;
        }
        if self.damaged(DWARPEN) && self.scanner.real > 4.0 {
            self.prout("Engineer Scott- \"I'm doing my best, Captain,");
            self.prout("  but right now we can only go warp 4.\"");
            return;
        }
        if self.scanner.real > 10.0 {
            self.prout("Helmsman Sulu- \"Our top speed is warp 10, Captain.\"");
            return;
        }
        if self.scanner.real < 1.0 {
            self.prout("Helmsman Sulu- \"We can't go below warp 1, Captain.\"");
            return;
        }
        let oldfac = self.game.warpfac;
        self.game.warpfac = self.scanner.real;
        if self.game.warpfac <= oldfac || self.game.warpfac <= 6.0 {
            let s = pf!("Helmsman Sulu- \"Warp factor %d, Captain.\"", trunc(self.game.warpfac));
            self.prout(&s);
            return;
        }
        if self.game.warpfac < 8.00 {
            self.prout("Engineer Scott- \"Aye, but our maximum safe speed is warp 6.\"");
            return;
        }
        if self.game.warpfac == 10.0 {
            self.prout("Engineer Scott- \"Aye, Captain, we'll try it.\"");
            return;
        }
        self.prout("Engineer Scott- \"Aye, Captain, but our engines may not take it.\"");
    }
}
