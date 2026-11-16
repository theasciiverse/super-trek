//! Planets: consumeTime, survey, orbit, sensor, beam, mine, usecrystals, shuttle, deathray (`sst` 5319-5804).

use crate::consts::*;
use crate::pf;
use crate::py::*;
use crate::sst::Sst;

impl Sst {
    /// `consumeTime()`: abort a lengthy operation if an event interrupts it.
    pub fn consume_time(&mut self) -> bool {
        self.game.ididit = true;
        self.events();
        if self.game.alldone || self.game.here().supernova || self.game.justin {
            return true;
        }
        false
    }

    /// `survey()`: report on (uninhabited) planets in the galaxy.
    pub fn survey(&mut self) {
        let mut iknow = false;
        self.skip(1);
        self.scanner.chew();
        self.prout("Spock-  \"Planet report follows, Captain.\"");
        self.skip(1);
        for i in 0..self.game.inplan as usize {
            let p = self.game.state.planets[i].clone();
            if p.pclass() == "destroyed" {
                continue;
            }
            if (p.known != "unknown" && !p.inhabited) || self.game.idebug {
                iknow = true;
                if self.game.idebug && p.known == "unknown" {
                    self.proutn("(Unknown) ");
                }
                let s = pf!("Quadrant %s", p.quadrant);
                self.proutn(&s);
                self.proutn("   class ");
                self.proutn(p.pclass());
                self.proutn("   ");
                if p.crystals != "present" {
                    self.proutn("no ");
                }
                self.prout("dilithium crystals present.");
                if p.known == "shuttle_down" {
                    self.prout("    Shuttle Craft Galileo on surface.");
                }
            }
        }
        if !iknow {
            self.prout("No information available.");
        }
    }

    /// `orbit()`: enter standard orbit.
    pub fn orbit(&mut self) {
        self.skip(1);
        self.scanner.chew();
        if self.game.inorbit {
            self.prout("Already in standard orbit.");
            return;
        }
        if self.damaged(DWARPEN) && self.damaged(DIMPULS) {
            self.prout("Both warp and impulse engines damaged.");
            return;
        }
        if self.game.iplnet.is_none() || self.game.plnet.is_none() {
            self.prout("There is no planet in this sector.");
            return;
        }
        if (self.game.sector.i - self.game.plnet.i).abs() > 1 || (self.game.sector.j - self.game.plnet.j).abs() > 1 {
            let s = self.crmshp() + " not adjacent to planet.";
            self.prout(&s);
            self.skip(1);
            return;
        }
        self.game.optime = self.rnd_real2(0.02, 0.05);
        self.prout("Helmsman Sulu-  \"Entering standard orbit, Sir.\"");
        self.newcnd();
        if self.consume_time() {
            return;
        }
        self.game.height = self.rnd_real2(1400.0, 8600.0);
        let s = pf!("Sulu-  \"Entered orbit at altitude %.2f kilometers.\"", self.game.height);
        self.prout(&s);
        self.game.inorbit = true;
        self.game.ididit = true;
    }

    /// `sensor()`: examine planets in this quadrant.
    pub fn sensor(&mut self) {
        if self.damaged(DSRSENS) {
            if self.game.options & OPTION_TTY != 0 {
                self.prout("Short range sensors damaged.");
            }
            return;
        }
        let Some(k) = self.game.iplnet else {
            if self.game.options & OPTION_TTY != 0 {
                self.prout("Spock- \"No planet in this quadrant, Captain.\"");
            }
            return;
        };
        let p = self.game.state.planets[k].clone();
        if p.known == "unknown" {
            let s = pf!("Spock-  \"Sensor scan for Quadrant %s-", self.game.quadrant);
            self.prout(&s);
            self.skip(1);
            let s = pf!("         Planet at Sector %s is of class %s.", self.game.plnet, p.pclass());
            self.prout(&s);
            if p.known == "shuttle_down" {
                self.prout("         Sensors show Galileo still on surface.");
            }
            self.proutn("         Readings indicate");
            if p.crystals != "present" {
                self.proutn(" no");
            }
            self.prout(" dilithium crystals present.\"");
            if p.known == "unknown" {
                self.game.state.planets[k].known = "known".into();
            }
        } else if p.inhabited {
            let s = pf!("Spock-  \"The inhabited planet %s ", p.name.clone());
            self.prout(&s);
            let s = pf!("        is located at Sector %s, Captain.\"", self.game.plnet);
            self.prout(&s);
        }
    }

    /// `game.iplnet.known`, with the planet guaranteed by the caller's checks.
    fn iplnet_known(&self) -> String {
        // DEVIATION: Python raises AttributeError on `game.iplnet.known`
        // when iplnet is None; here it reads as "unknown".
        self.game.iplnet().map(|p| p.known.clone()).unwrap_or_else(|| "unknown".into())
    }

    fn set_iplnet_known(&mut self, v: &str) {
        if let Some(p) = self.game.iplnet_mut() {
            p.known = v.into();
        }
    }

    /// `beam()`: use the transporter.
    pub fn beam(&mut self) {
        let mut nrgneed = 0.0;
        self.scanner.chew();
        self.skip(1);
        if !self.game.inorbit {
            let s = self.crmshp() + " not in standard orbit.";
            self.prout(&s);
            return;
        }
        if self.game.iplnet.is_none() {
            self.prout("Spock- \"No planet in this quadrant, Captain.\"");
            return;
        }
        if self.damaged(DTRANSP) {
            self.prout("Transporter damaged.");
            if !self.damaged(DSHUTTL) && (self.iplnet_known() == "shuttle_down" || self.game.iscraft == "onship") {
                self.skip(1);
                self.proutn("Spock-  \"May I suggest the shuttle craft, Sir?\" ");
                if self.ja() {
                    self.shuttle();
                }
            }
            return;
        }
        if self.game.shldup {
            self.prout("Impossible to transport through shields.");
            return;
        }
        if self.iplnet_known() == "unknown" {
            self.prout("Spock-  \"Captain, we have no information on this planet");
            self.prout("  and Starfleet Regulations clearly state that in this situation");
            self.prout("  you may not go down.\"");
            return;
        }
        if !self.game.landed && self.game.iplnet().map(|p| p.crystals == "absent").unwrap_or(false) {
            self.prout("Spock-  \"Captain, I fail to see the logic in");
            self.prout("  exploring a planet with no dilithium crystals.");
            self.proutn("  Are you sure this is wise?\" ");
            if !self.ja() {
                self.scanner.chew();
                return;
            }
        }
        if self.game.options & OPTION_ALMY != 0 {
            nrgneed = (50 * self.game.skill) as f64 + self.game.height / 100.0;
            if nrgneed > self.game.energy {
                self.prout("Engineering to bridge--");
                self.prout("  Captain, we don't have enough energy for transportation.");
                return;
            }
            if !self.game.landed && nrgneed * 2.0 > self.game.energy {
                self.prout("Engineering to bridge--");
                self.prout("  Captain, we have enough energy only to transport you down to");
                self.prout("  the planet, but there wouldn't be an energy for the trip back.");
                if self.iplnet_known() == "shuttle_down" {
                    self.prout("  Although the Galileo shuttle craft may still be on a surface.");
                }
                self.proutn("  Are you sure this is wise?\" ");
                if !self.ja() {
                    self.scanner.chew();
                    return;
                }
            }
        }
        if self.game.landed {
            // Coming from planet
            if self.iplnet_known() == "shuttle_down" {
                self.proutn("Spock-  \"Wouldn't you rather take the Galileo?\" ");
                if self.ja() {
                    self.scanner.chew();
                    return;
                }
                self.prout("Your crew hides the Galileo to prevent capture by aliens.");
            }
            self.prout("Landing party assembled, ready to beam up.");
            self.skip(1);
            self.prout("Kirk whips out communicator...");
            self.prouts("BEEP  BEEP  BEEP");
            self.skip(2);
            self.prout("\"Kirk to enterprise-  Lock on coordinates...energize.\"");
        } else {
            // Going to planet
            self.prout("Scotty-  \"Transporter room ready, Sir.\"");
            self.skip(1);
            self.prout("Kirk and landing party prepare to beam down to planet surface.");
            self.skip(1);
            self.prout("Kirk-  \"Energize.\"");
        }
        self.game.ididit = true;
        self.skip(1);
        self.prouts("WWHOOOIIIIIRRRRREEEE.E.E.  .  .  .  .   .    .");
        self.skip(2);
        if !self.withprob(0.98) {
            self.prouts("BOOOIIIOOOIIOOOOIIIOIING . . .");
            self.skip(2);
            self.prout("Scotty-  \"Oh my God!  I've lost them.\"");
            self.finish(FLOST);
            return;
        }
        self.prouts(".    .   .  .  .  .  .E.E.EEEERRRRRIIIIIOOOHWW");
        self.game.landed = !self.game.landed;
        self.game.energy -= nrgneed;
        self.skip(2);
        self.prout("Transport complete.");
        if self.game.landed && self.iplnet_known() == "shuttle_down" {
            self.prout("The shuttle craft Galileo is here!");
        }
        if !self.game.landed && self.game.imine {
            self.game.icrystl = true;
            self.game.cryprob = 0.05;
        }
        self.game.imine = false;
    }

    /// `mine()`: strip-mine a world for dilithium.
    pub fn mine(&mut self) {
        self.skip(1);
        self.scanner.chew();
        if !self.game.landed {
            self.prout("Mining party not on planet.");
            return;
        }
        // DEVIATION: Python raises AttributeError when landed with no
        // planet (iplnet None); here the command does nothing.
        let Some(k) = self.game.iplnet else {
            return;
        };
        let crystals = self.game.state.planets[k].crystals.clone();
        if crystals == "mined" {
            self.prout("This planet has already been strip-mined for dilithium.");
            return;
        } else if crystals == "absent" {
            self.prout("No dilithium crystals on this planet.");
            return;
        }
        if self.game.imine {
            self.prout("You've already mined enough crystals for this trip.");
            return;
        }
        if self.game.icrystl && self.game.cryprob == 0.05 {
            let s = "With all those fresh crystals aboard the ".to_string() + &self.crmshp();
            self.prout(&s);
            self.prout("there's no reason to mine more at this time.");
            return;
        }
        // ord(pclass) - ord("L"); pclass is "M", "N" or "O" here
        let c = self.game.state.planets[k].pclass().chars().next().unwrap_or('L');
        let r = self.rnd_real2(0.1, 0.3);
        self.game.optime = r * (c as i64 - 'L' as i64) as f64;
        if self.consume_time() {
            return;
        }
        self.prout("Mining operation complete.");
        // `game.iplnet` re-read after events(), as Python does
        if let Some(p) = self.game.iplnet_mut() {
            p.crystals = "mined".into();
        }
        self.game.imine = true;
        self.game.ididit = true;
    }

    /// `usecrystals()`: use dilithium crystals.
    pub fn usecrystals(&mut self) {
        self.game.ididit = false;
        self.skip(1);
        self.scanner.chew();
        if !self.game.icrystl {
            self.prout("No dilithium crystals available.");
            return;
        }
        if self.game.energy >= 1000.0 {
            self.prout("Spock-  \"Captain, Starfleet Regulations prohibit such an operation");
            self.prout("  except when Condition Yellow exists.");
            return;
        }
        self.prout("Spock- \"Captain, I must warn you that loading");
        self.prout("  raw dilithium crystals into the ship's power");
        self.prout("  system may risk a severe explosion.");
        self.proutn("  Are you sure this is wise?\" ");
        if !self.ja() {
            self.scanner.chew();
            return;
        }
        self.skip(1);
        self.prout("Engineering Officer Scott-  \"(GULP) Aye Sir.");
        self.prout("  Mr. Spock and I will try it.\"");
        self.skip(1);
        self.prout("Spock-  \"Crystals in place, Sir.");
        self.prout("  Ready to activate circuit.\"");
        self.skip(1);
        self.prouts("Scotty-  \"Keep your fingers crossed, Sir!\"");
        self.skip(1);
        let p = self.game.cryprob;
        if self.withprob(p) {
            self.prouts("  \"Activating now! - - No good!  It's***");
            self.skip(2);
            self.prouts("***RED ALERT!  RED A*L********************************");
            self.skip(1);
            self.stars();
            self.prouts("******************   KA-BOOM!!!!   *******************");
            self.skip(1);
            self.kaboom();
            return;
        }
        let r = self.rnd_real2(5000.0, 5500.0);
        self.game.energy += r;
        self.prouts("  \"Activating now! - - ");
        self.prout("The instruments");
        self.prout("   are going crazy, but I think it's");
        self.prout("   going to work!!  Congratulations, Sir!\"");
        self.game.cryprob *= 2.0;
        self.game.ididit = true;
    }

    /// `shuttle()`: use shuttlecraft for planetary jaunt.
    pub fn shuttle(&mut self) {
        self.scanner.chew();
        self.skip(1);
        if self.damaged(DSHUTTL) {
            if self.game.damage[DSHUTTL] == -1.0 {
                if self.game.inorbit && self.iplnet_known() == "shuttle_down" {
                    self.prout("Ye Faerie Queene has no shuttle craft bay to dock it at.");
                } else {
                    self.prout("Ye Faerie Queene had no shuttle craft.");
                }
            } else if self.game.damage[DSHUTTL] > 0.0 {
                self.prout("The Galileo is damaged.");
            } else {
                // game.damage[DSHUTTL] < 0
                self.prout("Shuttle craft is now serving Big Macs.");
            }
            return;
        }
        if !self.game.inorbit {
            let s = self.crmshp() + " not in standard orbit.";
            self.prout(&s);
            return;
        }
        if self.iplnet_known() != "shuttle_down" && self.game.iscraft != "onship" {
            self.prout("Shuttle craft not currently available.");
            return;
        }
        if !self.game.landed && self.iplnet_known() == "shuttle_down" {
            self.prout("You will have to beam down to retrieve the shuttle craft.");
            return;
        }
        if self.game.shldup || self.game.condition == "docked" {
            self.prout("Shuttle craft cannot pass through shields.");
            return;
        }
        if self.iplnet_known() == "unknown" {
            self.prout("Spock-  \"Captain, we have no information on this planet");
            self.prout("  and Starfleet Regulations clearly state that in this situation");
            self.prout("  you may not fly down.\"");
            return;
        }
        self.game.optime = 3.0e-5 * self.game.height;
        if !self.game.sandbox && self.game.optime >= 0.8 * self.game.state.remtime {
            self.prout("First Officer Spock-  \"Captain, I compute that such");
            let s = pf!(
                "  a maneuver would require approximately %2d%% of our",
                trunc(100.0 * self.game.optime / self.game.state.remtime)
            );
            self.proutn(&s);
            self.prout("remaining time.");
            self.proutn("Are you sure this is wise?\" ");
            if !self.ja() {
                self.game.optime = 0.0;
                return;
            }
        }
        if self.game.landed {
            // Kirk on planet
            if self.game.iscraft == "onship" {
                // Galileo on ship!
                if !self.damaged(DTRANSP) {
                    self.proutn("Spock-  \"Would you rather use the transporter?\" ");
                    if self.ja() {
                        self.beam();
                        return;
                    }
                    self.proutn("Shuttle crew");
                } else {
                    self.proutn("Rescue party");
                }
                self.prout(" boards Galileo and swoops toward planet surface.");
                self.game.iscraft = "offship".into();
                self.skip(1);
                if self.consume_time() {
                    return;
                }
                self.set_iplnet_known("shuttle_down");
                self.prout("Trip complete.");
            } else {
                // Ready to go back to ship
                self.prout("You and your mining party board the");
                self.prout("shuttle craft for the trip back to the Enterprise.");
                self.skip(1);
                self.prouts("The short hop begins . . .");
                self.skip(1);
                self.set_iplnet_known("known");
                self.game.icraft = true;
                self.skip(1);
                self.game.landed = false;
                if self.consume_time() {
                    return;
                }
                self.game.iscraft = "onship".into();
                self.game.icraft = false;
                if self.game.imine {
                    self.game.icrystl = true;
                    self.game.cryprob = 0.05;
                }
                self.game.imine = false;
                self.prout("Trip complete.");
            }
        } else {
            // Kirk on ship and so is Galileo
            self.prout("Mining party assembles in the hangar deck,");
            self.prout("ready to board the shuttle craft \"Galileo\".");
            self.skip(1);
            self.prouts("The hangar doors open; the trip begins.");
            self.skip(1);
            self.game.icraft = true;
            self.game.iscraft = "offship".into();
            if self.consume_time() {
                return;
            }
            self.set_iplnet_known("shuttle_down");
            self.game.landed = true;
            self.game.icraft = false;
            self.prout("Trip complete.");
        }
    }

    /// `deathray()`: use the big zapper.
    pub fn deathray(&mut self) {
        self.game.ididit = false;
        self.skip(1);
        self.scanner.chew();
        if self.game.ship != 'E' {
            self.prout("Ye Faerie Queene has no death ray.");
            return;
        }
        if self.game.enemies.is_empty() {
            self.prout("Sulu-  \"But Sir, there are no enemies in this quadrant.\"");
            return;
        }
        if self.damaged(DDRAY) {
            self.prout("Death Ray is damaged.");
            return;
        }
        self.prout("Spock-  \"Captain, the 'Experimental Death Ray'");
        self.prout("  is highly unpredictible.  Considering the alternatives,");
        self.proutn("  are you sure this is wise?\" ");
        if !self.ja() {
            return;
        }
        self.prout("Spock-  \"Acknowledged.\"");
        self.skip(1);
        self.game.ididit = true;
        self.prouts("WHOOEE ... WHOOEE ... WHOOEE ... WHOOEE");
        self.skip(1);
        self.prout("Crew scrambles in emergency preparation.");
        self.prout("Spock and Scotty ready the death ray and");
        self.prout("prepare to channel all ship's power to the device.");
        self.skip(1);
        self.prout("Spock-  \"Preparations complete, sir.\"");
        self.prout("Kirk-  \"Engage!\"");
        self.skip(1);
        self.prouts("WHIRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRRR");
        self.skip(1);
        let mut dprob = 0.30;
        // Ugh. This test (For Tom Almy's death-ray upgrade) was inverted for a long time.
        // Furthermore, somebody (ESR or Stas?) changed Tom Almy's 0.7 upgraded chance of
        // working to 0.5.
        if self.game.options & OPTION_ALMY != 0 {
            dprob = 0.5;
        }
        let r = self.rnd_real();
        if r > dprob {
            self.prouts("Sulu- \"Captain!  It's working!\"");
            self.skip(2);
            while let Some(&last) = self.game.enemies.last() {
                let loc = self.game.pool[last].location;
                let ch = self.game.q(loc);
                let before = self.game.enemies.len();
                self.deadkl(loc, ch, loc);
                // DEVIATION: Python would loop forever if deadkl left the
                // enemy in the list; stop instead.
                if self.game.enemies.len() >= before && self.game.enemies.last() == Some(&last) {
                    break;
                }
            }
            self.prout("Ensign Chekov-  \"Congratulations, Captain!\"");
            if self.game.unwon() == 0 {
                self.finish(FWON);
            }
            if self.game.options & OPTION_ALMY != 0 {
                self.prout("Spock-  \"Captain, I believe the `Experimental Death Ray'");
                if self.withprob(0.05) {
                    self.prout("   is still operational.\"");
                } else {
                    self.prout("   has been rendered nonfunctional.\"");
                    self.game.damage[DDRAY] = 39.95;
                }
            }
            return;
        }
        let r = self.rnd_real(); // Pick failure method
        if r <= 0.30 {
            self.prouts("Sulu- \"Captain!  It's working!\"");
            self.skip(1);
            self.prouts("***RED ALERT!  RED ALERT!");
            self.skip(1);
            self.prout("***MATTER-ANTIMATTER IMPLOSION IMMINENT!");
            self.skip(1);
            self.prouts("***RED ALERT!  RED A*L********************************");
            self.skip(1);
            self.stars();
            self.prouts("******************   KA-BOOM!!!!   *******************");
            self.skip(1);
            self.kaboom();
            return;
        }
        if r <= 0.55 {
            self.prouts("Sulu- \"Captain!  Yagabandaghangrapl, brachriigringlanbla!\"");
            self.skip(1);
            self.prout("Lt. Uhura-  \"Graaeek!  Graaeek!\"");
            self.skip(1);
            self.prout("Spock-  \"Fascinating!  . . . All humans aboard");
            self.prout("  have apparently been transformed into strange mutations.");
            self.prout("  Vulcans do not seem to be affected.");
            self.skip(1);
            self.prout("Kirk-  \"Raauch!  Raauch!\"");
            self.finish(FDRAY);
            return;
        }
        if r <= 0.75 {
            self.prouts("Sulu- \"Captain!  It's   --WHAT?!?!\"");
            self.skip(2);
            self.proutn("Spock-  \"I believe the word is");
            self.prouts(" *ASTONISHING*");
            self.prout(" Mr. Sulu.");
            for i in 0..QS {
                for j in 0..QS {
                    if self.game.quad[i][j] == '.' {
                        self.game.quad[i][j] = '?';
                    }
                }
            }
            self.prout("  Captain, our quadrant is now infested with");
            self.prouts(" - - - - - -  *THINGS*.");
            self.skip(1);
            self.prout("  I have no logical explanation.\"");
            return;
        }
        self.prouts("Sulu- \"Captain!  The Death Ray is creating tribbles!\"");
        self.skip(1);
        self.prout("Scotty-  \"There are so many tribbles down here");
        self.prout("  in Engineering, we can't move for 'em, Captain.\"");
        self.finish(FTRIBBLE);
    }
}
