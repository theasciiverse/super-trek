//! Being thrown about and getting out: atover, timwrp, probe, mayday, abandon (`sst` 4943-5313).

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, TrekError};

impl Sst {
    /// `atover(igrab)`: cope with being tossed out of quadrant by supernova
    /// or yanked by beam.
    pub fn atover(&mut self, igrab: bool) {
        self.scanner.chew();
        // is captain on planet?
        if self.game.landed {
            if self.damaged(DTRANSP) {
                self.finish(FPNOVA);
                return;
            }
            self.prout("Scotty rushes to the transporter controls.");
            if self.game.shldup {
                self.prout("But with the shields up it's hopeless.");
                self.finish(FPNOVA);
            }
            self.prouts("His desperate attempt to rescue you . . .");
            if self.withprob(0.5) {
                self.prout("fails.");
                self.finish(FPNOVA);
                return;
            }
            self.prout("SUCCEEDS!");
            if self.game.imine {
                self.game.imine = false;
                self.proutn("The crystals mined were ");
                if self.withprob(0.25) {
                    self.prout("lost.");
                } else {
                    self.prout("saved.");
                    self.game.icrystl = true;
                }
            }
        }
        if igrab {
            return;
        }
        // Check to see if captain in shuttle craft
        if self.game.icraft {
            self.finish(FSTRACTOR);
        }
        if self.game.alldone {
            return;
        }
        // Inform captain of attempt to reach safety
        self.skip(1);
        loop {
            if self.game.justin {
                self.prouts("***RED ALERT!  RED ALERT!");
                self.skip(1);
                let s = pf!("The %s has stopped in a quadrant containing", self.crmshp());
                self.proutn(&s);
                self.prouts("   a supernova.");
                self.skip(2);
            }
            let s = "***Emergency automatic override attempts to hurl ".to_string() + &self.crmshp();
            self.prout(&s);
            self.prout("safely out of quadrant.");
            let q = self.game.quadrant;
            self.game.state.page_mut(q).supernova = true;
            if !self.damaged(DRADIO) {
                self.game.state.gal_mut(q).charted = true;
            }
            // Try to use warp engines
            if self.damaged(DWARPEN) {
                self.skip(1);
                self.prout("Warp engines damaged.");
                self.finish(FSNOVAED);
                return;
            }
            self.game.warpfac = self.rnd_real2(6.0, 8.0);
            let s = pf!("Warp factor set to %d", trunc(self.game.warpfac));
            self.prout(&s);
            let power = 0.75 * self.game.energy;
            let wf = self.game.warpfac;
            let mut dist = power / (wf * wf * wf * (self.game.shldup as i64 + 1) as f64);
            // max() keeps the first of equals
            let r = self.rnd_real1(2.0f64.sqrt());
            if r > dist {
                dist = r;
            }
            let bearing = self.rnd_real1(12.0);
            let bugout = self.course(bearing, dist); // How dumb!
            self.game.optime = bugout.time(self.game.warpfac);
            self.game.justin = false;
            self.game.inorbit = false;
            self.warp(Some(bugout), true);
            if !self.game.justin {
                // This is bad news, we didn't leave quadrant.
                if self.game.alldone {
                    return;
                }
                self.skip(1);
                self.prout("Insufficient energy to leave quadrant.");
                self.finish(FSNOVAED);
                return;
            }
            // Repeat if another snova
            if !self.game.here().supernova {
                break;
            }
        }
        if self.game.unwon() == 0 {
            self.finish(FWON); // Snova killed remaining enemy.
        }
    }

    /// `timwrp()`: let's do the time warp again.
    pub fn timwrp(&mut self) {
        self.prout("***TIME WARP ENTERED.");
        if self.game.state.snap && self.withprob(0.5) {
            // Go back in time
            let s = pf!(
                "You are traveling backwards in time %d stardates.",
                trunc(self.game.state.date - self.game.snapsht.date)
            );
            self.prout(&s);
            // Python binds game.state to the very snapsht object, so both
            // names show snap False; later changes to state also show in
            // snapsht until the next FSNAP rebinds it with a deep copy.
            // snapsht is read only while state.snap is true, which takes
            // that FSNAP, so the copy here behaves the same.
            self.game.state = self.game.snapsht.clone();
            self.game.state.snap = false;
            self.game.snapsht.snap = false;
            if !self.game.state.kcmdr.is_empty() {
                let n = self.game.state.kcmdr.len() as f64;
                let e = self.expran(self.game.intime / n);
                self.schedule(FTBEAM, e);
                let e = self.expran(0.3 * self.game.intime);
                self.schedule(FBATTAK, e);
            }
            let e = self.expran(0.5 * self.game.intime);
            self.schedule(FSNOVA, e);
            // next snapshot will be sooner
            let e = self.expran(0.25 * self.game.state.remtime);
            self.schedule(FSNAP, e);

            if self.game.state.nscrem != 0 {
                self.schedule(FSCMOVE, 0.2777);
            }
            self.game.isatb = 0;
            self.unschedule(FCDBAS);
            self.unschedule(FSCDBAS);
            self.game.battle.invalidate();
            // Make sure Galileo is consistant -- Snapshot may have been taken
            // when on planet, which would give us two Galileos!
            let mut gotit = false;
            for l in 0..self.game.inplan.max(0) as usize {
                if self.game.state.planets[l].known == "shuttle_down" {
                    gotit = true;
                    if self.game.iscraft == "onship" && self.game.ship == 'E' {
                        self.prout("Chekov-  \"Security reports the Galileo has disappeared, Sir!");
                        self.game.iscraft = "offship".into();
                    }
                }
            }
            // Likewise, if in the original time the Galileo was abandoned, but
            // was on ship earlier, it would have vanished -- let's restore it.
            if self.game.iscraft == "offship" && !gotit && self.game.damage[DSHUTTL] >= 0.0 {
                self.prout("Chekov-  \"Security reports the Galileo has reappeared in the dock!\"");
                self.game.iscraft = "onship".into();
            }
            // There used to be code to do the actual reconstrction here,
            // but the starchart is now part of the snapshotted galaxy state.
            self.prout("Spock has reconstructed a correct star chart from memory");
        } else {
            // Go forward in time
            self.game.optime = self.expran(0.5 * self.game.intime);
            let s = pf!("You are traveling forward in time %d stardates.", trunc(self.game.optime));
            self.prout(&s);
            // cheat to make sure no tractor beams occur during time warp
            self.postpone(FTBEAM, self.game.optime);
            self.game.damage[DRADIO] += self.game.optime;
        }
        // game.iplnet: Python's still points at the Planet object of the
        // state before the swap, ours indexes the new state's planets.
        // Nothing reads it before newqad() below resets it, so the
        // difference is not observable.
        self.newqad();
        self.events(); // Stas Sergeev added this -- do pending events
    }

    /// `probe()`: launch deep-space probe.
    pub fn probe(&mut self) {
        // New code to launch a deep space probe
        if self.game.nprobes == 0 {
            self.scanner.chew();
            self.skip(1);
            if self.game.ship == 'E' {
                self.prout("Engineer Scott- \"We have no more deep space probes, Sir.\"");
            } else {
                self.prout("Ye Faerie Queene has no deep space probes.");
            }
            return;
        }
        if self.damaged(DDSP) {
            self.scanner.chew();
            self.skip(1);
            self.prout("Engineer Scott- \"The probe launcher is damaged, Sir.\"");
            return;
        }
        if self.is_scheduled(FDSPROB) {
            self.scanner.chew();
            self.skip(1);
            if self.damaged(DRADIO) && self.game.condition != "docked" {
                self.prout("Spock-  \"Records show the previous probe has not yet");
                self.prout("   reached its destination.\"");
            } else {
                self.prout("Uhura- \"The previous probe is still reporting data, Sir.\"");
            }
            return;
        }
        let mut key = self.nexttok();
        if key == Tok::Eol {
            if self.game.nprobes == 1 {
                self.prout("1 probe left.");
            } else {
                let s = pf!("%d probes left", self.game.nprobes);
                self.prout(&s);
            }
            self.proutn("Are you sure you want to fire a probe? ");
            if !self.ja() {
                return;
            }
        }
        self.game.isarmed = false;
        if key == Tok::Alpha && self.scanner.token == "armed" {
            self.game.isarmed = true;
            key = self.nexttok();
        } else if key == Tok::Eol {
            self.proutn("Arm NOVAMAX warhead? ");
            self.game.isarmed = self.ja();
        } else if key == Tok::Real {
            // first element of course
            let t = self.scanner.token.clone();
            self.scanner.push(&t);
        }
        let _ = key;
        match self.getcourse(true) {
            Ok(c) => self.game.probe = Some(c),
            Err(TrekError) => return,
        }
        self.game.nprobes -= 1;
        self.schedule(FDSPROB, 0.01); // Time to move one sector
        self.prout("Ensign Chekov-  \"The deep space probe is launched, Captain.\"");
        self.game.ididit = true;
    }

    /// `mayday()`: yell for help from nearest starbase.
    pub fn mayday(&mut self) {
        // There's more than one way to move in this game!
        self.scanner.chew();
        // Test for conditions which prevent calling for help
        if self.game.condition == "docked" {
            self.prout("Lt. Uhura-  \"But Captain, we're already docked.\"");
            return;
        }
        if self.damaged(DRADIO) {
            self.prout("Subspace radio damaged.");
            return;
        }
        if self.game.state.baseq.is_empty() {
            self.prout("Lt. Uhura-  \"Captain, I'm not getting any response from Starbase.\"");
            return;
        }
        if self.game.landed {
            let s = pf!("You must be aboard the %s.", self.crmshp());
            self.prout(&s);
            return;
        }
        // OK -- call for help from nearest starbase
        self.game.nhelp += 1;
        let ddist;
        if self.game.base.valid_sector() && self.game.here().starbase {
            // There's one in this quadrant
            ddist = self.game.base.sub(self.game.sector).distance();
        } else {
            // min() keeps the first of equals
            let gq = self.game.quadrant;
            let mut ibq = self.game.state.baseq[0];
            let mut best = ibq.sub(gq).distance();
            for &b in &self.game.state.baseq[1..] {
                let d = b.sub(gq).distance();
                if d < best {
                    best = d;
                    ibq = b;
                }
            }
            ddist = QUADSIZE as f64 * ibq.sub(gq).distance();
            // Since starbase not in quadrant, set up new quadrant
            self.game.quadrant = ibq;
            self.newqad();
        }
        // dematerialize starship
        let s = self.game.sector;
        self.game.set_q(s, '.');
        let s = pf!("Starbase in Quadrant %s responds--%s dematerializes", self.game.quadrant, self.crmshp());
        self.proutn(&s);
        self.game.sector.invalidate();
        for _m in 1..=5 {
            let w = self.scatter(self.game.base);
            if w.valid_sector() && self.game.q(w) == '.' {
                // found one -- finish up
                self.game.sector = w;
                break;
            }
        }
        if !self.game.sector.valid_sector() {
            self.prout("You have been lost in space...");
            self.finish(FMATERIALIZE);
            return;
        }
        // Give starbase three chances to rematerialize starship
        let probf = pypow(1.0 - pypow(0.98, ddist), 0.33333333);
        let mut succeeded = false;
        for m in 1..=3 {
            if m == 1 {
                self.proutn("1st");
            } else if m == 2 {
                self.proutn("2nd");
            } else if m == 3 {
                self.proutn("3rd");
            }
            let s = " attempt to re-materialize ".to_string() + &self.crmshp();
            self.proutn(&s);
            let sec = self.game.sector;
            self.game.set_q(sec, ['-', 'o', 'O'][m - 1]);
            self.warble();
            if self.rnd_real() > probf {
                succeeded = true;
                break;
            }
            self.prout("fails.");
        }
        if !succeeded {
            let sec = self.game.sector;
            self.game.set_q(sec, '?');
            self.game.alive = false;
            self.drawmaps(1);
            self.finish(FMATERIALIZE);
            return;
        }
        let sec = self.game.sector;
        let ship = self.game.ship;
        self.game.set_q(sec, ship);
        self.prout("succeeds.");
        self.dock(false);
        self.skip(1);
        self.prout("Lt. Uhura-  \"Captain, we made it!\"");
    }

    /// `abandon()`: abandon ship.
    pub fn abandon(&mut self) {
        self.scanner.chew();
        if self.game.condition == "docked" {
            if self.game.ship != 'E' {
                self.prout("You cannot abandon Ye Faerie Queene.");
                return;
            }
        } else {
            // Must take shuttle craft to exit
            if self.game.damage[DSHUTTL] == -1.0 {
                self.prout("Ye Faerie Queene has no shuttle craft.");
                return;
            }
            if self.game.damage[DSHUTTL] < 0.0 {
                self.prout("Shuttle craft now serving Big Macs.");
                return;
            }
            if self.game.damage[DSHUTTL] > 0.0 {
                self.prout("Shuttle craft damaged.");
                return;
            }
            if self.game.landed {
                self.prout("You must be aboard the ship.");
                return;
            }
            if self.game.iscraft != "onship" {
                self.prout("Shuttle craft not currently available.");
                return;
            }
            // Emit abandon ship messages
            self.skip(1);
            self.prouts("***ABANDON SHIP!  ABANDON SHIP!");
            self.skip(1);
            self.prouts("***ALL HANDS ABANDON SHIP!");
            self.skip(2);
            self.prout("Captain and crew escape in shuttle craft.");
            if self.game.state.baseq.is_empty() {
                // Oops! no place to go...
                self.finish(FABANDN);
                return;
            }
            let qplanet = self.game.here().planet;
            // Dispose of crew
            if (self.game.options & OPTION_WORLDS) == 0 && !self.damaged(DTRANSP) {
                self.prout("Remainder of ship's complement beam down");
                self.prout("to nearest habitable planet.");
            } else if qplanet.is_some() && !self.damaged(DTRANSP) {
                let name = self.game.state.planets[qplanet.unwrap()].name.clone();
                match name {
                    Some(name) => {
                        let s = pf!("Remainder of ship's complement beam down to %s.", name);
                        self.prout(&s);
                    }
                    None => {
                        // DEVIATION: an uninhabited planet has no name and
                        // Python's `"%s" % q.planet` raises TypeError
                        // (Planet.__str__ returned None), killing the game.
                        // The crew beam down as when WORLDS is off.
                        self.prout("Remainder of ship's complement beam down");
                        self.prout("to nearest habitable planet.");
                    }
                }
            } else {
                let s = pf!("Entire crew of %d left to die in outer space.", self.game.state.crew);
                self.prout(&s);
                self.game.casual += self.game.state.crew;
                self.game.abandoned += self.game.state.crew;
            }
            // If at least one base left, give 'em the Faerie Queene
            self.skip(1);
            self.game.icrystl = false; // crystals are lost
            self.game.nprobes = 0; // No probes
            self.prout("You are captured by Klingons and released to");
            self.prout("the Federation in a prisoner-of-war exchange.");
            let nb = self.rnd_integer(self.game.state.baseq.len() as f64) as usize;
            // Set up quadrant and position FQ adjacient to base
            if self.game.quadrant != self.game.state.baseq[nb] {
                // Python makes game.quadrant the same object as baseq[nb];
                // see the port notes on why that aliasing is not kept.
                self.game.quadrant = self.game.state.baseq[nb];
                self.game.sector.i = 5;
                self.game.sector.j = 5;
                self.newqad();
            }
            loop {
                // position next to base by trial and error
                let s = self.game.sector;
                self.game.set_q(s, '.');
                let mut found = false;
                for _attempt in 0..QUADSIZE {
                    self.game.sector = self.scatter(self.game.base);
                    if self.game.sector.valid_sector() && self.game.q(self.game.sector) == '.' {
                        found = true;
                        break;
                    }
                }
                if !found {
                    // No free adjacent sector found; rebuild the quadrant and retry.
                    self.game.sector = Coord::new(QUADSIZE / 2, QUADSIZE / 2);
                    self.newqad();
                    continue;
                }
                break; // found a spot
            }
        }
        // Get new commission
        let s = self.game.sector;
        self.game.set_q(s, 'F');
        self.game.ship = 'F';
        self.game.state.crew = FULLCREW;
        self.prout("Starfleet puts you in command of another ship,");
        self.prout("the Faerie Queene, which is antiquated but,");
        self.prout("still useable.");
        if self.game.icrystl {
            self.prout("The dilithium crystals have been moved.");
        }
        self.game.imine = false;
        self.game.iscraft = "offship".into(); // Galileo disappears
        // Resupply ship
        self.game.condition = "docked".into();
        for l in 0..NDEVICES {
            self.game.damage[l] = 0.0;
        }
        self.game.damage[DSHUTTL] = -1.0;
        self.game.inenrg = 3000.0;
        self.game.energy = 3000.0;
        self.game.inshld = 1250.0;
        self.game.shield = 1250.0;
        self.game.intorps = 6;
        self.game.torps = 6;
        self.game.inlsr = 3.0;
        self.game.lsupres = 3.0;
        self.game.shldup = false;
        self.game.warpfac = 5.0;
        self.game.brigcapacity = 300;
        self.game.brigfree = 300;
    }
}
