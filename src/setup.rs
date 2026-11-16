//! Setting up a game: prelim, setup, choose, dropin, newcnd, newkling,
//! newqad, setpassword (`sst` 6446-6456, 6883-7453).

use crate::consts::*;
use crate::coord::{Coord, dist};
use crate::pf;
use crate::py::{str_repr, trunc};
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit};
use crate::state::*;

impl Sst {
    /// `prelim()`: a historically correct banner.
    pub fn prelim(&mut self) {
        self.skip(2);
        self.prout("-SUPER- STAR TREK");
        self.skip(1);
    }

    /// `setup()`: prepare to play, set up the cosmos.
    pub fn setup(&mut self) {
        if self.choose() {
            return; // frozen game
        }
        let g = &mut self.game;
        g.alldone = false;
        g.gamewon = false;
        g.shldchg = false;
        g.shldup = false;
        g.ship = 'E';
        g.iscloaked = false;
        g.isviolreported = false;
        g.brigfree = 400;
        g.brigcapacity = 400;
        g.state.crew = FULLCREW;
        g.energy = 5000.0;
        g.inenrg = 5000.0;
        g.shield = 2500.0;
        g.inshld = 2500.0;
        g.inlsr = 4.0;
        g.lsupres = 4.0;
        self.game.quadrant = self.randplace(GALSIZE);
        self.game.sector = self.randplace(QUADSIZE);
        self.game.torps = 10;
        self.game.intorps = 10;
        self.game.nprobes = self.rnd_integer2(2.0, 5.0);
        self.game.warpfac = 5.0;
        for i in 0..NDEVICES {
            self.game.damage[i] = 0.0;
        }
        self.game.battle = Coord::none();
        let d = 100.0 * self.rnd_real2(20.0, 51.0);
        self.game.state.date = d;
        self.game.indate = d;
        let g = &mut self.game;
        g.nkinks = 0;
        g.nhelp = 0;
        g.casual = 0;
        g.abandoned = 0;
        g.kcaptured = 0;
        g.ncviol = 0;
        g.score = 0.0;
        g.perdate = 0.0;
        g.iscate = false;
        g.resting = false;
        g.imine = false;
        g.icrystl = false;
        g.icraft = false;
        g.isatb = 0;
        g.state.nplankl = 0;
        g.state.starkl = 0;
        g.state.basekl = 0;
        g.state.nworldkl = 0;
        g.iscraft = "onship".into();
        g.landed = false;
        g.alive = true;
        g.state.galaxy = vec![vec![Quadrant::default(); GS]; GS];
        g.state.chart = vec![vec![Page::default(); GS]; GS];
        g.state.planets = Vec::new();
        g.state.baseq = Vec::new();
        g.state.kcmdr = Vec::new();
        g.state.kscmdr = Coord::none();
        g.lastchart = FOREVER;
        // Put stars in the galaxy
        g.instar = 0;
        for i in 0..GS {
            for j in 0..GS {
                // min(10, QUADSIZE**2 / 10) is the int 10
                let k = self.rnd_integer2(1.0, 10.0);
                self.game.instar += k;
                self.game.state.galaxy[i][j].stars = k;
            }
        }
        // Locate star bases in galaxy
        if self.game.idebug {
            let s = pf!("=== Allocating %d bases", self.game.inbase);
            self.prout(&s);
        }
        let mut w;
        for i in 0..self.game.inbase {
            loop {
                loop {
                    w = self.randplace(GALSIZE);
                    if !self.game.state.gal(w).starbase {
                        break;
                    }
                }
                let mut contflag = false;
                for j in 1..i {
                    let distq = dist(w, self.game.state.baseq[j as usize]);
                    let lim = 6.0 * (BASEMAX + 1 - self.game.inbase) as f64;
                    if distq < lim && self.withprob(0.75) {
                        contflag = true;
                        if self.game.idebug {
                            let s = pf!("=== Abandoning base #%d at %s", i, w);
                            self.prout(&s);
                        }
                        break;
                    } else if distq < lim && self.game.idebug {
                        let s = pf!("=== Saving base #%d, close to #%d", i, j);
                        self.prout(&s);
                    }
                }
                if !contflag {
                    break;
                }
            }
            if self.game.idebug {
                let s = pf!("=== Placing base #%d in quadrant %s", i, w);
                self.prout(&s);
            }
            self.game.state.baseq.push(w);
            self.game.state.gal_mut(w).starbase = true;
            self.game.state.page_mut(w).starbase = true;
        }
        // Position ordinary Klingon Battle Cruisers
        let mut krem = self.game.inkling;
        let mut klumper = 0.25 * self.game.skill as f64 * (9.0 - self.game.length as f64) + 1.0;
        klumper = klumper.min(MAXKLQUAD as f64);
        loop {
            let r = self.rnd_real();
            let mut klump = trunc((1.0 - r * r) * klumper);
            klump = klump.min(krem);
            krem -= klump;
            loop {
                w = self.randplace(GALSIZE);
                let q = self.game.state.gal(w);
                if !q.supernova && q.klingons + klump <= MAXKLQUAD {
                    break;
                }
            }
            self.game.state.gal_mut(w).klingons += klump;
            if krem <= 0 {
                break;
            }
        }
        // Position Klingon Commander Ships
        for _ in 0..self.game.incom {
            loop {
                w = self.randplace(GALSIZE);
                if !self.welcoming(w) || self.game.state.kcmdr.contains(&w) {
                    continue;
                }
                if self.game.state.gal(w).klingons != 0 || self.withprob(0.25) {
                    break;
                }
            }
            self.game.state.gal_mut(w).klingons += 1;
            self.game.state.kcmdr.push(w);
        }
        // Locate planets in galaxy
        for i in 0..self.game.inplan {
            loop {
                w = self.randplace(GALSIZE);
                if self.game.state.gal(w).planet.is_none() {
                    break;
                }
            }
            let mut new = Planet { quadrant: w, crystals: "absent".into(), ..Planet::default() };
            if self.game.options & OPTION_WORLDS != 0 && i < NINHAB {
                new.pclass = Some("M".into());
                new.crystals = "absent".into();
                new.known = "known".into();
                new.name = Some(SYSTNAMES[i as usize].into());
                new.inhabited = true;
            } else {
                let k = self.rnd_integer2(0.0, 3.0);
                new.pclass = Some(["M", "N", "O"][k as usize].into());
                if self.withprob(0.33) {
                    new.crystals = "present".into();
                }
                new.known = "unknown".into();
                new.inhabited = false;
            }
            let idx = self.game.state.planets.len();
            self.game.state.planets.push(new);
            self.game.state.gal_mut(w).planet = Some(idx);
        }
        // Locate Romulans
        for _ in 0..self.game.state.nromrem {
            w = self.randplace(GALSIZE);
            self.game.state.gal_mut(w).romulans += 1;
        }
        // Place the Super-Commander if needed
        if self.game.state.nscrem > 0 {
            loop {
                w = self.randplace(GALSIZE);
                if self.welcoming(w) {
                    break;
                }
            }
            self.game.state.kscmdr = w;
            self.game.state.gal_mut(w).klingons += 1;
        }
        // Initialize times for extraneous events
        let v = self.expran(0.5 * self.game.intime);
        self.schedule(FSNOVA, v);
        if !self.game.state.kcmdr.is_empty() {
            let v = self.expran(1.5 * (self.game.intime / self.game.state.kcmdr.len() as f64));
            self.schedule(FTBEAM, v);
        } else {
            self.unschedule(FTBEAM);
        }
        let v = self.rnd_real2(1.0, 2.0);
        self.schedule(FSNAP, v); // Force an early snapshot
        let v = self.expran(0.3 * self.game.intime);
        self.schedule(FBATTAK, v);
        self.unschedule(FCDBAS);
        if self.game.state.nscrem != 0 {
            self.schedule(FSCMOVE, 0.2777);
        } else {
            self.unschedule(FSCMOVE);
        }
        self.unschedule(FSCDBAS);
        self.unschedule(FDSPROB);
        if self.game.options & OPTION_WORLDS != 0 && self.game.skill >= SKILL_GOOD {
            let v = self.expran(1.0 + self.game.intime);
            self.schedule(FDISTR, v);
        } else {
            self.unschedule(FDISTR);
        }
        self.unschedule(FENSLV);
        self.unschedule(FREPRO);
        // Place thing (in tournament game, we don't want one!), never at a
        // starbase, without disturbing the RNG chain.
        if self.game.tourn.is_none() {
            let state = self.game.lcg_x;
            loop {
                self.thing.location = self.randplace(GALSIZE);
                if !self.game.state.baseq.contains(&self.thing.location) {
                    break;
                }
            }
            self.game.lcg_x = state;
        }
        self.skip(2);
        self.game.state.snap = false;
        if self.game.skill == SKILL_NOVICE {
            let s = pf!("It is stardate %d. The Federation is being attacked by", trunc(self.game.state.date));
            self.prout(&s);
            self.prout("a deadly Klingon invasion force. As captain of the United");
            self.prout("Starship U.S.S. Enterprise, it is your mission to seek out");
            let s = pf!(
                "and destroy this invasion force of %d battle cruisers.",
                self.game.inkling + self.game.incom + self.game.inscom
            );
            self.prout(&s);
            if self.game.sandbox {
                self.prout("SANDBOX MODE: There is no time limit on your mission.");
            } else {
                let s = pf!("You have an initial allotment of %d stardates to complete", trunc(self.game.intime));
                self.prout(&s);
                self.prout("your mission.  As you proceed you may be given more time.");
            }
            self.skip(1);
            let s = pf!("You will have %d supporting starbases.", self.game.inbase);
            self.prout(&s);
            self.proutn("Starbase locations-  ");
        } else {
            let s = pf!("Stardate %d.", trunc(self.game.state.date));
            self.prout(&s);
            self.skip(1);
            let s = pf!("%d Klingons.", self.game.inkling + self.game.incom + self.game.inscom);
            self.prout(&s);
            self.prout("An unknown number of Romulans.");
            if self.game.state.nscrem != 0 {
                self.prout("And one (GULP) Super-Commander.");
            }
            if self.game.sandbox {
                self.prout("Sandbox mode - no time limit.");
            } else {
                let s = pf!("%d stardates.", trunc(self.game.intime));
                self.prout(&s);
            }
            let s = pf!("%d starbases in ", self.game.inbase);
            self.proutn(&s);
        }
        for i in 0..self.game.inbase as usize {
            let s = self.game.state.baseq[i].to_string();
            self.proutn(&s);
            self.proutn("  ");
        }
        self.skip(2);
        let s = pf!("The Enterprise is currently in Quadrant %s", self.game.quadrant);
        self.proutn(&s);
        let s = pf!(" Sector %s", self.game.sector);
        self.proutn(&s);
        self.skip(2);
        self.prout("Good Luck!");
        if self.game.state.nscrem != 0 {
            self.prout("  YOU'LL NEED IT.");
        }
        self.waitfor();
        self.clrscr();
        self.newqad();
        let n = self.game.enemies.len() as i64
            - (self.thing.location == self.game.quadrant) as i64
            - self.game.tholian.is_some() as i64;
        if n != 0 {
            self.game.shldup = true;
        }
        if self.game.neutz {
            // bad luck to start in a Romulan Neutral Zone
            self.attack(false);
        }
    }

    /// `choose()`: choose the game type. True for a thawed game.
    pub fn choose(&mut self) -> bool {
        self.game.tourn = None;
        self.game.length = 0;
        self.game.sandbox = false;
        self.game.thawed = false;
        self.game.skill = SKILL_NONE;
        let mut gametype: Option<&str> = None;
        let mut wayback: i64 = 0;
        while gametype.is_none() || self.game.length == 0 || self.game.skill == SKILL_NONE || wayback == 0 {
            let mut eol_is_fancy = false;
            // `scanner.token == "IHEOL"` can never hold (tokens are lowercased)
            if self.scanner.inqueue.is_empty() || (!self.scanner.token_none && self.scanner.token == "IHEOL") {
                if gametype.is_none() {
                    self.proutn("Would you like a regular, tournament, or saved game? ");
                } else if self.game.length == 0 {
                    self.proutn("Would you like a Short, Medium, Long, or Sandbox game? ");
                } else if self.game.skill == SKILL_NONE {
                    self.proutn("Are you a Novice, Fair, Good, Expert, or Emeritus player? ");
                } else if wayback == 0 {
                    self.proutn("Wayback setting (press enter for current year): ");
                    eol_is_fancy = true;
                }
            }
            self.nexttok();
            if self.game.idebug {
                let ty = match self.scanner.type_ {
                    Tok::Eol => "IHEOL",
                    Tok::Real => "IHREAL",
                    Tok::Alpha => "IHALPHA",
                    _ => "None",
                };
                let s = pf!("-- Token: %s=%s", ty, str_repr(&self.scanner.token));
                self.prout(&s);
            }
            let tok = self.scanner.token.clone();
            if tok.is_empty() {
                std::panic::resume_unwind(Box::new(SystemExit(0))); // Early end of replay
            }
            if tok.starts_with('r') {
                gametype = Some("regular");
            } else if tok.starts_with('t') {
                gametype = Some("tournament");
                self.proutn("Type in tournament number-");
                self.game.tourn = Some(0);
                while self.nexttok() != Tok::Real {
                    if self.scanner.token.is_empty() {
                        std::panic::resume_unwind(Box::new(SystemExit(0)));
                    }
                    if self.scanner.type_ != Tok::Eol {
                        self.huh();
                    }
                    self.proutn("Type in tournament number-");
                }
                self.game.tourn = Some(self.scanner.int());
                let r = self.scanner.real;
                self.rnd_seed_float(r);
                if let Some(f) = self.io.logfp.as_mut() {
                    use std::io::Write;
                    let _ = write!(f, "{}", pf!("# rnd.seed(%d)\n", r));
                }
            } else if tok.starts_with("sand") {
                gametype = Some("regular");
                self.game.length = 4;
                self.game.sandbox = true;
            } else if tok.starts_with("sa") || tok.starts_with("fr") {
                gametype = Some("saved");
                if self.thaw() {
                    continue;
                }
                self.scanner.chew();
                if self.game.passwd.is_none() {
                    continue;
                }
                if !self.game.alldone {
                    self.game.thawed = true; // No plaque if not finished
                }
                self.report();
                self.waitfor();
                return true;
            } else if tok.starts_with('s') {
                self.game.length = 1;
            } else if tok.starts_with('m') {
                self.game.length = 2;
            } else if tok.starts_with('l') {
                self.game.length = 4;
            } else if tok.starts_with('n') {
                self.game.skill = SKILL_NOVICE;
            } else if tok.starts_with('f') {
                self.game.skill = SKILL_FAIR;
            } else if tok.starts_with('g') {
                self.game.skill = SKILL_GOOD;
            } else if tok.starts_with("em") {
                self.game.skill = SKILL_EMERITUS;
            } else if tok.starts_with('e') {
                self.game.skill = SKILL_EXPERT;
            } else if self.scanner.type_ == Tok::Real {
                wayback = self.scanner.int();
            } else if eol_is_fancy && tok.starts_with('\n') {
                wayback = current_year();
            } else if tok.starts_with('\n') {
                continue;
            } else if tok.starts_with("idebug") {
                self.game.idebug = true;
            } else {
                let s = pf!("Can't interpret %s", str_repr(&tok));
                self.prout(&s);
            }
        }
        // Restore only features suppressed by the preceding game's year.
        let mut opts = self.game.options | self.game.wayback_disabled;
        self.game.wayback_disabled = 0;
        for &(_, option, year) in OPTION_NAMES {
            if wayback < year {
                self.game.wayback_disabled |= opts & option;
                opts &= !option;
            }
        }
        self.set_options(opts);
        self.setpassword();
        if self.game.passwd.as_deref() == Some("debug") {
            self.game.idebug = true;
            self.prout("=== Debug mode enabled.");
        }
        if self.game.idebug {
            let s = pf!(
                "--- Setup: type=%s length=%s skill=%s wayback=%s",
                gametype.unwrap_or("None"),
                self.game.length,
                self.game.skill,
                wayback
            );
            self.prout(&s);
        }
        // Use parameters to generate initial values of things
        self.game.damfac = 0.5 * self.game.skill as f64;
        self.game.inbase = self.rnd_integer2(BASEMIN as f64, (BASEMAX + 1) as f64);
        self.game.inplan = 0;
        if self.game.options & OPTION_PLANETS != 0 {
            self.game.inplan += self.rnd_integer2(MAXUNINHAB as f64 / 2.0, (MAXUNINHAB + 1) as f64);
        }
        if self.game.options & OPTION_WORLDS != 0 {
            self.game.inplan += NINHAB;
        }
        let r = self.rnd_integer(2.0 * self.game.skill as f64);
        self.game.state.nromrem = r;
        self.game.inrom = r;
        let sc = (self.game.skill > SKILL_FAIR && self.game.options & OPTION_SUPERCMDR != 0) as i64;
        self.game.state.nscrem = sc;
        self.game.inscom = sc;
        self.game.state.remtime = 7.0 * self.game.length as f64;
        self.game.intime = self.game.state.remtime;
        let r = self.rnd_real();
        self.game.inkling =
            trunc(2.0 * self.game.intime * ((self.game.skill as f64 + 1.0 - 2.0 * r) * self.game.skill as f64 * 0.1 + 0.15));
        let r = self.rnd_real();
        self.game.incom = MAXCMDR.min(trunc(self.game.skill as f64 + 0.0625 * self.game.inkling as f64 * r));
        let initial_klingons = self.game.inkling + self.game.incom + self.game.inscom;
        self.game.state.remres = (initial_klingons + 4 * self.game.incom) as f64 * self.game.intime;
        self.game.inresor = self.game.state.remres;
        if self.game.inkling > 50 {
            self.game.inbase += 1;
        }
        if self.game.sandbox {
            self.game.state.remtime = 9999.0;
            self.game.state.remres = 9999999.0;
            self.game.inresor = self.game.state.remres;
        }
        false
    }

    /// `dropin(iquad)`: drop a feature on a random dot in the quadrant.
    pub fn dropin(&mut self, iquad: Option<char>) -> Coord {
        let mut w;
        loop {
            w = self.randplace(QUADSIZE);
            if self.game.q(w) == '.' {
                break;
            }
        }
        if let Some(c) = iquad {
            self.game.set_q(w, c);
        }
        w
    }

    /// `newcnd()`: update our alert status.
    pub fn newcnd(&mut self) {
        let old = self.game.condition.clone();
        self.game.condition = "green".into();
        if self.game.energy < 1000.0 {
            self.game.condition = "yellow".into();
        }
        let q = self.game.here();
        if q.klingons != 0 || q.romulans != 0 {
            self.game.condition = "red".into();
        }
        if !self.game.alive {
            self.game.condition = "dead".into();
        }
        if old != self.game.condition {
            log::debug!(target: "state", "condition {old} -> {}", self.game.condition);
        }
    }

    /// `newkling()`: drop a new Klingon into the current quadrant.
    pub fn newkling(&mut self) -> EnemyId {
        let loc = self.dropin(None);
        let p = self.rnd_real2(300.0, 450.0) + 25.0 * self.game.skill as f64;
        self.new_enemy('K', Some(loc), p)
    }

    /// `newqad()`: set up a quadrant on entering or re-entering it.
    pub fn newqad(&mut self) {
        log::debug!(target: "state", "entering quadrant {}", self.game.quadrant);
        self.game.justin = true;
        self.game.base = Coord::none();
        self.game.iplnet = None;
        self.game.plnet = Coord::none();
        self.game.neutz = false;
        self.game.inorbit = false;
        self.game.landed = false;
        self.game.ientesc = false;
        self.game.iseenit = false;
        self.game.isviolreported = false;
        self.game.tholian = None;
        self.game.quad = [['.'; QS]; QS];
        if self.game.iscate {
            // Attempt to escape Super-commander, so tbeam back!
            self.game.iscate = false;
            self.game.ientesc = true;
        }
        let qc = self.game.quadrant;
        if self.game.state.gal(qc).supernova {
            self.game.state.page_mut(qc).supernova = true;
            return;
        }
        let (qk, qr, qstars, qbase, qplanet) = {
            let q = self.game.state.gal(qc);
            (q.klingons, q.romulans, q.stars, q.starbase, q.planet)
        };
        self.game.klhere = qk;
        self.game.irhere = qr;
        // Position Starship
        let ship = self.game.ship;
        let sec = self.game.sector;
        self.game.set_q(sec, ship);
        self.game.enemies = Vec::new();
        if qk != 0 {
            for _ in 0..self.game.klhere {
                self.newkling();
            }
            // If we need a commander, promote a Klingon
            if self.game.state.kcmdr.contains(&qc) {
                if self.game.klhere > 0 && !self.game.enemies.is_empty() {
                    let e = self.game.enemies[(self.game.klhere - 1) as usize];
                    let l = self.game.pool[e].location;
                    self.game.set_q(l, 'C');
                    self.game.pool[e].etype = 'C';
                    let p = self.rnd_real2(950.0, 1350.0) + 50.0 * self.game.skill as f64;
                    self.game.pool[e].power = p;
                }
            }
            // If we need a super-commander, promote a Klingon
            if self.game.state.nscrem > 0 && qc == self.game.state.kscmdr {
                let e = self.game.enemies[0];
                let l = self.game.pool[e].location;
                self.game.set_q(l, 'S');
                self.game.pool[e].etype = 'S';
                let p = self.rnd_real2(1175.0, 1575.0) + 125.0 * self.game.skill as f64;
                self.game.pool[e].power = p;
                self.game.iscate = self.game.remkl() > 1;
            }
        }
        // Put in Romulans if needed
        for _ in 0..qr {
            let loc = self.dropin(None);
            let p = self.rnd_real2(400.0, 850.0) + 50.0 * self.game.skill as f64;
            self.new_enemy('R', Some(loc), p);
        }
        // If quadrant needs a starbase, put it in
        if qbase {
            self.game.base = self.dropin(Some('B'));
        }
        // If quadrant needs a planet, put it in
        if let Some(k) = qplanet {
            self.game.iplnet = Some(k);
            if !self.game.state.planets[k].inhabited {
                self.game.plnet = self.dropin(Some('P'));
            } else {
                self.game.plnet = self.dropin(Some('@'));
            }
        }
        // Check for condition
        self.newcnd();
        // Check for RNZ
        if self.game.irhere > 0 && self.game.klhere == 0 {
            self.game.neutz = true;
            if !self.damaged(DRADIO) {
                self.skip(1);
                self.prout("LT. Uhura- \"Captain, an urgent message.");
                self.prout("  I'll put it on audio.\"  CLICK");
                self.skip(1);
                self.prout("INTRUDER! YOU HAVE VIOLATED THE ROMULAN NEUTRAL ZONE.");
                self.prout("LEAVE AT ONCE, OR YOU WILL BE DESTROYED!");
            }
        }
        // Put in THING if needed
        if self.thing.location == self.game.quadrant {
            let loc = self.dropin(None);
            let p = self.rnd_real2(6000.0, 6500.0) + 250.0 * self.game.skill as f64;
            self.new_enemy('?', Some(loc), p);
            if !self.damaged(DSRSENS) {
                self.skip(1);
                self.prout("Mr. Spock- \"Captain, this is most unusual.");
                self.prout("    Please examine your short-range scan.\"");
            }
        }
        // Decide if quadrant needs a Tholian; lighten up if skill is low
        if self.game.options & OPTION_THOLIAN != 0 {
            let skill = self.game.skill;
            let t = (skill < SKILL_GOOD && self.withprob(0.02))
                || (skill == SKILL_GOOD && self.withprob(0.05))
                || (skill > SKILL_GOOD && self.withprob(0.08));
            if t {
                let mut w = Coord::new(0, 0);
                loop {
                    w.i = self.withprob(0.5) as i64 * (QUADSIZE - 1);
                    w.j = self.withprob(0.5) as i64 * (QUADSIZE - 1);
                    if self.game.q(w) == '.' {
                        break;
                    }
                }
                let p = self.rnd_integer2(100.0, 500.0) as f64 + 25.0 * self.game.skill as f64;
                self.game.tholian = Some(self.new_enemy('T', Some(w), p));
                // Reserve unoccupied corners
                for (a, b) in corners() {
                    if self.game.quad[a][b] == '.' {
                        self.game.quad[a][b] = 'X';
                    }
                }
            }
        }
        self.sortenemies();
        // And finally the stars
        for _ in 0..qstars {
            self.dropin(Some('*'));
        }
        // Put in a few black holes
        for _ in 1..=3 {
            if self.withprob(0.5) {
                self.dropin(Some(' '));
            }
        }
        // Take out X's in corners if Tholian present
        if self.game.tholian.is_some() {
            for (a, b) in corners() {
                if self.game.quad[a][b] == 'X' {
                    self.game.quad[a][b] = '.';
                }
            }
        }
        // This should guarantee that replay games don't lose info about the chart
        if self.game.options & OPTION_AUTOSCAN != 0 || self.io.replay_started {
            self.lrscan(true);
        }
    }

    /// `setpassword()`: set the self-destruct password.
    pub fn setpassword(&mut self) {
        if self.game.options & OPTION_AUTOPASS != 0 {
            let mut p = String::new();
            for _ in 0..3 {
                let k = self.rnd_integer(26.0);
                p.push(char::from(b'a' + k as u8));
            }
            self.game.passwd = Some(p);
        } else {
            loop {
                self.scanner.chew();
                self.proutn("Please type in a secret password- ");
                self.nexttok();
                // Python's token is a str here, never None
                self.game.passwd = Some(self.scanner.token.clone());
                break;
            }
        }
    }

    /// `welcoming(iq)`: would this quadrant welcome another Klingon?
    pub fn welcoming(&self, iq: Coord) -> bool {
        iq.valid_quadrant() && !self.game.state.gal(iq).supernova && self.game.state.gal(iq).klingons < MAXKLQUAD
    }
}

fn corners() -> [(usize, usize); 4] {
    [(0, 0), (0, QS - 1), (QS - 1, 0), (QS - 1, QS - 1)]
}

/// `time.localtime().tm_year` (the one place the game reads the clock).
/// SUPER_TREK_YEAR overrides it for tests.
pub fn current_year() -> i64 {
    if let Ok(y) = std::env::var("SUPER_TREK_YEAR")
        && let Ok(v) = y.parse()
    {
        return v;
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    // civil year from days since epoch (UTC; local time differs only at New Year)
    let days = secs.div_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    if m <= 2 { y + 1 } else { y }
}
