//! Enemy-side battle: collision, torpedo, fry, attack, deadkl (`sst` 1451-1986).

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::py::*;
use crate::sst::{Sst, SystemExit, cramen, crmena};
use crate::state::*;

impl Sst {
    /// Recompute every enemy's distance to the ship (both `kdist` and
    /// `kavgd`), as the torpedo displacement code does.
    fn redistance_enemies(&mut self) {
        for k in 0..self.game.enemies.len() {
            let e = self.game.enemies[k];
            let d = self.game.sector.sub(self.game.pool[e].location).distance();
            self.game.pool[e].kdist = d;
            self.game.pool[e].kavgd = d;
        }
    }

    /// `collision(rammed, enemy)`: collision handling for ramming events.
    pub fn collision(&mut self, rammed: bool, enemy: EnemyId) {
        self.prouts("***RED ALERT!  RED ALERT!");
        self.skip(1);
        self.prout("***COLLISION IMMINENT.");
        self.skip(2);
        self.proutn("***");
        let s = self.crmshp();
        self.proutn(&s);
        let etype = self.game.pool[enemy].etype;
        let eloc = self.game.pool[enemy].location;
        let hardness = match etype {
            'R' => 1.5,
            'C' => 2.0,
            'S' => 2.5,
            'T' => 0.5,
            '?' => 4.0,
            _ => 1.0,
        };
        if rammed {
            self.proutn(" rammed by ");
        } else {
            self.proutn(" rams ");
        }
        self.proutn(&crmena(false, etype, "sector", &eloc.to_string()));
        if rammed {
            self.proutn(" (original position)");
        }
        self.skip(1);
        let sector = self.game.sector;
        self.deadkl(eloc, etype, sector);
        let s = "***".to_string() + &self.crmshp() + " heavily damaged.";
        self.prout(&s);
        let icas = self.rnd_integer2(10.0, 30.0);
        let s = pf!("***Sickbay reports %d casualties", icas);
        self.prout(&s);
        self.game.casual += icas;
        self.game.state.crew -= icas;
        // In the pre-SST2K versions, all devices got equiprobably damaged,
        // which was silly.  Instead, pick up to half the devices at
        // random according to our weighting table,
        let mut ncrits = self.rnd_integer((NDEVICES / 2) as f64);
        while ncrits > 0 {
            ncrits -= 1;
            let dev = self.randdevice();
            if self.game.damage[dev] < 0.0 {
                continue;
            }
            let r = self.rnd_real();
            let extradm = (10.0 * hardness * r + 1.0) * self.game.damfac;
            // Damage for at least time of travel!
            self.game.damage[dev] += self.game.optime + extradm;
        }
        self.game.shldup = false;
        self.prout("***Shields are down.");
        if self.game.unwon() != 0 {
            self.announce();
            self.damagereport();
        } else {
            self.finish(FWON);
        }
    }

    /// `torpedo(origin, bearing, dispersion, number, nburst)`: let a photon
    /// torpedo fly. Returns the hit on our ship, if any.
    #[allow(clippy::approx_constant)] // SST2K's 0.5235988, not FRAC_PI_6
    pub fn torpedo(&mut self, origin: Coord, bearing: f64, dispersion: f64, number: i64, nburst: i64) -> Option<f64> {
        let ac = bearing + 0.25 * dispersion; // dispersion is a random variable
        let bullseye = (15.0 - bearing) * 0.5235988;
        // cartesian(origin): game.quadrant * QUADSIZE + origin
        let start = self.game.quadrant.mul(QUADSIZE).add(origin).to_vec2();
        let mut track = Course::new(ac, QUADSIZE as f64, start);
        // Loop to move a single torpedo
        for step in 1..QUADSIZE * 2 {
            if !track.nextstep() {
                break;
            }
            let w = track.sector();
            if !w.valid_sector() {
                break;
            }
            let iquad = self.game.q(w);
            self.tracktorpedo(w, step, number, nburst, iquad);
            if iquad == '.' {
                continue;
            }
            // hit something
            if !self.damaged(DSRSENS) || self.game.condition == "docked" {
                self.skip(1); // start new line after text track
            }
            match iquad {
                'E' | 'F' => {
                    // Hit our ship
                    self.skip(1);
                    let s = pf!("Torpedo hits %s.", self.crmshp());
                    self.prout(&s);
                    let r = self.rnd_real1(100.0);
                    let hit = 700.0 + r - 1000.0 * w.sub(origin).distance() * (bullseye - track.angle).sin().abs();
                    self.newcnd(); // we're blown out of dock
                    if self.game.landed || self.game.condition == "docked" {
                        return Some(hit); // Cheat if on a planet
                    }
                    // In the C/FORTRAN version, dispersion was 2.5 radians, which
                    // is 143 degrees, which is almost exactly 4.8 clockface units
                    let r = self.rnd_real2(-2.4, 2.4);
                    let mut displacement = self.course(track.bearing + r, pypow(2.0, 0.5));
                    displacement.nextstep();
                    let bumpto = displacement.sector();
                    if !bumpto.valid_sector() {
                        return Some(hit);
                    }
                    if self.game.q(bumpto) == ' ' {
                        self.finish(FHOLE);
                        return Some(hit);
                    }
                    if self.game.q(bumpto) != '.' {
                        // can't move into object
                        return Some(hit);
                    }
                    self.game.sector = bumpto;
                    let s = self.crmshp();
                    self.proutn(&s);
                    self.game.set_q(w, '.');
                    self.game.set_q(bumpto, iquad);
                    let s = pf!(" displaced by blast to Sector %s ", bumpto);
                    self.prout(&s);
                    self.redistance_enemies();
                    self.sortenemies();
                    return None;
                }
                'C' | 'S' | 'R' | 'K' => {
                    // Hit a regular enemy
                    // find the enemy
                    if (iquad == 'C' || iquad == 'S') && self.withprob(0.05) {
                        let s = crmena(true, iquad, "sector", &w.to_string()) + " uses anti-photon device;";
                        self.prout(&s);
                        self.prout("   torpedo neutralized.");
                        return None;
                    }
                    // The Python loop body runs only for the first enemy at w
                    // and always ends the loop, so finding it first is the same.
                    let found = self.game.enemies.iter().copied().find(|&e| w == self.game.pool[e].location);
                    let Some(enemy) = found else {
                        self.prout("Internal error, no enemy where expected!");
                        std::panic::resume_unwind(Box::new(SystemExit(1)));
                    };
                    let kp = self.game.pool[enemy].power.abs();
                    let r = self.rnd_integer(100.0);
                    let mut h1 =
                        700.0 + r as f64 - 1000.0 * w.sub(origin).distance() * (bullseye - track.angle).sin().abs();
                    h1 = h1.abs();
                    // min(h1, kp)
                    if kp < h1 {
                        h1 = kp;
                    }
                    if self.game.pool[enemy].power < 0.0 {
                        self.game.pool[enemy].power -= -h1;
                    } else {
                        self.game.pool[enemy].power -= h1;
                    }
                    if self.game.pool[enemy].power == 0.0 {
                        self.deadkl(w, iquad, w);
                        return None;
                    }
                    self.proutn(&crmena(true, iquad, "sector", &w.to_string()));
                    let r = self.rnd_real2(-2.4, 2.4);
                    let mut displacement = Course::new(track.bearing + r, pypow(2.0, 0.5), w.to_vec2());
                    displacement.nextstep();
                    let bumpto = displacement.sector();
                    if self.game.q(bumpto) == ' ' {
                        self.prout(" buffeted into black hole.");
                        self.deadkl(w, iquad, bumpto);
                        return None;
                    }
                    if !bumpto.valid_sector() {
                        self.prout(" damaged but not destroyed.");
                        return None;
                    }
                    if self.game.q(bumpto) != '.' {
                        self.prout(" damaged but not destroyed.");
                    } else {
                        let s = pf!(" damaged-- displaced by blast to Sector %s ", bumpto);
                        self.prout(&s);
                        self.game.pool[enemy].location = bumpto;
                        self.game.set_q(w, '.');
                        self.game.set_q(bumpto, iquad);
                        self.redistance_enemies();
                        self.sortenemies();
                    }
                    return None;
                }
                'B' => {
                    // Hit a base
                    self.skip(1);
                    self.prout("***STARBASE DESTROYED..");
                    let q = self.game.quadrant;
                    self.game.state.baseq.retain(|x| *x != q);
                    self.game.set_q(w, '.');
                    self.game.base.invalidate();
                    self.game.state.gal_mut(q).starbase = false;
                    self.game.state.page_mut(q).starbase = false;
                    self.game.state.basekl += 1;
                    self.newcnd();
                    return None;
                }
                'P' | '@' => {
                    // Hit a planet, or an inhabited world -- very bad!
                    let s = crmena(true, iquad, "sector", &w.to_string()) + " destroyed.";
                    self.prout(&s);
                    if iquad == 'P' {
                        self.game.state.nplankl += 1;
                    } else {
                        self.game.state.nworldkl += 1;
                    }
                    self.game.here_mut().planet = None;
                    // DEVIATION: Python raises AttributeError if game.iplnet is None here.
                    if let Some(p) = self.game.iplnet_mut() {
                        p.pclass = Some("destroyed".into());
                    }
                    self.game.iplnet = None;
                    self.game.plnet.invalidate();
                    self.game.set_q(w, '.');
                    if self.game.landed {
                        // captain perishes on planet
                        self.finish(FDPLANET);
                    }
                    if iquad == '@' {
                        self.prout("The torpedo destroyed an inhabited planet.");
                    }
                    return None;
                }
                '*' => {
                    // Hit a star
                    if self.withprob(0.9) {
                        self.nova(w);
                    } else {
                        let s = crmena(true, '*', "sector", &w.to_string()) + " unaffected by photon blast.";
                        self.prout(&s);
                    }
                    return None;
                }
                '?' => {
                    // Hit a thingy
                    self.skip(1);
                    self.prouts("AAAAIIIIEEEEEEEEAAAAAAAAUUUUUGGGGGHHHHHHHHHHHH!!!");
                    self.skip(1);
                    self.prouts("    HACK!     HACK!    HACK!        *CHOKE!*  ");
                    self.skip(1);
                    self.proutn("Mr. Spock-");
                    self.prouts("  \"Fascinating!\"");
                    self.skip(1);
                    self.deadkl(w, iquad, w);
                    return None;
                }
                ' ' => {
                    // Black hole
                    self.skip(1);
                    let s = crmena(true, ' ', "sector", &w.to_string()) + " swallows torpedo.";
                    self.prout(&s);
                    return None;
                }
                '#' => {
                    // hit the web
                    self.skip(1);
                    self.prout("***Torpedo absorbed by Tholian web.");
                    return None;
                }
                'T' => {
                    // Hit a Tholian
                    let r = self.rnd_integer(100.0);
                    let mut h1 =
                        700.0 + r as f64 - 1000.0 * w.sub(origin).distance() * (bullseye - track.angle).sin().abs();
                    h1 = h1.abs();
                    if h1 >= 600.0 {
                        self.game.set_q(w, '.');
                        self.deadkl(w, iquad, w);
                        self.game.tholian = None;
                        return None;
                    }
                    self.skip(1);
                    self.proutn(&crmena(true, 'T', "sector", &w.to_string()));
                    if self.withprob(0.05) {
                        self.prout(" survives photon blast.");
                        return None;
                    }
                    self.prout(" disappears.");
                    // DEVIATION: Python raises AttributeError if game.tholian is None here.
                    if let Some(t) = self.game.tholian {
                        self.enemy_move(t, None);
                    }
                    self.game.set_q(w, '#');
                    self.dropin(Some(' '));
                    return None;
                }
                _ => {
                    // Problem!
                    self.skip(1);
                    self.proutn("Don't know how to handle torpedo collision with ");
                    self.proutn(&crmena(true, iquad, "sector", &w.to_string()));
                    self.skip(1);
                    return None;
                }
            }
        }
        self.skip(1);
        self.prout("Torpedo missed.");
        None
    }

    /// `fry(hit)`: critical-hit resolution.
    pub fn fry(&mut self, hit: f64) {
        let r = self.rnd_real2(1.0, 1.5);
        if hit < (275.0 - 25.0 * self.game.skill as f64) * r {
            return;
        }
        let r = self.rnd_real1(100.0);
        let mut ncrit = trunc(1.0 + hit / (500.0 + r));
        self.proutn("***CRITICAL HIT--");
        // Select devices and cause damage
        let mut cdam: Vec<usize> = Vec::new();
        while ncrit > 0 {
            let j = loop {
                let j = self.randdevice();
                // Cheat to prevent shuttle damage unless on ship
                if !(self.game.damage[j] < 0.0
                    || (j == DSHUTTL && self.game.iscraft != "onship")
                    || ((j == DCLOAK && self.game.ship != 'E') || j == DDRAY))
                {
                    break j;
                }
            };
            cdam.push(j);
            let r = self.rnd_real2(75.0, 100.0);
            let extradm = (hit * self.game.damfac) / (ncrit as f64 * r);
            self.game.damage[j] += extradm;
            ncrit -= 1;
        }
        let mut skipcount = 0;
        let n = cdam.len();
        for (i, &j) in cdam.iter().enumerate() {
            self.proutn(DEVICE[j]);
            if skipcount % 3 == 2 && i + 1 < n {
                self.skip(1);
            }
            skipcount += 1;
            if i + 1 < n {
                self.proutn(" and ");
            }
        }
        self.prout(" damaged.");
        if self.damaged(DSHIELD) && self.game.shldup {
            self.prout("***Shields knocked down.");
            self.game.shldup = false;
        }
        if self.damaged(DCLOAK) && self.game.iscloaked {
            self.prout("***Cloaking device rendered inoperative.");
            self.game.iscloaked = false;
        }
    }

    /// `attack(torps_ok)`: bad guy attacks us. `torps_ok == false` forces
    /// use of phasers in an attack.
    pub fn attack(&mut self, torps_ok: bool) {
        if self.game.iscloaked {
            return;
        }
        // game could be over at this point, check
        if self.game.alldone {
            return;
        }
        let mut attempt = false;
        let mut ihurt = false;
        let mut hitmax = 0.0f64;
        let mut hittot = 0.0f64;
        let mut chgfac = 1.0;
        let mut where_ = "neither";
        if self.game.idebug {
            self.prout("=== ATTACK!");
        }
        // Tholian gets to move before attacking
        if self.game.tholian.is_some() {
            self.movetholian();
        }
        // if you have just entered the RNZ, you'll get a warning
        if self.game.neutz {
            // The one chance not to be attacked
            self.game.neutz = false;
            return;
        }
        // commanders get a chance to tac-move towards you
        let q = self.game.quadrant;
        if (((self.game.state.kcmdr.contains(&q) || self.game.state.kscmdr == q) && !self.game.justin)
            || self.game.skill == SKILL_EMERITUS)
            && torps_ok
        {
            let moves = self.moveklings();
            for mv in moves {
                let enemy = mv.enemy;
                let etype = self.game.pool[enemy].etype;
                if mv.left {
                    // we know about this if either short or long range
                    // sensors are working
                    if self.damaged(DSRSENS) && self.damaged(DLRSENS) && self.game.condition != "docked" {
                        let s = crmena(true, etype, "sector", &mv.oldloc.to_string())
                            + &pf!(" escapes to Quadrant %s (and regains strength).", mv.goto);
                        self.prout(&s);
                    }
                } else {
                    // Enemy still in-sector
                    if self.enemy_move(enemy, Some(mv.goto)) && (!self.damaged(DSRSENS) || self.game.condition == "docked") {
                        let s = pf!("*** %s from Sector %s", cramen(etype), mv.oldloc);
                        self.proutn(&s);
                        if self.game.pool[enemy].kdist < mv.olddist {
                            self.proutn(" advances to ");
                        } else {
                            self.proutn(" retreats to ");
                        }
                        let s = pf!("Sector %s.", mv.goto);
                        self.prout(&s);
                    }
                }
            }
            self.sortenemies();
        }
        // if no enemies remain after movement, we're done
        if self.game.enemies.is_empty() || (self.game.enemies.len() == 1 && self.thing.at(self.game.quadrant)) {
            return;
        }
        // set up partial hits if attack happens during shield status change
        let pfac = 1.0 / self.game.inshld;
        if self.game.shldchg {
            chgfac = 0.25 + self.rnd_real1(0.5);
        }
        self.skip(1);
        // message verbosity control
        if self.game.skill <= SKILL_FAIR {
            where_ = "sector";
        }
        // Python's list iterator over the live list: torpedoes can kill
        // (remove) or displace and re-sort enemies mid-loop.
        let mut k = 0;
        while k < self.game.enemies.len() {
            let enemy = self.game.enemies[k];
            k += 1;
            if self.game.pool[enemy].power < 0.0 {
                continue; // too weak to attack
            }
            // compute hit strength and diminish shield power
            let mut r = self.rnd_real();
            // Increase chance of photon torpedos if docked or enemy energy is low
            if self.game.condition == "docked" {
                r *= 0.25;
            }
            if self.game.pool[enemy].power < 500.0 {
                r *= 0.25;
            }
            let etype = self.game.pool[enemy].etype;
            if etype == 'T' || etype == '?' {
                continue;
            }
            // different enemies have different probabilities of throwing a torp
            let usephasers = !torps_ok
                || (etype == 'K' && r > 0.0005)
                || (etype == 'C' && r > 0.015)
                || (etype == 'R' && r > 0.3)
                || (etype == 'S' && r > 0.07)
                || (etype == '?' && r > 0.05);
            let mut hit: f64;
            if usephasers {
                // Enemy uses phasers
                if self.game.condition == "docked" {
                    continue; // Don't waste the effort!
                }
                attempt = true; // Attempt to attack
                let dustfac = self.rnd_real2(0.8, 0.85);
                let e = &mut self.game.pool[enemy];
                hit = e.power * pypow(dustfac, e.kavgd);
                e.power *= 0.75;
            } else {
                // Enemy uses photon torpedo
                // We should be able to make the bearing() method work here
                let eloc = self.game.pool[enemy].location;
                let pcourse = 1.90985
                    * ((self.game.sector.j - eloc.j) as f64).atan2((eloc.i - self.game.sector.i) as f64);
                self.proutn("***TORPEDO INCOMING");
                if !self.damaged(DSRSENS) {
                    let s = " From ".to_string() + &crmena(false, etype, where_, &eloc.to_string());
                    self.proutn(&s);
                }
                attempt = true;
                self.prout("  ");
                let r1 = self.rnd_real();
                let r2 = self.rnd_real();
                let mut dispersion = (r1 + r2) * 0.5 - 0.5;
                dispersion += 0.002 * self.game.pool[enemy].power * dispersion;
                let h = self.torpedo(eloc, pcourse, dispersion, 1, 1);
                if self.game.unwon() == 0 {
                    self.finish(FWON); // Klingons did themselves in!
                }
                if self.game.here().supernova || self.game.alldone {
                    return; // Supernova or finished
                }
                match h {
                    None => continue,
                    Some(x) => hit = x,
                }
            }
            // incoming phaser or torpedo, shields may dissipate it
            if self.game.shldup || self.game.shldchg || self.game.condition == "docked" {
                // shields will take hits
                let mut propor = pfac * self.game.shield;
                if self.game.condition == "docked" {
                    propor *= 2.1;
                }
                // max(propor, 0.1)
                if 0.1 > propor {
                    propor = 0.1;
                }
                let hitsh = propor * chgfac * hit + 1.0;
                let mut absorb = 0.8 * hitsh;
                // min(absorb, game.shield)
                if self.game.shield < absorb {
                    absorb = self.game.shield;
                }
                self.game.shield -= absorb;
                hit -= hitsh;
                // taking a hit blasts us out of a starbase dock
                if self.game.condition == "docked" {
                    self.dock(false);
                }
                // but the shields may take care of it
                if propor > 0.1 && hit < 0.005 * self.game.energy {
                    continue;
                }
            }
            // hit from this opponent got through shields, so take damage
            ihurt = true;
            let s = pf!("%d unit hit", trunc(hit));
            self.proutn(&s);
            if (self.damaged(DSRSENS) && usephasers) || self.game.skill <= SKILL_FAIR {
                let s = " on the ".to_string() + &self.crmshp();
                self.proutn(&s);
            }
            if !self.damaged(DSRSENS) && usephasers {
                let eloc = self.game.pool[enemy].location;
                let s = " from ".to_string() + &crmena(false, etype, where_, &eloc.to_string());
                self.prout(&s);
            }
            self.skip(1);
            // Decide if hit is critical
            // max(hitmax, hit)
            if hit > hitmax {
                hitmax = hit;
            }
            hittot += hit;
            self.fry(hit);
            self.game.energy -= hit;
        }
        if self.game.energy <= 0.0 {
            // Returning home upon your shield, not with it...
            self.finish(FBATTLE);
            return;
        }
        if !attempt && self.game.condition == "docked" {
            self.prout("***Enemies decide against attacking your ship.");
        }
        let percent = 100.0 * pfac * self.game.shield + 0.5;
        if !ihurt {
            // Shields fully protect ship
            self.proutn("Enemy attack reduces shield strength to ");
        } else {
            // Emit message if starship suffered hit(s)
            self.skip(1);
            let s = pf!("Energy left %2d    shields ", trunc(self.game.energy));
            self.proutn(&s);
            if self.game.shldup {
                self.proutn("up ");
            } else if !self.damaged(DSHIELD) {
                self.proutn("down ");
            } else {
                self.proutn("damaged, ");
            }
        }
        let s = pf!("%d%%,   torpedoes left %d", percent, self.game.torps);
        self.prout(&s);
        // Check if anyone was hurt
        if hitmax >= 200.0 || hittot >= 500.0 {
            let icas = self.rnd_integer(trunc(hittot * 0.015) as f64);
            if icas >= 2 {
                self.skip(1);
                let s = pf!("Mc Coy-  \"Sickbay to bridge.  We suffered %d casualties", icas);
                self.prout(&s);
                self.prout("   in that last attack.\"");
                self.game.casual += icas;
                self.game.state.crew -= icas;
            }
        }
        // After attack, reset average distance to enemies
        for k in 0..self.game.enemies.len() {
            let e = self.game.enemies[k];
            self.game.pool[e].kavgd = self.game.pool[e].kdist;
        }
        self.sortenemies();
    }

    /// `deadkl(w, etype, mv)`: kill a Klingon, Tholian, Romulan, or Thingy.
    /// `mv` allows the enemy to "move" before dying.
    pub fn deadkl(&mut self, w: Coord, etype: char, mv: Coord) {
        self.proutn(&crmena(true, etype, "sector", &mv.to_string()));
        // Decide what kind of enemy it is and update appropriately
        match etype {
            'R' => {
                // Chalk up a Romulan
                self.game.here_mut().romulans -= 1;
                self.game.irhere -= 1;
                self.game.state.nromrem -= 1;
            }
            'T' => {
                // Killed a Tholian
                self.game.tholian = None;
            }
            '?' => {
                // Remove the Thingy from the galaxy while keeping its location queryable.
                self.thing.location.invalidate();
            }
            _ => {
                // Killed some type of Klingon
                self.game.here_mut().klingons -= 1;
                self.game.klhere -= 1;
                if etype == 'C' {
                    let q = self.game.quadrant;
                    if let Some(idx) = self.game.state.kcmdr.iter().position(|c| *c == q) {
                        self.game.state.kcmdr.remove(idx);
                    }
                    self.unschedule(FTBEAM);
                    if !self.game.state.kcmdr.is_empty() {
                        let v = self.expran(1.0 * self.game.incom as f64 / self.game.state.kcmdr.len() as f64);
                        self.schedule(FTBEAM, v);
                    }
                    if self.is_scheduled(FCDBAS) && self.game.battle == self.game.quadrant {
                        self.unschedule(FCDBAS);
                    }
                } else if etype == 'S' {
                    self.game.state.nscrem -= 1;
                    self.game.state.kscmdr.invalidate();
                    self.game.isatb = 0;
                    self.game.iscate = false;
                    self.unschedule(FSCMOVE);
                    self.unschedule(FSCDBAS);
                }
            }
        }
        // For each kind of enemy, finish message to player
        self.prout(" destroyed.");
        // Avoid dividing by zero after victory, but still remove the final enemy.
        if self.game.unwon() != 0 {
            self.game.recompute();
        }
        // Remove enemy ship from arrays describing local conditions
        let found = self.game.enemies.iter().copied().find(|&e| self.game.pool[e].location == w);
        if let Some(e) = found {
            self.enemy_move(e, None);
        }
    }
}
