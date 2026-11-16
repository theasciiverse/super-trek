//! Reports and scans: attackreport, report, lrscan, damagereport, rechart, chart, sectscan, status, request, srscan, eta (`sst` 5810-6400).

use crate::consts::*;
use crate::coord::{Coord, letterize};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::Sst;

impl Sst {
    /// `attackreport(curt)`: report status of bases under attack.
    pub fn attackreport(&mut self, curt: bool) {
        if !curt {
            if self.is_scheduled(FCDBAS) {
                let s = pf!("Starbase in Quadrant %s is currently under Commander attack.", self.game.battle);
                self.prout(&s);
                let s = pf!("It can hold out until Stardate %d.", trunc(self.scheduled(FCDBAS)));
                self.prout(&s);
            } else if self.game.isatb == 1 {
                let s = pf!("Starbase in Quadrant %s is under Super-commander attack.", self.game.state.kscmdr);
                self.prout(&s);
                let s = pf!("It can hold out until Stardate %d.", trunc(self.scheduled(FSCDBAS)));
                self.prout(&s);
            } else {
                self.prout("No Starbase is currently under attack.");
            }
        } else {
            if self.is_scheduled(FCDBAS) {
                let s = pf!("Base in %s attacked by C. Alive until %.1f", self.game.battle, self.scheduled(FCDBAS));
                self.proutn(&s);
            }
            if self.game.isatb != 0 {
                let s = pf!("Base in %s attacked by S. Alive until %.1f", self.game.state.kscmdr, self.scheduled(FSCDBAS));
                self.proutn(&s);
            }
        }
    }

    /// `report()`: report on general game status.
    pub fn report(&mut self) {
        self.scanner.chew();
        let s1 = if self.game.thawed { "thawed " } else { "" };
        let s2 = if self.game.sandbox {
            "sandbox"
        } else {
            match self.game.length {
                1 => "short",
                2 => "medium",
                4 => "long",
                // DEVIATION: Python raises KeyError for any other length.
                _ => "",
            }
        };
        let s3: Option<&str> = match self.game.skill {
            1 => Some("novice"),
            2 => Some("fair"),
            3 => Some("good"),
            4 => Some("expert"),
            5 => Some("emeritus"),
            _ => None,
        };
        // Known SST2K bug kept: "were playing" while the game is on.
        let playing = if self.game.alldone { "are playing" } else { "were playing" };
        let s = pf!("You %s a %s%s %s game.", playing, s1, s2, s3);
        self.prout(&s);
        if self.game.skill > SKILL_GOOD && self.game.thawed && !self.game.alldone {
            self.prout("No plaque is allowed.");
        }
        if let Some(t) = self.game.tourn
            && t != 0
        {
            let s = pf!("This is tournament game %d.", t);
            self.prout(&s);
        }
        let s = pf!("Your secret password is \"%s\"", self.game.passwd.clone());
        self.prout(&s);
        let total = self.game.inkling + self.game.incom + self.game.inscom;
        let s = pf!("%d of %d Klingons have been killed", total - self.game.unwon(), total);
        self.proutn(&s);
        let kc = self.game.incom - self.game.state.kcmdr.len() as i64;
        if kc != 0 {
            let s = pf!(", including %d Commander%s.", kc, if kc == 1 { "" } else { "s" });
            self.prout(&s);
        } else if self.game.inkling - self.game.remkl() + (self.game.inscom - self.game.state.nscrem) > 0 {
            self.prout(", but no Commanders.");
        } else {
            self.prout(".");
        }
        if self.game.skill > SKILL_FAIR {
            // ("", "not ")[nscrem]
            let s = pf!("The Super Commander has %sbeen destroyed.", if self.game.state.nscrem != 0 { "not " } else { "" });
            self.prout(&s);
        }
        let nb = self.game.state.baseq.len() as i64;
        if nb != self.game.inbase {
            self.proutn("There ");
            if self.game.inbase - nb == 1 {
                self.proutn("has been 1 base");
            } else {
                let s = pf!("have been %d bases", self.game.inbase - nb);
                self.proutn(&s);
            }
            let s = pf!(" destroyed, %d remaining.", nb);
            self.prout(&s);
        } else {
            let s = pf!("There are %d bases.", self.game.inbase);
            self.prout(&s);
        }
        if self.communicating() || self.game.iseenit {
            // Don't report this if not seen and
            // either the radio is dead or not at base!
            self.attackreport(false);
            self.game.iseenit = true;
        }
        if self.game.casual != 0 {
            let s = pf!("%d casualt%s suffered so far.", self.game.casual, if self.game.casual != 1 { "ies" } else { "y" });
            self.prout(&s);
        }
        if self.game.brigcapacity != self.game.brigfree {
            let embriggened = self.game.brigcapacity - self.game.brigfree;
            if embriggened == 1 {
                self.prout("1 Klingon in brig");
            } else {
                let s = pf!("%d Klingons in brig.", embriggened);
                self.prout(&s);
            }
            if self.game.kcaptured == 0 {
            } else if self.game.kcaptured == 1 {
                self.prout("1 captured Klingon turned in to Starfleet.");
            } else {
                let s = pf!("%d captured Klingons turned in to Star Fleet.", self.game.kcaptured);
                self.prout(&s);
            }
        }
        if self.game.nhelp != 0 {
            let s = pf!("There were %d call%s for help.", self.game.nhelp, if self.game.nhelp != 1 { "s" } else { "" });
            self.prout(&s);
        }
        if self.game.ship == 'E' {
            self.proutn("You have ");
            if self.game.nprobes != 0 {
                let s = pf!("%d", self.game.nprobes);
                self.proutn(&s);
            } else {
                self.proutn("no");
            }
            self.proutn(" deep space probe");
            if self.game.nprobes != 1 {
                self.proutn("s");
            }
            self.prout(".");
        }
        if self.communicating() && self.is_scheduled(FDSPROB) {
            if self.game.isarmed {
                self.proutn("An armed deep space probe is in ");
            } else {
                self.proutn("A deep space probe is in ");
            }
            // DEVIATION: Python raises AttributeError if game.probe is None
            // while FDSPROB is scheduled; here it prints "Nowhere".
            let pq = self.game.probe.as_ref().map(|p| p.quadrant()).unwrap_or_default();
            let s = pf!("Quadrant %s.", pq);
            self.prout(&s);
        }
        if self.game.icrystl {
            if self.game.cryprob <= 0.05 {
                self.prout("Dilithium crystals aboard ship... not yet used.");
            } else {
                let mut i: i64 = 0;
                let mut ai = 0.05;
                while self.game.cryprob > ai {
                    ai *= 2.0;
                    i += 1;
                }
                let s = pf!("Dilithium crystals have been used %d time%s.", i, if i == 1 { "" } else { "s" });
                self.prout(&s);
            }
        }
        self.skip(1);
    }

    /// `lrscan(silent)`: long-range sensor scan.
    pub fn lrscan(&mut self, silent: bool) {
        if self.damaged(DLRSENS) {
            // Now allow base's sensors if docked
            if self.game.condition != "docked" {
                if !silent {
                    self.prout("LONG-RANGE SENSORS DAMAGED.");
                }
                return;
            }
            if !silent {
                self.prout("Starbase's long-range scan");
            }
        } else if !silent {
            self.prout("Long-range scan");
        }
        for x in (self.game.quadrant.i - 1)..(self.game.quadrant.i + 2) {
            if !silent {
                self.proutn(" ");
            }
            for y in (self.game.quadrant.j - 1)..(self.game.quadrant.j + 2) {
                if !Coord::new(x, y).valid_quadrant() {
                    if !silent {
                        self.proutn("  -1");
                    }
                } else {
                    let (xu, yu) = (x as usize, y as usize);
                    if !self.damaged(DRADIO) {
                        self.game.state.galaxy[xu][yu].charted = true;
                    }
                    let g = self.game.state.galaxy[xu][yu].clone();
                    let page = &mut self.game.state.chart[xu][yu];
                    page.klingons = Some(g.klingons);
                    page.starbase = g.starbase;
                    page.stars = Some(g.stars);
                    page.supernova = g.supernova;
                    if !silent && g.supernova {
                        self.proutn(" ***");
                    } else if !silent {
                        let cn = pf!(" %3d", g.klingons * 100 + g.starbase as i64 * 10 + g.stars);
                        // ((3 - len(cn)) * ".") + cn: cn is always 4+ long
                        let dots = ".".repeat(3usize.saturating_sub(cn.chars().count()));
                        let s = dots + &cn;
                        self.proutn(&s);
                    }
                }
            }
            if !silent {
                self.prout(" ");
            }
        }
    }

    /// `damagereport()`
    pub fn damagereport(&mut self) {
        let mut jdam = false;
        self.scanner.chew();
        for i in 0..NDEVICES {
            if self.damaged(i) {
                if !jdam {
                    self.prout("DEVICE                             REPAIR TIMES");
                    self.prout("                                   IN FLIGHT                DOCKED");
                    jdam = true;
                }
                let s = pf!(
                    "  %-26s\t%8.2f\t\t%8.2f",
                    DEVICE[i],
                    self.game.damage[i] + 0.05,
                    DOCKFAC * self.game.damage[i] + 0.005
                );
                self.prout(&s);
            }
        }
        if !jdam {
            self.prout("All devices functional.");
        }
    }

    /// `rechart()`: update the chart from galaxy data.
    pub fn rechart(&mut self) {
        self.game.lastchart = self.game.state.date;
        for i in 0..GS {
            for j in 0..GS {
                // Starfleet reports supernovae even in unexplored quadrants.
                let g = self.game.state.galaxy[i][j].clone();
                let page = &mut self.game.state.chart[i][j];
                page.supernova = g.supernova;
                if g.charted {
                    page.klingons = Some(g.klingons);
                    page.starbase = g.starbase;
                    page.stars = Some(g.stars);
                }
            }
        }
    }

    /// `chart()`: display the star chart.
    pub fn chart(&mut self) {
        self.scanner.chew();
        if self.game.options & OPTION_AUTOSCAN != 0 {
            self.lrscan(true);
        }
        if self.communicating() {
            self.rechart();
        }
        if self.game.lastchart < self.game.state.date && self.game.condition == "docked" {
            self.prout("Spock-  \"I revised the Star Chart from the starbase's records.\"");
            self.rechart();
        }
        self.prout("       STAR CHART FOR THE KNOWN GALAXY");
        if self.game.state.date > self.game.lastchart {
            let s = pf!("(Last surveillance update %d stardates ago).", trunc(self.game.state.date - self.game.lastchart));
            self.prout(&s);
        }
        self.prout("      1    2    3    4    5    6    7    8");
        for i in 0..GALSIZE {
            if self.game.options & OPTION_ALPHAMERIC != 0 {
                let s = pf!("%c |", letterize(i + 1));
                self.proutn(&s);
            } else {
                let s = pf!("%d |", i + 1);
                self.proutn(&s);
            }
            for j in 0..GALSIZE {
                let me = self.game.options & OPTION_SHOWME != 0 && i == self.game.quadrant.i && j == self.game.quadrant.j;
                self.proutn(if me { "<" } else { " " });
                let page = &self.game.state.chart[i as usize][j as usize];
                let charted = self.game.state.galaxy[i as usize][j as usize].charted;
                let show = if page.supernova {
                    "***".to_string()
                } else if !charted && page.starbase {
                    ".1.".to_string()
                } else if charted {
                    // DEVIATION: Python would raise TypeError on a charted
                    // quadrant whose chart page was never filled (None); 0 here.
                    let mut show =
                        pf!("%3d", page.klingons.unwrap_or(0) * 100 + page.starbase as i64 * 10 + page.stars.unwrap_or(0));
                    if self.game.options & OPTION_DOTFILL != 0 {
                        show = show.replace(' ', ".");
                    }
                    show
                } else {
                    "...".to_string()
                };
                self.proutn(&show);
                self.proutn(if me { ">" } else { " " });
            }
            self.proutn("  |");
            if i < GALSIZE {
                self.skip(1);
            }
        }
    }

    /// `sectscan(goodScan, i, j)`: light up an individual dot in a sector.
    pub fn sectscan(&mut self, good_scan: bool, i: i64, j: i64) {
        if good_scan || ((i - self.game.sector.i).abs() <= 1 && (j - self.game.sector.j).abs() <= 1) {
            // the textcolor/highvideo calls are no-ops on a teletype
            let s = pf!("%c ", self.game.quad[i as usize][j as usize]);
            self.proutn(&s);
        } else {
            self.proutn("- ");
        }
    }

    /// `status(req)`: emit status report lines.
    pub fn status(&mut self, req: i64) {
        if req == 0 || req == 1 {
            if req == 1 {
                let s = pf!("%.1f", self.game.state.date);
                self.prstat("Stardate", &s);
            } else if self.game.sandbox {
                let s = pf!("%.1f, Time Left: Unlimited", self.game.state.date);
                self.prstat("Stardate", &s);
            } else {
                let s = pf!("%.1f, Time Left %.2f", self.game.state.date, self.game.state.remtime);
                self.prstat("Stardate", &s);
            }
        }
        if req == 0 || req == 2 {
            if self.game.condition != "docked" {
                self.newcnd();
            }
            let ndam = self.game.damage.iter().filter(|&&x| x > 0.0).count() as i64;
            let s = pf!("%s, %i DAMAGES", self.game.condition.to_uppercase(), ndam);
            self.prstat("Condition", &s);
            if self.game.iscloaked {
                self.prout(", CLOAKED");
            }
        }
        if req == 0 || req == 3 {
            let s = pf!("%s , %s", self.game.quadrant, self.game.sector);
            self.prstat("Position", &s);
        }
        if req == 0 || req == 4 {
            let s = if self.damaged(DLIFSUP) {
                if self.game.condition == "docked" {
                    "DAMAGED, Base provides".to_string()
                } else {
                    pf!("DAMAGED, reserves=%4.2f", self.game.lsupres)
                }
            } else {
                "ACTIVE".to_string()
            };
            self.prstat("Life Support", &s);
        }
        if req == 0 || req == 5 {
            let s = pf!("%.1f", self.game.warpfac);
            self.prstat("Warp Factor", &s);
        }
        if req == 0 || req == 6 {
            let mut extra = "";
            if self.game.icrystl && (self.game.options & OPTION_SHOWME) != 0 {
                extra = " (have crystals)";
            }
            let s = pf!("%.2f%s", self.game.energy, extra);
            self.prstat("Energy", &s);
        }
        if req == 0 || req == 7 {
            let s = pf!("%d", self.game.torps);
            self.prstat("Torpedoes", &s);
        }
        if req == 0 || req == 8 {
            let s = if self.damaged(DSHIELD) {
                "DAMAGED,"
            } else if self.game.shldup {
                "UP,"
            } else {
                "DOWN,"
            };
            let data = pf!(" %d%% %.1f units", trunc((100.0 * self.game.shield) / self.game.inshld + 0.5), self.game.shield);
            let s = s.to_string() + &data;
            self.prstat("Shields", &s);
        }
        if req == 0 || req == 9 {
            let s = pf!("%d", self.game.unwon());
            self.prstat("Klingons Left", &s);
        }
        if req == 0 || req == 10 {
            if self.game.options & OPTION_WORLDS != 0 {
                let plnet = self.game.here().planet;
                match plnet.map(|k| self.game.state.planets[k].clone()) {
                    Some(p) if p.inhabited => {
                        let name = pf!("%s", p.name.clone());
                        self.prstat("Major system", &name);
                    }
                    _ => self.prout("Sector is uninhabited"),
                }
            }
        } else if req == 0 || req == 11 {
            self.attackreport(req == 0);
        }
        // Separate REQUEST fields without adding rows to the screen status panel.
        if req == 12 {
            let s = if self.game.sandbox { "Unlimited".to_string() } else { pf!("%.2f", self.game.state.remtime) };
            self.prstat("Time Left", &s);
        }
        if req == 13 {
            let s = pf!("%d", self.game.state.baseq.len() as i64);
            self.prstat("Bases Left", &s);
        }
    }

    /// `request()`: request specified status data.
    pub fn request(&mut self) {
        // Full item name, shortest abbreviation, and status field number.
        const REQUESTS: [(&str, &str, i64); 11] = [
            ("date", "d", 1),
            ("condition", "c", 2),
            ("position", "p", 3),
            ("lsupport", "l", 4),
            ("warpfactor", "w", 5),
            ("energy", "e", 6),
            ("torpedoes", "t", 7),
            ("shields", "s", 8),
            ("klingons", "k", 9),
            ("time", "ti", 12),
            ("bases", "ba", 13),
        ];
        while self.nexttok() == Tok::Eol {
            self.proutn("Information desired? ");
        }
        let item = self.scanner.token.clone();
        self.scanner.chew();
        for (name, shortest, field) in REQUESTS {
            if !item.is_empty() && name.starts_with(item.as_str()) && item.starts_with(shortest) {
                self.status(field);
                return;
            }
        }
        self.prout("UNRECOGNIZED REQUEST. Legal requests are:");
        self.prout("  date, condition, position, lsupport, warpfactor,");
        self.prout("  energy, torpedoes, shields, klingons, time, bases.");
    }

    /// `srscan()`: short-range scan.
    pub fn srscan(&mut self) {
        let mut good_scan = true;
        if self.damaged(DSRSENS) {
            // Allow base's sensors if docked
            if self.game.condition != "docked" {
                self.prout("   S.R. SENSORS DAMAGED!");
                good_scan = false;
            } else {
                self.prout("  [Using Base's sensors]");
            }
        } else {
            self.prout("     Short-range scan");
        }
        if good_scan && self.communicating() {
            let q = self.game.quadrant;
            let g = self.game.state.gal(q).clone();
            let page = self.game.state.page_mut(q);
            page.klingons = Some(g.klingons);
            page.starbase = g.starbase;
            page.stars = Some(g.stars);
            page.supernova = g.supernova;
            self.game.state.gal_mut(q).charted = true;
        }
        self.prout("    1 2 3 4 5 6 7 8 9 10");
        if self.game.condition != "docked" {
            self.newcnd();
        }
        for i in 0..QUADSIZE {
            if self.game.options & OPTION_ALPHAMERIC != 0 {
                let s = pf!("%c   ", letterize(i + 1));
                self.proutn(&s);
            } else {
                let s = pf!("%2d  ", i + 1);
                self.proutn(&s);
            }
            for j in 0..QUADSIZE {
                self.sectscan(good_scan, i, j);
            }
            self.skip(1);
        }
    }

    /// `eta()`: use the computer to get the time of arrival for a warp jump.
    pub fn eta(&mut self) {
        let mut w1 = Coord::none();
        let mut w2 = Coord::none();
        let mut prompt = false;
        if self.damaged(DCOMPTR) {
            self.prout("COMPUTER DAMAGED, USE A POCKET CALCULATOR.");
            self.skip(1);
            return;
        }
        if self.nexttok() != Tok::Real {
            prompt = true;
            self.scanner.chew();
            self.proutn("Destination quadrant and/or sector? ");
            if self.nexttok() != Tok::Real {
                self.huh();
                return;
            }
        }
        w1.j = trunc(self.scanner.real - 0.5);
        if self.nexttok() != Tok::Real {
            self.huh();
            return;
        }
        w1.i = trunc(self.scanner.real - 0.5);
        if self.nexttok() == Tok::Real {
            w2.j = trunc(self.scanner.real - 0.5);
            if self.nexttok() != Tok::Real {
                self.huh();
                return;
            }
            w2.i = trunc(self.scanner.real - 0.5);
        } else {
            if self.game.quadrant.j > w1.i {
                w2.i = 0;
            } else {
                w2.i = QUADSIZE - 1;
            }
            if self.game.quadrant.i > w1.j {
                w2.j = 0;
            } else {
                w2.j = QUADSIZE - 1;
            }
        }
        if !w1.valid_quadrant() || !w2.valid_sector() {
            self.huh();
            return;
        }
        let q = self.game.quadrant;
        let sct = self.game.sector;
        let dist = (pypow((w1.j - q.j) as f64 + (w2.j - sct.j) as f64 / (QUADSIZE as f64 * 1.0), 2.0)
            + pypow((w1.i - q.i) as f64 + (w2.i - sct.i) as f64 / (QUADSIZE as f64 * 1.0), 2.0))
        .sqrt();
        let mut wfl = false;
        if prompt {
            self.prout("Answer \"no\" if you don't know the value:");
        }
        let mut twarp: f64;
        loop {
            self.scanner.chew();
            self.proutn("Time or arrival date? ");
            if self.nexttok() == Tok::Real {
                let mut ttime = self.scanner.real;
                if ttime > self.game.state.date {
                    ttime -= self.game.state.date; // Actually a star date
                }
                if ttime <= 1e-10 {
                    self.prout("We'll never make it, sir.");
                    self.scanner.chew();
                    return;
                }
                twarp = ((((10.0 * dist) / ttime).sqrt() * 10.0).floor() + 1.0) / 10.0;
                if twarp > 10.0 {
                    self.prout("We'll never make it, sir.");
                    self.scanner.chew();
                    return;
                }
                // max(twarp, 1.0) returns twarp unless 1.0 is greater
                if 1.0 > twarp {
                    twarp = 1.0;
                }
                break;
            }
            self.scanner.chew();
            self.proutn("Warp factor? ");
            if self.nexttok() == Tok::Real {
                wfl = true;
                twarp = self.scanner.real;
                if !(1.0..=10.0).contains(&twarp) {
                    self.huh();
                    return;
                }
                break;
            }
            self.prout("Captain, certainly you can give me one of these.");
        }
        loop {
            self.scanner.chew();
            let ttime = (10.0 * dist) / pypow(twarp, 2.0);
            let mut tpower = dist * twarp * twarp * twarp * (self.game.shldup as i64 + 1) as f64;
            if tpower >= self.game.energy {
                self.prout("Insufficient energy, sir.");
                if !self.game.shldup || tpower > self.game.energy * 2.0 {
                    if !wfl {
                        return;
                    }
                    self.proutn("New warp factor to try? ");
                    if self.nexttok() == Tok::Real {
                        wfl = true;
                        twarp = self.scanner.real;
                        if !(1.0..=10.0).contains(&twarp) {
                            self.huh();
                            return;
                        }
                        continue;
                    } else {
                        self.scanner.chew();
                        self.skip(1);
                        return;
                    }
                }
                self.prout("But if you lower your shields,");
                self.proutn("remaining");
                tpower /= 2.0;
            } else {
                self.proutn("Remaining");
            }
            let s = pf!(" energy will be %.2f.", self.game.energy - tpower);
            self.prout(&s);
            if wfl {
                let s = pf!("And we will arrive at stardate %.2f.", self.game.state.date + ttime);
                self.prout(&s);
            } else if twarp == 1.0 {
                self.prout("Any warp speed is adequate.");
            } else {
                let s = pf!("Minimum warp needed is %.2f,", twarp);
                self.prout(&s);
                let s = pf!("and we will arrive at stardate %.2f.", self.game.state.date + ttime);
                self.prout(&s);
            }
            if !self.game.sandbox && self.game.state.remtime < ttime {
                self.prout("Unfortunately, the Federation will be destroyed by then.");
            }
            if twarp > 6.0 {
                self.prout("You'll be taking risks at that speed, Captain");
            }
            if (self.game.isatb == 1
                && self.game.state.kscmdr == w1
                && self.scheduled(FSCDBAS) < ttime + self.game.state.date)
                || (self.scheduled(FCDBAS) < ttime + self.game.state.date && self.game.battle == w1)
            {
                self.prout("The starbase there will be destroyed by then.");
            }
            self.proutn("New warp factor to try? ");
            if self.nexttok() == Tok::Real {
                wfl = true;
                twarp = self.scanner.real;
                if !(1.0..=10.0).contains(&twarp) {
                    self.huh();
                    return;
                }
            } else {
                self.scanner.chew();
                self.skip(1);
                return;
            }
        }
    }
}
