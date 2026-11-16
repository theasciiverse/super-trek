//! The event queue: cancelrest, events (with tractorbeam and destroybase), wait (`sst` 2577-3100). schedule/unschedule/... are in sst.rs.

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::Sst;

/// The locals of `events()` that its nested `destroybase()` reads.
///
/// Python's `hold` is a Coord object that can be the very object
/// `game.battle` names (after `hold = game.battle` or `game.battle = hold`).
/// While they are the same object, an in-place `game.battle.invalidate()`
/// invalidates `hold` too; `hold_is_battle` tracks that sharing.
struct EvLocals {
    hold: Coord,
    hold_is_battle: bool,
}

impl EvLocals {
    /// `game.battle.invalidate()` seen through the alias.
    fn battle_invalidated(&mut self) {
        if self.hold_is_battle {
            self.hold.invalidate();
        }
    }
}

impl Sst {
    /// `cancelrest()`: rest period is interrupted by event.
    pub fn cancelrest(&mut self) -> bool {
        if self.game.resting {
            self.skip(1);
            self.proutn("Mr. Spock-  \"Captain, shall we cancel the rest period?\"");
            if self.ja() {
                self.game.resting = false;
                self.game.optime = 0.0;
                return true;
            }
        }
        false
    }

    /// `str(planet)` for `%s` of a `Quadrant.planet`: None prints "None".
    fn planet_str(&self, p: Option<usize>) -> String {
        match p {
            None => "None".into(),
            // DEVIATION: Python's Planet.__str__ returns the name, and for an
            // uninhabited planet that is None, so `%s` raises TypeError
            // (`sst` 3043, 3047). We print "None", as for a missing planet.
            Some(k) => self.game.state.planets[k].name.clone().unwrap_or_else(|| "None".into()),
        }
    }

    /// `galaxy[c.i][c.j].starbase`, false for a None coordinate.
    fn starbase_at(&self, c: Coord) -> bool {
        // DEVIATION: Python indexes the galaxy with None and raises TypeError
        // (`sst` 2875, 2885) when the coordinate is invalid; we read "no base".
        if c.is_none() {
            return false;
        }
        self.game.state.gal(c).starbase
    }

    /// `tractorbeam(yank)`, nested in `events()`; `evcode` and `i` are the
    /// enclosing function's locals.
    fn tractorbeam(&mut self, yank: f64, evcode: usize, i: usize) {
        self.announce();
        self.game.optime = (10.0 / (7.5 * 7.5)) * yank; // 7.5 is yank rate (warp 7.5)
        self.skip(1);
        let s = "***".to_string() + &self.crmshp() + " caught in long range tractor beam--";
        self.prout(&s);
        // If Kirk & Co. screwing around on planet, handle
        self.atover(true); // atover(true) is Grab
        if self.game.alldone {
            return;
        }
        if self.game.icraft {
            // Caught in Galileo?
            self.finish(FSTRACTOR);
            return;
        }
        // Check to see if shuttle is aboard
        if self.game.iscraft == "offship" {
            self.skip(1);
            if self.withprob(0.5) {
                self.prout("Galileo, left on the planet surface, is captured");
                self.prout("by aliens and made into a flying McDonald's.");
                self.game.damage[DSHUTTL] = -10.0;
                self.game.iscraft = "removed".into();
            } else {
                self.prout("Galileo, left on the planet surface, is well hidden.");
            }
        }
        if evcode == FSPY {
            self.game.quadrant = self.game.state.kscmdr;
        } else {
            self.game.quadrant = self.game.state.kcmdr[i];
        }
        self.game.sector = self.randplace(QUADSIZE);
        let s = self.crmshp() + &pf!(" is pulled to Quadrant %s, Sector %s", self.game.quadrant, self.game.sector);
        self.prout(&s);
        if self.game.resting {
            self.prout("(Remainder of rest/repair period cancelled.)");
            self.game.resting = false;
        }
        if !self.game.shldup {
            if !self.damaged(DSHIELD) && self.game.shield > 0.0 {
                self.doshield(true); // raise shields
                self.game.shldchg = false;
            } else {
                self.prout("(Shields not currently useable.)");
            }
        }
        self.newqad();
        // Adjust finish time to time of tractor beaming?
        // fintim = game.state.date+game.optime
        self.attack(false);
        if self.game.state.kcmdr.is_empty() {
            self.unschedule(FTBEAM);
        } else {
            let e = self.expran(1.5 * self.game.intime / self.game.state.kcmdr.len() as f64);
            self.schedule(FTBEAM, self.game.optime + e);
        }
    }

    /// `destroybase()`, nested in `events()`: code merges here for any
    /// commander destroying a starbase.
    fn destroybase(&mut self, lv: &mut EvLocals) {
        // Not perfect, but will have to do
        // Handle case where base is in same quadrant as starship
        let battle = self.game.battle;
        if battle == self.game.quadrant {
            self.game.state.page_mut(battle).starbase = false;
            // DEVIATION: Python raises TypeError here when game.base is None
            // or invalidated (`sst` 2660); we skip the map update then.
            if !self.game.base.is_none() {
                let b = self.game.base;
                self.game.set_q(b, '.');
            }
            self.game.base.invalidate();
            self.newcnd();
            self.skip(1);
            self.prout("Spock-  \"Captain, I believe the starbase has been destroyed.\"");
        } else if !self.game.state.baseq.is_empty() && self.communicating() {
            // Get word via subspace radio
            self.announce();
            self.skip(1);
            self.prout("Lt. Uhura-  \"Captain, Starfleet Command reports that");
            self.proutn(&pf!("   the starbase in Quadrant %s has been destroyed by", battle));
            if self.game.isatb == 2 {
                self.prout("the Klingon Super-Commander");
            } else {
                self.prout("a Klingon Commander");
            }
            self.game.state.page_mut(battle).starbase = false;
        }
        // Remove Starbase from galaxy
        self.game.state.gal_mut(battle).starbase = false;
        self.game.state.baseq.retain(|x| *x != battle);
        if self.game.isatb == 2 {
            // reinstate a commander's base attack
            self.game.battle = lv.hold;
            lv.hold_is_battle = true;
            self.game.isatb = 0;
        } else {
            // DEVIATION: when game.quadrant is the same object as game.battle
            // (ABANDON put the ship at the very baseq entry a commander was
            // attacking), Python invalidates the ship's quadrant here too and
            // crashes later; our Coords are values, so only battle changes.
            self.game.battle.invalidate();
            lv.battle_invalidated();
        }
    }

    /// `events()`: run through the event queue looking for things to do.
    pub fn events(&mut self) {
        let mut i: usize = 0;
        let fintim = self.game.state.date + self.game.optime;
        let mut ictbeam = false;
        let mut istract = false;
        let mut w: Coord;
        // `hold = Coord()`; set to 0, 0 (a1) below: a known quirk, kept.
        let mut lv = EvLocals { hold: Coord::none(), hold_is_battle: false };

        if self.game.idebug {
            let s = pf!("=== EVENTS from %.2f to %.2f:", self.game.state.date, fintim);
            self.prout(&s);
            for i in 1..NEVENTS {
                match i {
                    FSNOVA => self.proutn("=== Supernova       "),
                    FTBEAM => self.proutn("=== T Beam          "),
                    FSNAP => self.proutn("=== Snapshot        "),
                    FBATTAK => self.proutn("=== Base Attack     "),
                    FCDBAS => self.proutn("=== Base Destroy    "),
                    FSCMOVE => self.proutn("=== SC Move         "),
                    FSCDBAS => self.proutn("=== SC Base Destroy "),
                    FDSPROB => self.proutn("=== Probe Move      "),
                    FDISTR => self.proutn("=== Distress Call   "),
                    FENSLV => self.proutn("=== Enslavement     "),
                    FREPRO => self.proutn("=== Klingon Build   "),
                    _ => {}
                }
                if self.is_scheduled(i) {
                    let s = pf!("%.2f", self.scheduled(i));
                    self.prout(&s);
                } else {
                    self.prout("never");
                }
            }
        }
        let radio_was_broken = self.damaged(DRADIO);
        lv.hold = Coord::new(0, 0);
        loop {
            // Select earliest extraneous event, evcode==0 if no events
            let mut evcode = FSPY;
            if self.game.alldone {
                return;
            }
            let mut datemin = fintim;
            for l in 1..NEVENTS {
                if self.game.future[l].date < datemin {
                    evcode = l;
                    if self.game.idebug {
                        let s = pf!("== Event %d fires", evcode);
                        self.prout(&s);
                    }
                    datemin = self.game.future[l].date;
                }
            }
            let xtime = datemin - self.game.state.date;
            if self.game.iscloaked {
                self.game.energy -= xtime * 500.0;
                if self.game.energy <= 0.0 {
                    self.finish(FNRG);
                    return;
                }
            }
            self.game.state.date = datemin;
            // Decrement Federation resources and recompute remaining time
            self.game.state.remres -= (self.game.remkl() + 4 * self.game.state.kcmdr.len() as i64) as f64 * xtime;
            // DEVIATION: with no Klingons and no commanders left Python's
            // recompute() raises ZeroDivisionError; ours yields inf (or NaN).
            self.game.recompute();
            if self.game.state.remtime <= 0.0 {
                self.finish(FDEPLETE);
                return;
            }
            // Any crew left alive?
            if self.game.state.crew <= 0 {
                self.finish(FCREW);
                return;
            }
            // Is life support adequate?
            if self.damaged(DLIFSUP) && self.game.condition != "docked" {
                if self.game.lsupres < xtime && self.game.damage[DLIFSUP] > self.game.lsupres {
                    self.finish(FLIFESUP);
                    return;
                }
                self.game.lsupres -= xtime;
                if self.game.damage[DLIFSUP] <= xtime {
                    self.game.lsupres = self.game.inlsr;
                }
            }
            // Fix devices
            let mut repair = xtime;
            if self.game.condition == "docked" {
                repair /= DOCKFAC;
            }
            // Don't fix Deathray here
            for l in 0..NDEVICES {
                if self.game.damage[l] > 0.0 && l != DDRAY {
                    if self.game.damage[l] - repair > 0.0 {
                        self.game.damage[l] -= repair;
                    } else {
                        self.game.damage[l] = 0.0;
                    }
                }
            }
            // If radio repaired, update star chart and attack reports
            if radio_was_broken && !self.damaged(DRADIO) {
                self.prout("Lt. Uhura- \"Captain, the sub-space radio is working and");
                self.prout("   surveillance reports are coming in.");
                self.skip(1);
                if !self.game.iseenit {
                    self.attackreport(false);
                    self.game.iseenit = true;
                }
                self.rechart();
                self.prout("   The star chart is now up to date.\"");
                self.skip(1);
            }
            // Cause extraneous event EVCODE to occur
            self.game.optime -= xtime;
            match evcode {
                FSNOVA => {
                    // Supernova
                    self.announce();
                    self.supernova(None);
                    let e = self.expran(0.5 * self.game.intime);
                    self.schedule(FSNOVA, e);
                    if self.game.here().supernova {
                        return;
                    }
                }
                FSPY => {
                    // Check with spy to see if SC should tractor beam
                    if self.game.state.nscrem == 0
                        || self.game.iscloaked
                        || ictbeam
                        || istract
                        || self.game.condition == "docked"
                        || self.game.isatb == 1
                        || self.game.iscate
                    {
                        return;
                    }
                    let g = &self.game;
                    if g.ientesc
                        || (g.energy < 2000.0 && g.torps < 4 && g.shield < 1250.0)
                        || (g.damaged(DPHASER) && (g.damaged(DPHOTON) || g.torps < 4))
                        || (g.damaged(DSHIELD)
                            && (g.energy < 2500.0 || g.damaged(DPHASER))
                            && (g.torps < 5 || g.damaged(DPHOTON)))
                    {
                        // Tractor-beam her!
                        istract = true;
                        ictbeam = true;
                        let yank = self.game.state.kscmdr.sub(self.game.quadrant).distance();
                        self.tractorbeam(yank, evcode, i);
                    } else {
                        return;
                    }
                }
                FTBEAM => {
                    // Tractor beam
                    if self.game.state.kcmdr.is_empty() {
                        self.unschedule(FTBEAM);
                        continue;
                    }
                    i = self.rnd_integer(self.game.state.kcmdr.len() as f64) as usize;
                    let yank = self.game.state.kcmdr[i].sub(self.game.quadrant).distance();
                    if istract || self.game.condition == "docked" || self.game.iscloaked || yank == 0.0 {
                        // Drats! Have to reschedule
                        let e = self.expran(1.5 * self.game.intime / self.game.state.kcmdr.len() as f64);
                        self.schedule(FTBEAM, self.game.optime + e);
                        continue;
                    }
                    ictbeam = true;
                    self.tractorbeam(yank, evcode, i);
                }
                FSNAP => {
                    // Snapshot of the universe (for time warp)
                    self.game.snapsht = self.game.state.clone();
                    self.game.state.snap = true;
                    let e = self.expran(0.5 * self.game.intime);
                    self.schedule(FSNAP, e);
                }
                FBATTAK => {
                    // Commander attacks starbase
                    if self.game.state.kcmdr.is_empty() || self.game.state.baseq.is_empty() {
                        // no can do
                        self.unschedule(FBATTAK);
                        self.unschedule(FCDBAS);
                        continue;
                    }
                    let mut found: Option<Coord> = None;
                    'outer: for &ibq in &self.game.state.baseq {
                        for &cmdr in &self.game.state.kcmdr {
                            if ibq == cmdr && ibq != self.game.quadrant && ibq != self.game.state.kscmdr {
                                found = Some(ibq);
                                break 'outer;
                            }
                        }
                    }
                    let Some(ibq) = found else {
                        // no match found -- try later
                        let e = self.expran(0.3 * self.game.intime);
                        self.schedule(FBATTAK, e);
                        self.unschedule(FCDBAS);
                        continue;
                    };
                    // commander + starbase combination found -- launch attack
                    self.game.battle = ibq;
                    lv.hold_is_battle = false;
                    let r = self.rnd_real2(1.0, 4.0);
                    self.schedule(FCDBAS, r);
                    if self.game.isatb != 0 {
                        // extra time if SC already attacking
                        let d = self.scheduled(FSCDBAS) - self.game.state.date;
                        self.postpone(FCDBAS, d);
                    }
                    let e = self.expran(0.3 * self.game.intime);
                    self.game.future[FBATTAK].date = self.game.future[FCDBAS].date + e;
                    self.game.iseenit = false;
                    if !self.communicating() {
                        continue; // No warning :-(
                    }
                    self.game.iseenit = true;
                    self.announce();
                    self.skip(1);
                    let s = pf!("Lt. Uhura-  \"Captain, the starbase in Quadrant %s", self.game.battle);
                    self.prout(&s);
                    self.prout("   reports that it is under attack and that it can");
                    let s = pf!("   hold out only until stardate %d.\"", trunc(self.scheduled(FCDBAS)));
                    self.prout(&s);
                    if self.cancelrest() {
                        return;
                    }
                }
                FSCDBAS => {
                    // Supercommander destroys base
                    self.unschedule(FSCDBAS);
                    self.game.isatb = 2;
                    if !self.starbase_at(self.game.state.kscmdr) {
                        continue; // WAS RETURN!
                    }
                    lv.hold = self.game.battle;
                    // `game.battle = game.state.kscmdr`: battle and hold part
                    self.game.battle = self.game.state.kscmdr;
                    lv.hold_is_battle = false;
                    self.destroybase(&mut lv);
                }
                FCDBAS => {
                    // Commander succeeds in destroying base
                    self.unschedule(FCDBAS);
                    if self.game.state.baseq.is_empty() || !self.starbase_at(self.game.battle) {
                        self.game.battle.invalidate();
                        lv.battle_invalidated();
                        continue;
                    }
                    // find the lucky pair
                    let battle = self.game.battle;
                    if !self.game.state.kcmdr.contains(&battle) {
                        // No action to take after all
                        continue;
                    }
                    self.destroybase(&mut lv);
                }
                FSCMOVE => {
                    // Supercommander moves
                    self.schedule(FSCMOVE, 0.2777);
                    if !self.game.ientesc
                        && !istract
                        && self.game.isatb != 1
                        && (!self.game.iscate || !self.game.justin)
                    {
                        self.supercommander();
                    }
                }
                FDSPROB => {
                    // Move deep space probe
                    self.schedule(FDSPROB, 0.01);
                    // DEVIATION: Python raises AttributeError when the probe
                    // event fires with no probe; we drop the event instead.
                    let Some(mut probe) = self.game.probe.take() else {
                        self.unschedule(FDSPROB);
                        continue;
                    };
                    let same = probe.nextstep();
                    self.game.probe = Some(probe);
                    let pq = self.probe_quadrant();
                    if !same {
                        if !pq.valid_quadrant() || self.game.state.gal(pq).supernova {
                            // Left galaxy or ran into supernova
                            if self.communicating() {
                                self.announce();
                                self.skip(1);
                                self.proutn("Lt. Uhura-  \"The deep space probe ");
                                if !pq.valid_quadrant() {
                                    self.prout("has left the galaxy.\"");
                                } else {
                                    self.prout("is no longer transmitting.\"");
                                }
                            }
                            self.unschedule(FDSPROB);
                            continue;
                        }
                        if self.communicating() {
                            // announce()
                            self.skip(1);
                            let s = pf!("Lt. Uhura-  \"The deep space probe is now in Quadrant %s.\"", pq);
                            self.prout(&s);
                        }
                    }
                    let pquad = pq;
                    if self.communicating() {
                        let pdest = self.game.state.gal(pquad).clone();
                        let page = self.game.state.page_mut(pquad);
                        page.klingons = Some(pdest.klingons);
                        page.starbase = pdest.starbase;
                        page.stars = Some(pdest.stars);
                        page.supernova = pdest.supernova;
                        self.game.state.gal_mut(pquad).charted = true;
                    }
                    let probe = self.game.probe.as_mut().unwrap();
                    probe.moves -= 1; // One less to travel
                    let arrived = probe.arrived();
                    if arrived && self.game.isarmed && self.game.state.gal(pquad).stars != 0 {
                        let pq = self.probe_quadrant();
                        self.supernova(Some(pq)); // fire in the hole!
                        self.unschedule(FDSPROB);
                        if self.game.state.gal(pquad).supernova {
                            return;
                        }
                    }
                }
                FDISTR => {
                    // inhabited system issues distress call
                    self.unschedule(FDISTR);
                    // try a whole bunch of times to find something suitable
                    let mut got = None;
                    for _ in 0..100 {
                        // need a quadrant which is not the current one,
                        // which has some stars which are inhabited and
                        // not already under attack, which is not
                        // supernova'ed, and which has some Klingons in it
                        let wc = self.randplace(GALSIZE);
                        let q = self.game.state.gal(wc);
                        if !(self.game.quadrant == wc
                            || q.planet.is_none()
                            || !self.game.state.planets[q.planet.unwrap()].inhabited
                            || q.supernova
                            || q.status != "secure"
                            || q.klingons <= 0)
                        {
                            got = Some(wc);
                            break;
                        }
                    }
                    let Some(wc) = got else {
                        // can't seem to find one; ignore this call
                        if self.game.idebug {
                            self.prout("=== Couldn't find location for distress event.");
                        }
                        continue;
                    };
                    w = wc;
                    // got one!!  Schedule its enslavement
                    let e = self.expran(self.game.intime);
                    self.schedule(FENSLV, e);
                    self.game.future[FENSLV].quadrant = w;
                    self.game.state.gal_mut(w).status = "distressed".into();
                    // tell the captain about it if we can
                    if self.communicating() {
                        let pl = self.planet_str(self.game.state.gal(w).planet);
                        let s = pf!("Uhura- Captain, %s in Quadrant %s reports it is under attack", pl, w);
                        self.prout(&s);
                        self.prout("by a Klingon invasion fleet.");
                        if self.cancelrest() {
                            return;
                        }
                    }
                }
                FENSLV => {
                    // starsystem is enslaved
                    self.unschedule(FENSLV);
                    let evq = self.game.future[FENSLV].quadrant;
                    // see if current distress call still active
                    if self.game.state.gal(evq).klingons <= 0 {
                        self.game.state.gal_mut(evq).status = "secure".into();
                        continue;
                    }
                    self.game.state.gal_mut(evq).status = "enslaved".into();

                    // play stork and schedule the first baby
                    let e = self.expran(2.0 * self.game.intime);
                    self.schedule(FREPRO, e);
                    self.game.future[FREPRO].quadrant = evq;

                    // report the disaster if we can
                    if self.communicating() {
                        let pl = self.planet_str(self.game.state.gal(evq).planet);
                        let s = pf!("Uhura- We've lost contact with starsystem %s", pl);
                        self.prout(&s);
                        let s = pf!("in Quadrant %s.\n", evq);
                        self.prout(&s);
                    }
                }
                FREPRO => {
                    // Klingon reproduces
                    // If we ever switch to a real event queue, we'll need to
                    // explicitly retrieve and restore the x and y.
                    let e = self.expran(1.0 * self.game.intime);
                    self.schedule(FREPRO, e);
                    let evq = self.game.future[FREPRO].quadrant;
                    // see if current distress call still active
                    let mut qc = evq;
                    if self.game.state.gal(qc).klingons <= 0 {
                        self.game.state.gal_mut(qc).status = "secure".into();
                        continue;
                    }
                    if self.game.remkl() >= MAXKLGAME {
                        continue; // full right now
                    }
                    // reproduce one Klingon
                    w = evq;
                    if self.game.klhere >= MAXKLQUAD {
                        // this quadrant not ok, pick an adjacent one
                        let mut found = None;
                        'search: for mi in (w.i - 1)..(w.i + 2) {
                            for mj in (w.j - 1)..(w.j + 2) {
                                let m = Coord::new(mi, mj);
                                if !m.valid_quadrant() {
                                    continue;
                                }
                                qc = m;
                                let q = self.game.state.gal(m);
                                // check for this quad ok (not full & no snova)
                                if q.klingons >= MAXKLQUAD || q.supernova {
                                    continue;
                                }
                                found = Some(m);
                                break 'search;
                            }
                        }
                        match found {
                            // search for eligible quadrant failed
                            None => continue,
                            Some(m) => w = m,
                        }
                    }
                    // deliver the child
                    self.game.state.gal_mut(qc).klingons += 1;
                    if self.game.quadrant == w {
                        self.game.klhere += 1;
                        self.newkling(); // also adds it to game.enemies
                    }
                    // recompute time left
                    self.game.recompute();
                    if self.communicating() {
                        let qplanet = self.game.state.gal(qc).planet;
                        if self.game.quadrant == w {
                            self.prout("Spock- sensors indicate the Klingons have");
                            let s = pf!("launched a warship from %s.", self.planet_str(qplanet));
                            self.prout(&s);
                        } else {
                            self.prout("Uhura- Starfleet reports increased Klingon activity");
                            if qplanet.is_some() {
                                let s = pf!("near %s ", self.planet_str(qplanet));
                                self.proutn(&s);
                            }
                            let s = pf!("in Quadrant %s.", w);
                            self.prout(&s);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// `game.probe.quadrant()`
    fn probe_quadrant(&self) -> Coord {
        self.game.probe.as_ref().map(|p| p.quadrant()).unwrap_or_default() // Coord::none()
    }

    /// `wait()`: wait on events.
    pub fn wait(&mut self) {
        self.game.ididit = false;
        let mut key;
        loop {
            key = self.nexttok();
            if key != Tok::Eol {
                break;
            }
            self.proutn("How long? ");
            self.scanner.chew();
        }
        if key != Tok::Real {
            self.huh();
            return;
        }
        let orig_time = self.scanner.real;
        let mut delay = orig_time;
        if delay <= 0.0 {
            return;
        }
        if (!self.game.sandbox && delay >= self.game.state.remtime) || !self.game.enemies.is_empty() {
            self.proutn("Are you sure? ");
            if !self.ja() {
                return;
            }
        }
        // Alternate resting periods (events) with attacks
        self.game.resting = true;
        loop {
            if delay <= 0.0 {
                self.game.resting = false;
            }
            if !self.game.resting {
                if !self.game.sandbox {
                    let s = pf!("%d stardates left.", trunc(self.game.state.remtime));
                    self.prout(&s);
                }
                return;
            }
            let mut temp = delay;
            self.game.optime = delay;
            if !self.game.enemies.is_empty() {
                let rtime = self.rnd_real2(1.0, 2.0);
                // min(temp, rtime): the first of equals
                if rtime < temp {
                    temp = rtime;
                }
                self.game.optime = temp;
            }
            if self.game.optime < delay {
                self.attack(false);
            }
            if self.game.alldone {
                return;
            }
            self.events();
            self.game.ididit = true;
            if self.game.alldone {
                return;
            }
            delay -= temp;
            // Repair Deathray if long rest at starbase
            if orig_time - delay >= 9.99 && self.game.condition == "docked" {
                self.game.damage[DDRAY] = 0.0;
            }
            // leave if quadrant supernovas
            if self.game.here().supernova {
                break;
            }
        }
        self.game.resting = false;
        self.game.optime = 0.0;
    }
}
