//! Endings and scoring: selfdestruct, kaboom, killrate, badpoints, finish, score, plaque (`sst` 3362-3888).

use crate::consts::*;
use crate::pf;
use crate::sst::Sst;
use std::io::Write as _;
use std::path::Path;

/// `game.ship = None` (the ship after the Faerie Queene is lost too).
/// `'\0'` stays the initial `0`, which Python's `is None` does not match.
pub const SHIP_NONE: char = '-';

impl Sst {
    /// `selfdestruct()`: self-destruct maneuver. Finish with a BANG!
    pub fn selfdestruct(&mut self) {
        self.scanner.chew();
        if self.damaged(DCOMPTR) {
            self.prout("Computer damaged; cannot execute destruct sequence.");
            return;
        }
        self.prouts("---WORKING---");
        self.skip(1);
        self.prouts("SELF-DESTRUCT-SEQUENCE-ACTIVATED");
        self.skip(1);
        self.prouts("   10");
        self.skip(1);
        self.prouts("       9");
        self.skip(1);
        self.prouts("          8");
        self.skip(1);
        self.prouts("             7");
        self.skip(1);
        self.prouts("                6");
        self.skip(1);
        self.skip(1);
        self.prout("ENTER-CORRECT-PASSWORD-TO-CONTINUE-");
        self.skip(1);
        self.prout("SELF-DESTRUCT-SEQUENCE-OTHERWISE-");
        self.skip(1);
        self.prout("SELF-DESTRUCT-SEQUENCE-WILL-BE-ABORTED");
        self.skip(1);
        self.nexttok();
        // `game.passwd != scanner.token`; the token is a str after nexttok
        if self.game.passwd.as_deref() != Some(self.scanner.token.as_str()) {
            self.prouts("PASSWORD-REJECTED;");
            self.skip(1);
            self.prouts("CONTINUITY-EFFECTED");
            self.skip(2);
            return;
        }
        self.prouts("PASSWORD-ACCEPTED");
        self.skip(1);
        self.prouts("                   5");
        self.skip(1);
        self.prouts("                      4");
        self.skip(1);
        self.prouts("                         3");
        self.skip(1);
        self.prouts("                            2");
        self.skip(1);
        self.prouts("                              1");
        self.skip(1);
        if self.withprob(0.15) {
            self.prouts("GOODBYE-CRUEL-WORLD");
            self.skip(1);
        }
        self.kaboom();
    }

    /// `kaboom()`
    pub fn kaboom(&mut self) {
        self.stars();
        if self.game.ship == 'E' {
            self.prouts("***");
        }
        let s = pf!("********* Entropy of %s maximized *********", self.crmshp());
        self.prouts(&s);
        self.skip(1);
        self.stars();
        self.skip(1);
        if !self.game.enemies.is_empty() {
            let whammo = 25.0 * self.game.energy;
            // `game.enemies[::-1]` iterates a reversed copy of the list
            let ids: Vec<_> = self.game.enemies.iter().rev().copied().collect();
            for e in ids {
                let en = &self.game.pool[e];
                if en.power * en.kdist <= whammo {
                    let loc = en.location;
                    let etype = self.game.q(loc);
                    self.deadkl(loc, etype, loc);
                }
            }
        }
        self.finish(FDILITHIUM);
    }

    /// `killrate()`: rate of kills over time (Python returns the int 0 on
    /// turn 0; every caller treats it as a number, so 0.0 here).
    pub fn killrate(&mut self) -> f64 {
        let elapsed = self.game.state.date - self.game.indate;
        if elapsed == 0.0 {
            0.0
        } else {
            let starting = self.game.inkling + self.game.incom + self.game.inscom;
            let remaining = self.game.unwon();
            (starting - remaining) as f64 / elapsed
        }
    }

    /// `badpoints()`: compute demerits.
    pub fn badpoints(&mut self) -> f64 {
        let g = &self.game;
        let mut badpt = 5.0 * g.state.starkl as f64
            + g.casual as f64
            + 10.0 * g.state.nplankl as f64
            + (300 * g.state.nworldkl) as f64
            + 45.0 * g.nhelp as f64
            + 100.0 * g.state.basekl as f64
            + 3.0 * g.abandoned as f64
            + (100 * g.ncviol) as f64;
        if g.ship == 'F' {
            badpt += 100.0;
        } else if g.ship == SHIP_NONE {
            badpt += 200.0;
        }
        badpt
    }

    /// `finish(ifin)`: end the game, with appropriate notifications.
    pub fn finish(&mut self, ifin: i64) {
        let mut igotit = false;
        self.game.alldone = true;
        self.skip(3);
        let s = pf!("It is stardate %.1f.", self.game.state.date);
        self.prout(&s);
        self.skip(1);
        if ifin == FWON {
            // Game has been won
            if self.game.state.nromrem != 0 {
                let s = pf!("The remaining %d Romulans surrender to Starfleet Command.", self.game.state.nromrem);
                self.prout(&s);
            }
            self.prout("You have smashed the Klingon invasion fleet and saved");
            self.prout("the Federation.");
            if self.game.alive && self.game.brigcapacity - self.game.brigfree > 0 {
                self.game.kcaptured += self.game.brigcapacity - self.game.brigfree;
                let s = pf!(
                    "The %d captured Klingons are transferred to Star Fleet Command.",
                    self.game.brigcapacity - self.game.brigfree
                );
                self.prout(&s);
            }
            self.game.gamewon = true;
            if self.game.alive {
                let mut badpt = self.badpoints();
                if badpt < 100.0 {
                    badpt = 0.0; // Close enough!
                }
                // killsPerDate >= RateMax
                let skill = self.game.skill as f64;
                if self.game.state.date - self.game.indate < 5.0
                    || self.killrate() >= 0.1 * skill * (skill + 1.0) + 0.1 + 0.008 * badpt
                {
                    self.skip(1);
                    self.prout("In fact, you have done so well that Starfleet Command");
                    match self.game.skill {
                        SKILL_NOVICE => self.prout("promotes you one step in rank from \"Novice\" to \"Fair\"."),
                        SKILL_FAIR => self.prout("promotes you one step in rank from \"Fair\" to \"Good\"."),
                        SKILL_GOOD => self.prout("promotes you one step in rank from \"Good\" to \"Expert\"."),
                        SKILL_EXPERT => {
                            self.prout("promotes you to Commodore Emeritus.");
                            self.skip(1);
                            self.prout("Now that you think you're really good, try playing");
                            self.prout("the \"Emeritus\" game. It will splatter your ego.");
                        }
                        SKILL_EMERITUS => {
                            self.skip(1);
                            self.proutn("Computer-  ");
                            self.prouts("ERROR-ERROR-ERROR-ERROR");
                            self.skip(2);
                            self.prouts("  YOUR-SKILL-HAS-EXCEEDED-THE-CAPACITY-OF-THIS-PROGRAM");
                            self.skip(1);
                            self.prouts("  THIS-PROGRAM-MUST-SURVIVE");
                            self.skip(1);
                            self.prouts("  THIS-PROGRAM-MUST-SURVIVE");
                            self.skip(1);
                            self.prouts("  THIS-PROGRAM-MUST-SURVIVE");
                            self.skip(1);
                            self.prouts("  THIS-PROGRAM-MUST?- MUST ? - SUR? ? -?  VI");
                            self.skip(2);
                            self.prout("Now you can retire and write your own Star Trek game!");
                            self.skip(1);
                        }
                        _ => {}
                    }
                    if self.game.skill >= SKILL_EXPERT {
                        if self.game.thawed && !self.game.idebug {
                            self.prout("You cannot get a citation, so...");
                        } else {
                            self.proutn("Do you want your Commodore Emeritus Citation printed? ");
                            self.scanner.chew();
                            if self.ja() {
                                igotit = true;
                            }
                        }
                    }
                }
                // Only grant long life if alive (original didn't!)
                self.skip(1);
                self.prout("LIVE LONG AND PROSPER.");
            }
            self.score();
            if igotit {
                self.plaque();
            }
            return;
        } else if ifin == FDEPLETE {
            // Federation Resources Depleted
            self.prout("Your time has run out and the Federation has been");
            self.prout("conquered.  Your starship is now Klingon property,");
            self.prout("and you are put on trial as a war criminal.  On the");
            self.proutn("basis of your record, you are ");
            if self.game.unwon() as f64 * 3.0 > (self.game.inkling + self.game.incom + self.game.inscom) as f64 {
                self.prout("acquitted.");
                self.skip(1);
                self.prout("LIVE LONG AND PROSPER.");
            } else {
                self.prout("found guilty and");
                self.prout("sentenced to death by slow torture.");
                self.game.alive = false;
            }
            self.score();
            return;
        } else if ifin == FLIFESUP {
            self.prout("Your life support reserves have run out, and");
            self.prout("you die of thirst, starvation, and asphyxiation.");
            self.prout("Your starship is a derelict in space.");
        } else if ifin == FNRG {
            self.prout("Your energy supply is exhausted.");
            self.skip(1);
            self.prout("Your starship is a derelict in space.");
        } else if ifin == FBATTLE {
            let s = pf!("The %s has been destroyed in battle.", self.crmshp());
            self.prout(&s);
            self.skip(1);
            self.prout("Dulce et decorum est pro patria mori.");
        } else if ifin == FNEG3 {
            self.prout("You have made three attempts to cross the negative energy");
            self.prout("barrier which surrounds the galaxy.");
            self.skip(1);
            self.prout("Your navigation is abominable.");
            self.score();
        } else if ifin == FNOVA {
            self.prout("Your starship has been destroyed by a nova.");
            self.prout("That was a great shot.");
            self.skip(1);
        } else if ifin == FSNOVAED {
            let s = pf!("The %s has been fried by a supernova.", self.crmshp());
            self.prout(&s);
            self.prout("...Not even cinders remain...");
        } else if ifin == FABANDN {
            self.prout("You have been captured by the Klingons. If you still");
            self.prout("had a starbase to be returned to, you would have been");
            self.prout("repatriated and given another chance. Since you have");
            self.prout("no starbases, you will be mercilessly tortured to death.");
        } else if ifin == FDILITHIUM {
            self.prout("Your starship is now an expanding cloud of subatomic particles");
        } else if ifin == FMATERIALIZE {
            self.prout("Starbase was unable to re-materialize your starship.");
            self.prout("Sic transit gloria mundi.");
        } else if ifin == FPHASER {
            let s = pf!("The %s has been cremated by its own phasers.", self.crmshp());
            self.prout(&s);
        } else if ifin == FLOST {
            self.prout("You and your landing party have been");
            self.prout("converted to energy, dissipating through space.");
        } else if ifin == FMINING {
            // This does not seem to be reachable from any code path.
            self.prout("You are left with your landing party on");
            self.prout("a wild jungle planet inhabited by primitive cannibals.");
            self.skip(1);
            self.prout("They are very fond of \"Captain Kirk\" soup.");
            self.skip(1);
            let s = pf!("Without your leadership, the %s is destroyed.", self.crmshp());
            self.prout(&s);
        } else if ifin == FDPLANET {
            self.prout("You and your mining party perish.");
            self.skip(1);
            self.prout("That was a great shot.");
            self.skip(1);
        } else if ifin == FSSC {
            // This does not seem to be reachable from any code path.
            self.prout("The Galileo is instantly annihilated by the supernova.");
            self.prout("You and your mining party are atomized.");
            self.skip(1);
            let s = pf!("Mr. Spock takes command of the %s and", self.crmshp());
            self.prout(&s);
            self.prout("joins the Romulans, wreaking terror on the Federation.");
        } else if ifin == FPNOVA {
            self.prout("You and your mining party are atomized.");
            self.skip(1);
            let s = pf!("Mr. Spock takes command of the %s and", self.crmshp());
            self.prout(&s);
            self.prout("joins the Romulans, wreaking terror on the Federation.");
        } else if ifin == FSTRACTOR {
            self.prout("The shuttle craft Galileo is also caught,");
            self.prout("and breaks up under the strain.");
            self.skip(1);
            self.prout("Your debris is scattered for millions of miles.");
            let s = pf!("Without your leadership, the %s is destroyed.", self.crmshp());
            self.prout(&s);
        } else if ifin == FDRAY {
            self.prout("The mutants attack and kill Spock.");
            self.prout("Your ship is captured by Klingons, and");
            self.prout("your crew is put on display in a Klingon zoo.");
        } else if ifin == FTRIBBLE {
            self.prout("Tribbles consume all remaining water,");
            self.prout("food, and oxygen on your ship.");
            self.skip(1);
            self.prout("You die of thirst, starvation, and asphyxiation.");
            self.prout("Your starship is a derelict in space.");
        } else if ifin == FHOLE {
            self.prout("Your ship is drawn to the center of the black hole.");
            self.prout("You are crushed into extremely dense matter.");
        } else if ifin == FCLOAK {
            self.game.ncviol += 1;
            self.prout("You have violated the Treaty of Algeron.");
            self.prout("The Romulan Empire can never trust you again.");
        } else if ifin == FCREW {
            self.prout("Your last crew member has died.");
        }
        if ifin != FWON && ifin != FCLOAK && self.game.iscloaked {
            self.prout("Your ship was cloaked so your subspace radio did not receive anything.");
            self.prout("You may have missed some warning messages.");
            self.skip(1);
        }
        if self.game.ship == 'F' {
            self.game.ship = SHIP_NONE;
        } else if self.game.ship == 'E' {
            self.game.ship = 'F';
        }
        self.game.alive = false;
        if self.game.unwon() != 0 {
            // DEVIATION: Python raises ZeroDivisionError if inresor or
            // inkling + 2*incom is 0 (setup never allows it); here the
            // float division just yields inf or NaN.
            let goodies = self.game.state.remres / self.game.inresor;
            let baddies = (self.game.remkl() as f64 + 2.0 * self.game.state.kcmdr.len() as f64)
                / (self.game.inkling as f64 + 2.0 * self.game.incom as f64);
            let r = self.rnd_real2(1.0, 1.5);
            if goodies / baddies >= r {
                self.prout("As a result of your actions, a treaty with the Klingon");
                self.prout("Empire has been signed. The terms of the treaty are");
                let r = self.rnd_real1(3.0);
                if goodies / baddies >= r {
                    self.prout("favorable to the Federation.");
                    self.skip(1);
                    self.prout("Congratulations!");
                } else {
                    self.prout("highly unfavorable to the Federation.");
                }
            } else {
                self.prout("The Federation will be destroyed.");
            }
        } else {
            self.prout("Since you took the last Klingon with you, you are a");
            self.prout("martyr and a hero. Someday maybe they'll erect a");
            self.prout("statue in your memory. Rest in peace, and try not");
            self.prout("to think about pigeons.");
            self.game.gamewon = true;
        }
        self.score();
        self.scanner.chew(); // Clean up leftovers
    }

    /// `score()`: compute player's score.
    pub fn score(&mut self) {
        // `timused` is computed and never used in SST2K; nothing to keep.
        self.game.perdate = self.killrate();
        let ithperd = 500.0 * self.game.perdate + 0.5;
        let mut iwon: i64 = 0;
        if self.game.gamewon {
            iwon = 100 * self.game.skill;
        }
        let klship: i64 = match self.game.ship {
            'E' => 0,
            'F' => 1,
            _ => 2,
        };
        let g = &self.game;
        let ncmdr = g.state.kcmdr.len() as i64;
        let dead_ordinaries = g.inkling - g.remkl() + ncmdr + g.state.nscrem;
        let base = (10 * dead_ordinaries + 50 * (g.incom - ncmdr)) as f64;
        let partial = base + ithperd + iwon as f64 + (20 * (g.inrom - g.state.nromrem)) as f64
            + (200 * (g.inscom - g.state.nscrem)) as f64
            - g.state.nromrem as f64
            + (3 * g.kcaptured) as f64;
        self.game.score = partial - self.badpoints();
        if !self.game.alive {
            self.game.score -= 200.0;
        }
        self.skip(2);
        self.prout("Your score --");
        let g = self.game.clone();
        let st = &g.state;
        if g.inrom - st.nromrem != 0 {
            let s = pf!("%6d Romulans destroyed                 %5d", g.inrom - st.nromrem, 20 * (g.inrom - st.nromrem));
            self.prout(&s);
        }
        if st.nromrem != 0 && g.gamewon {
            let s = pf!("%6d Romulans captured                  %5d", st.nromrem, st.nromrem);
            self.prout(&s);
        }
        if dead_ordinaries != 0 {
            let s = pf!("%6d ordinary Klingons destroyed        %5d", dead_ordinaries, 10 * dead_ordinaries);
            self.prout(&s);
        }
        if g.incom - ncmdr != 0 {
            let s = pf!("%6d Klingon commanders destroyed       %5d", g.incom - ncmdr, 50 * (g.incom - ncmdr));
            self.prout(&s);
        }
        if g.kcaptured != 0 {
            let s = pf!("%d Klingons captured                   %5d", g.kcaptured, 3 * g.kcaptured);
            self.prout(&s);
        }
        if g.inscom - st.nscrem != 0 {
            let s = pf!("%6d Super-Commander destroyed          %5d", g.inscom - st.nscrem, 200 * (g.inscom - st.nscrem));
            self.prout(&s);
        }
        if ithperd != 0.0 {
            let s = pf!("%6.2f Klingons per stardate              %5d", g.perdate, ithperd);
            self.prout(&s);
        }
        if st.starkl != 0 {
            let s = pf!("%6d stars destroyed by your action     %5d", st.starkl, -5 * st.starkl);
            self.prout(&s);
        }
        if st.nplankl != 0 {
            let s = pf!("%6d planets destroyed by your action   %5d", st.nplankl, -10 * st.nplankl);
            self.prout(&s);
        }
        if (g.options & OPTION_WORLDS) != 0 && st.nworldkl != 0 {
            let s = pf!("%6d inhabited planets destroyed by your action   %5d", st.nworldkl, -300 * st.nworldkl);
            self.prout(&s);
        }
        if st.basekl != 0 {
            let s = pf!("%6d bases destroyed by your action     %5d", st.basekl, -100 * st.basekl);
            self.prout(&s);
        }
        if g.nhelp != 0 {
            let s = pf!("%6d calls for help from starbase       %5d", g.nhelp, -45 * g.nhelp);
            self.prout(&s);
        }
        if g.casual != 0 {
            let s = pf!("%6d casualties incurred                %5d", g.casual, -g.casual);
            self.prout(&s);
        }
        if g.abandoned != 0 {
            let s = pf!("%6d crew abandoned in space            %5d", g.abandoned, -3 * g.abandoned);
            self.prout(&s);
        }
        if klship != 0 {
            let s = pf!("%6d ship(s) lost or destroyed          %5d", klship, -100 * klship);
            self.prout(&s);
        }
        if g.ncviol > 0 {
            if g.ncviol == 1 {
                self.prout("1 Treaty of Algeron violation          -100");
            } else {
                let s = pf!("%6d Treaty of Algeron violations       %5d\n", g.ncviol, -100 * g.ncviol);
                self.prout(&s);
            }
        }
        if !g.alive {
            self.prout("Penalty for getting yourself killed        -200");
        }
        if g.gamewon {
            self.proutn("Bonus for winning ");
            match g.skill {
                SKILL_NOVICE => self.proutn("Novice game  "),
                SKILL_FAIR => self.proutn("Fair game    "),
                SKILL_GOOD => self.proutn("Good game    "),
                SKILL_EXPERT => self.proutn("Expert game  "),
                SKILL_EMERITUS => self.proutn("Emeritus game"),
                _ => {}
            }
            let s = pf!("           %5d", iwon);
            self.prout(&s);
        }
        self.skip(1);
        let s = pf!("TOTAL SCORE                               %5d", self.game.score);
        self.prout(&s);
    }

    /// `plaque()`: emit winner's commemorative plaque.
    pub fn plaque(&mut self) {
        let timestring = ctime_now();
        self.plaque_in(None, &timestring);
    }

    /// `plaque()` with the clock (`time.ctime()`) given, and file names
    /// taken relative to `dir` when one is given (for the tests).
    fn plaque_in(&mut self, dir: Option<&Path>, timestring: &str) {
        self.skip(2);
        let mut fp;
        loop {
            self.proutn("File or device name for your plaque: ");
            let winner = self.cgetline().trim_end_matches(['\r', '\n']).to_string();
            let path = match dir {
                Some(d) if !winner.is_empty() => d.join(&winner),
                _ => std::path::PathBuf::from(&winner),
            };
            match std::fs::File::create(&path) {
                Ok(f) => {
                    fp = f;
                    break;
                }
                Err(_) => self.prout("Invalid name."),
            }
        }

        self.proutn("Enter name to go on plaque (up to 30 characters): ");
        let winner = self.cgetline().trim_end_matches(['\r', '\n']).to_string();
        // The 38 below must be 64 for 132-column paper
        let nskip = 38 - (winner.chars().count() as i64).div_euclid(2);
        // This is where the ASCII art picture was emitted.
        // It got garbled somewhere in the chain of transmission to the Almy version.
        // We should restore it if we can find old enough FORTRAN sources.
        let mut out = String::new();
        out.push_str("\n\n\n");
        out.push_str("                                                       U. S. S. ENTERPRISE\n");
        out.push_str("\n\n\n\n");
        out.push_str("                                  For demonstrating outstanding ability as a starship captain\n");
        out.push('\n');
        out.push_str("                                                Starfleet Command bestows to you\n");
        out.push('\n');
        // "%*s%s\n\n" % (nskip, "", winner): a negative width left-justifies,
        // so an empty string pads to |nskip| either way
        out.push_str(&" ".repeat(nskip.unsigned_abs() as usize));
        out.push_str(&winner);
        out.push_str("\n\n");
        out.push_str("                                                           the rank of\n\n");
        out.push_str("                                                       \"Commodore Emeritus\"\n\n");
        out.push_str("                                                          ");
        if self.game.skill == SKILL_EXPERT {
            out.push_str(" Expert level\n\n");
        } else if self.game.skill == SKILL_EMERITUS {
            out.push_str("Emeritus level\n\n");
        } else {
            out.push_str(" Cheat level\n\n");
        }
        out.push_str(&pf!(
            "                                                 This day of %.6s %.4s, %.8s\n\n",
            pyslice(timestring, 4, 10),
            pyslice(timestring, 20, 24),
            pyslice(timestring, 11, 19)
        ));
        out.push_str(&pf!("                                                        Your score:  %d\n\n", self.game.score));
        out.push_str(&pf!("                                                    Klingons per stardate:  %.2f\n", self.game.perdate));
        // DEVIATION: Python raises (and crashes) if writing the opened file
        // fails; the plaque is simply left incomplete here.
        let _ = fp.write_all(out.as_bytes());
    }
}

/// Python `s[a:b]` on characters (never out of range).
fn pyslice(s: &str, a: usize, b: usize) -> String {
    s.chars().skip(a).take(b.saturating_sub(a)).collect()
}

/// `time.ctime()`: "Sat Sep  5 12:34:56 2026" in local time.
/// SUPER_TREK_DATE, if set, is used instead (a string in that same form),
/// so recorded plaques can be reproduced.
pub fn ctime_now() -> String {
    if let Ok(d) = std::env::var("SUPER_TREK_DATE")
        && !d.is_empty()
    {
        return d;
    }
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let (year, mon, mday, hour, min, sec, wday) = local_time(secs);
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    format!(
        "{} {} {:>2} {:02}:{:02}:{:02} {}",
        DAYS[wday.rem_euclid(7) as usize],
        MONTHS[mon.rem_euclid(12) as usize],
        mday,
        hour,
        min,
        sec,
        year
    )
}

/// (year, month 0-11, day of month, hour, minute, second, weekday 0=Sunday)
#[cfg(unix)]
fn local_time(secs: i64) -> (i64, i64, i64, i64, i64, i64, i64) {
    // POSIX `struct tm` starts with these nine ints; the padding covers
    // the platform's extra fields (tm_gmtoff, tm_zone).
    #[repr(C)]
    struct Tm {
        sec: std::ffi::c_int,
        min: std::ffi::c_int,
        hour: std::ffi::c_int,
        mday: std::ffi::c_int,
        mon: std::ffi::c_int,
        year: std::ffi::c_int,
        wday: std::ffi::c_int,
        yday: std::ffi::c_int,
        isdst: std::ffi::c_int,
        _rest: [u64; 8],
    }
    unsafe extern "C" {
        fn tzset();
        fn localtime_r(t: *const std::ffi::c_long, tm: *mut Tm) -> *mut Tm;
    }
    let t = secs as std::ffi::c_long;
    let mut tm = Tm { sec: 0, min: 0, hour: 0, mday: 0, mon: 0, year: 0, wday: 0, yday: 0, isdst: 0, _rest: [0; 8] };
    // SAFETY: localtime_r writes at most sizeof(struct tm) bytes into `tm`,
    // which is larger than any platform's struct tm.
    let ok = unsafe {
        tzset();
        !localtime_r(&t, &mut tm).is_null()
    };
    if !ok {
        return utc_time(secs);
    }
    (
        tm.year as i64 + 1900,
        tm.mon as i64,
        tm.mday as i64,
        tm.hour as i64,
        tm.min as i64,
        tm.sec as i64,
        tm.wday as i64,
    )
}

#[cfg(not(unix))]
fn local_time(secs: i64) -> (i64, i64, i64, i64, i64, i64, i64) {
    utc_time(secs)
}

/// The civil date and time in UTC.
fn utc_time(secs: i64) -> (i64, i64, i64, i64, i64, i64, i64) {
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    let wday = (days + 4).rem_euclid(7); // 1970-01-01 was a Thursday
    (year, m - 1, d, rem / 3600, rem % 3600 / 60, rem % 60, wday)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{LineReader, Output};
    use std::sync::{Arc, Mutex};

    struct Capture(Arc<Mutex<String>>);
    impl Output for Capture {
        fn write(&mut self, s: &str) {
            self.0.lock().unwrap().push_str(s);
        }
    }

    /// tests/sst2k/test62.tst: the Expert and Emeritus plaques, with the
    /// clock fixed at "Sat Sep  5 12:34:56 2026".
    #[test]
    fn plaque_matches_test62() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sst2k");
        let log = std::fs::read_to_string(dir.join("test62.log")).unwrap();
        let chk = std::fs::read_to_string(dir.join("test62.chk")).unwrap();
        let tmp = std::env::temp_dir().join(format!("super-trek-plaque-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();

        let buf = Arc::new(Mutex::new(String::new()));
        let mut s = Sst::new();
        s.io.out = Box::new(Capture(buf.clone()));
        s.io.slow = false;
        s.set_options(OPTION_TTY);
        s.game.score = 1234.0;
        s.game.perdate = 12.34;
        s.io.replay = Some(LineReader::new(Box::new(std::io::Cursor::new(log.into_bytes()))));
        s.io.replay_started = true;
        s.io.replay_open = true;
        let mut got = String::new();
        for (skill, filename) in [(SKILL_EXPERT, "expert.txt"), (SKILL_EMERITUS, "emeritus.txt")] {
            s.game.skill = skill;
            s.plaque_in(Some(&tmp), "Sat Sep  5 12:34:56 2026");
            got.push_str(&std::mem::take(&mut *buf.lock().unwrap()));
            got.push_str(&format!("\n--- {filename} ---\n"));
            got.push_str(&std::fs::read_to_string(tmp.join(filename)).unwrap());
        }
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(got, chk);
    }

    #[test]
    fn ctime_shape() {
        assert_eq!(utc_time(0), (1970, 0, 1, 0, 0, 0, 4));
        // 2026-09-05 12:34:56 UTC was a Saturday
        assert_eq!(utc_time(1_788_611_696), (2026, 8, 5, 12, 34, 56, 6));
        assert_eq!(ctime_now().chars().count(), 24);
    }

    #[test]
    fn score_without_elapsed_time() {
        let buf = Arc::new(Mutex::new(String::new()));
        let mut s = Sst::new();
        s.io.out = Box::new(Capture(buf.clone()));
        s.game.ship = 'E';
        s.game.alive = true;
        s.game.skill = SKILL_GOOD;
        assert_eq!(s.killrate(), 0.0);
        s.score();
        let out = buf.lock().unwrap().clone();
        assert!(out.ends_with("TOTAL SCORE                                   0\n"), "{out:?}");
    }
}
