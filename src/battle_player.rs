//! Player weapons and defences: cloak, doshield, randdevice, targetcheck, torps, overheat, checkshctrl, hittem, phasers, capture (`sst` 1224-1450, 1987-2549).

use crate::consts::*;
use crate::coord::{Coord, dist};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, cramen, crmena};

#[derive(Clone, Copy, PartialEq, Eq)]
enum CloakAction {
    On,
    Off,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShieldAction {
    None,
    Up,
    Down,
    Nrg,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoMode {
    NotSet,
    Automatic,
    ForceMan,
    Manual,
}

/// Device weights for `randdevice()`, in tenths of a percent.
const DEVICE_WEIGHTS: [i64; 17] = [
    105, // DSRSENS: short range scanners         10.5%
    105, // DLRSENS: long range scanners          10.5%
    120, // DPHASER: phasers                      12.0%
    120, // DPHOTON: photon torpedoes             12.0%
    25,  // DLIFSUP: life support                  2.5%
    65,  // DWARPEN: warp drive                    6.5%
    70,  // DIMPULS: impulse engines               6.5%
    135, // DSHIELD: deflector shields            13.5%
    30,  // DRADIO:  subspace radio                3.0%
    45,  // DSHUTTL: shuttle                       4.5%
    15,  // DCOMPTR: computer                      1.5%
    20,  // NAVCOMP: navigation system             2.0%
    75,  // DTRANSP: transporter                   7.5%
    20,  // DSHCTRL: high-speed shield controller  2.0%
    10,  // DDRAY: death ray                       1.0%
    30,  // DDSP: deep-space probes                3.0%
    10,  // DCLOAK: the cloaking device            1.0
];

impl Sst {
    /// `cloak()`: change cloaking-device status.
    pub fn cloak(&mut self) {
        if self.game.ship == 'F' {
            self.prout("Ye Faerie Queene hath no cloaking device.");
            return;
        }

        let key = self.nexttok();

        if key == Tok::Real {
            self.huh();
            return;
        }

        let action;
        if key == Tok::Alpha {
            if self.scanner.sees("on") {
                if self.game.iscloaked {
                    self.prout("The cloaking device has already been switched on.");
                    return;
                }
                action = CloakAction::On;
            } else if self.scanner.sees("off") {
                if !self.game.iscloaked {
                    self.prout("The cloaking device has already been switched off.");
                    return;
                }
                action = CloakAction::Off;
            } else {
                self.huh();
                return;
            }
        } else if !self.game.iscloaked {
            self.proutn("Switch cloaking device on? ");
            if !self.ja() {
                return;
            }
            action = CloakAction::On;
        } else {
            self.proutn("Switch cloaking device off? ");
            if !self.ja() {
                return;
            }
            action = CloakAction::Off;
        }

        if action == CloakAction::Off {
            if self.game.irhere != 0 && self.game.state.date >= ALGERON && !self.game.isviolreported {
                self.prout("Spock- \"Captain, the Treaty of Algeron is in effect.\n   Are you sure this is wise?\"");
                if !self.ja() {
                    return;
                }
            }
            self.prout("Engineer Scott- \"Aye, Sir.\"");
            self.game.iscloaked = false;
            if self.game.irhere != 0 && self.game.state.date >= ALGERON && !self.game.isviolreported {
                self.prout("The Romulan ship discovers you are breaking the Treaty of Algeron!");
                self.game.ncviol += 1;
                self.game.isviolreported = true;
                return;
            }
        }

        if action == CloakAction::On {
            if self.damaged(DCLOAK) {
                self.prout("Engineer Scott- \"The cloaking device is damaged, Sir.\"");
                return;
            }

            if self.game.condition == "docked" {
                self.prout("You cannot cloak while docked.");
            }

            if self.game.state.date >= ALGERON && !self.game.isviolreported {
                self.prout("Spock- \"Captain, using the cloaking device is a violation");
                self.prout("  of the Treaty of Algeron. Considering the alternatives,");
                self.proutn("  are you sure this is wise? ");
                if !self.ja() {
                    return;
                }
            }
            self.prout("Engineer Scott- \"Cloaking device has engaging, Sir...\"");
            self.attack(true);
            self.prout("Engineer Scott- \"Cloaking device has engaged, Sir.\"");
            self.game.iscloaked = true;

            if self.game.irhere != 0 && self.game.state.date >= ALGERON && !self.game.isviolreported {
                self.prout("The Romulan ship discovers you are breaking the Treaty of Algeron!");
                self.game.ncviol += 1;
                self.game.isviolreported = true;
            }
        }
    }

    /// `doshield(shraise)`: change shield status.
    pub fn doshield(&mut self, shraise: bool) {
        let mut action = ShieldAction::None;
        self.game.ididit = false;
        if shraise {
            action = ShieldAction::Up;
        } else {
            let key = self.nexttok();
            if key == Tok::Alpha {
                if self.scanner.sees("transfer") {
                    action = ShieldAction::Nrg;
                } else {
                    if self.damaged(DSHIELD) {
                        self.prout("Shields damaged and down.");
                        return;
                    }
                    if self.scanner.sees("up") {
                        action = ShieldAction::Up;
                    } else if self.scanner.sees("down") {
                        action = ShieldAction::Down;
                    }
                }
            }
            if action == ShieldAction::None {
                self.proutn("Do you wish to change shield energy? ");
                if self.ja() {
                    action = ShieldAction::Nrg;
                } else if self.damaged(DSHIELD) {
                    self.prout("Shields damaged and down.");
                    return;
                } else if self.game.shldup {
                    self.proutn("Shields are up. Do you want them down? ");
                    if self.ja() {
                        action = ShieldAction::Down;
                    } else {
                        self.scanner.chew();
                        return;
                    }
                } else {
                    self.proutn("Shields are down. Do you want them up? ");
                    if self.ja() {
                        action = ShieldAction::Up;
                    } else {
                        self.scanner.chew();
                        return;
                    }
                }
            }
        }
        match action {
            ShieldAction::Up => {
                // raise shields
                if self.game.shldup {
                    self.prout("Shields already up.");
                    return;
                }
                self.game.shldup = true;
                self.game.shldchg = true;
                if self.game.condition != "docked" {
                    self.game.energy -= 50.0;
                }
                self.prout("Shields raised.");
                if self.game.energy <= 0.0 {
                    self.skip(1);
                    self.prout("Shields raising uses up last of energy.");
                    self.finish(FNRG);
                    return;
                }
                self.game.ididit = true;
            }
            ShieldAction::Down => {
                if !self.game.shldup {
                    self.prout("Shields already down.");
                    return;
                }
                self.game.shldup = false;
                self.game.shldchg = true;
                self.prout("Shields lowered.");
                self.game.ididit = true;
            }
            ShieldAction::Nrg => {
                while self.nexttok() != Tok::Real {
                    self.scanner.chew();
                    self.proutn("Energy to transfer to shields- ");
                }
                let nrg = self.scanner.real;
                self.scanner.chew();
                if nrg == 0.0 {
                    return;
                }
                if nrg > self.game.energy {
                    self.prout("Insufficient ship energy.");
                    return;
                }
                self.game.ididit = true;
                if self.game.shield + nrg >= self.game.inshld {
                    self.prout("Shield energy maximized.");
                    if self.game.shield + nrg > self.game.inshld {
                        self.prout("Excess energy requested returned to ship energy");
                    }
                    self.game.energy -= self.game.inshld - self.game.shield;
                    self.game.shield = self.game.inshld;
                    return;
                }
                if nrg < 0.0 && self.game.energy - nrg > self.game.inenrg {
                    // Prevent shield drain loophole
                    self.skip(1);
                    self.prout("Engineering to bridge--");
                    self.prout("  Scott here. Power circuit problem, Captain.");
                    self.prout("  I can't drain the shields.");
                    self.game.ididit = false;
                    return;
                }
                if self.game.shield + nrg < 0.0 {
                    self.prout("All shield energy transferred to ship.");
                    self.game.energy += self.game.shield;
                    self.game.shield = 0.0;
                    return;
                }
                self.proutn("Scotty- \"");
                if nrg > 0.0 {
                    self.prout("Transferring energy to shields.\"");
                } else {
                    self.prout("Draining energy from shields.\"");
                }
                self.game.shield += nrg;
                self.game.energy -= nrg;
            }
            ShieldAction::None => {}
        }
    }

    /// `randdevice()`: choose a device to damage, at random.
    pub fn randdevice(&mut self) -> usize {
        let idx = self.rnd_integer(1000.0);
        let mut wsum = 0;
        for (i, w) in DEVICE_WEIGHTS.iter().enumerate() {
            wsum += w;
            if idx < wsum {
                return i;
            }
        }
        unreachable!("randdevice: index {idx} past the weights")
    }

    /// `targetcheck(w)`: None if the target is invalid, otherwise a course angle.
    pub fn targetcheck(&mut self, w: Coord) -> Option<f64> {
        if !w.valid_sector() {
            self.huh();
            return None;
        }
        // C code this was translated from is wacky -- why the sign reversal?
        let delta = Coord::new(self.game.sector.i - w.i, w.j - self.game.sector.j);
        if delta == Coord::new(0, 0) {
            self.skip(1);
            self.prout("Spock-  \"Bridge to sickbay.  Dr. McCoy,");
            self.prout("  I recommend an immediate review of");
            self.prout("  the Captain's psychological profile.\"");
            self.scanner.chew();
            return None;
        }
        Some(delta.bearing())
    }

    /// `torps()`: launch photon torpedo salvo.
    pub fn torps(&mut self) {
        let mut tcourse: Vec<f64> = Vec::new();
        self.game.ididit = false;
        if self.damaged(DPHOTON) {
            self.prout("Photon tubes damaged.");
            self.scanner.chew();
            return;
        }
        if self.game.torps == 0 {
            self.prout("No torpedoes left.");
            self.scanner.chew();
            return;
        }
        // First, get torpedo count
        let n: i64;
        loop {
            self.nexttok();
            // Python compares scanner.token (the token text) with "IHALPHA"
            // and "IHEOL", which never match: only `not scanner.waiting()`
            // decides this branch.
            if !self.scanner.waiting() {
                let s = pf!("%d torpedoes left.", self.game.torps);
                self.prout(&s);
                self.scanner.chew();
                self.proutn("Number of torpedoes to fire- ");
                continue; // Go back around to get a number
            } else {
                // scanner.int() is round(None) for an alpha token: TypeError
                if self.scanner.real.is_nan() {
                    self.huh();
                    return;
                }
                let v = self.scanner.int();
                if v <= 0 {
                    // abort command
                    self.scanner.chew();
                    return;
                }
                if v > MAXBURST {
                    self.scanner.chew();
                    let s = pf!("Maximum of %d torpedoes per burst.", MAXBURST);
                    self.prout(&s);
                    return;
                }
                if v > self.game.torps {
                    self.scanner.chew(); // User requested more torps than available
                    continue; // Go back around
                }
                n = v;
                break; // All is good, go to next stage
            }
        }
        // Next, get targets
        let mut ntargets = 0usize;
        for i in 0..n {
            let key = self.nexttok();
            if i == 0 && key == Tok::Eol {
                break; // no coordinate waiting, we will try prompting
            }
            if i == 1 && key == Tok::Eol {
                // direct all torpedoes at one target
                let mut i = i;
                while i < n {
                    ntargets += 1;
                    tcourse.push(tcourse[0]);
                    i += 1;
                }
                break;
            }
            let tok = self.scanner.token.clone();
            self.scanner.push(&tok);
            let Some(target) = self.getcoord() else {
                return;
            };
            ntargets += 1;
            match self.targetcheck(target) {
                Some(c) => tcourse.push(c),
                None => return,
            }
        }
        self.scanner.chew();
        if ntargets == 0 {
            // prompt for each one
            for i in 0..n {
                let s = pf!("Target sector for torpedo number %d- ", i + 1);
                self.proutn(&s);
                self.scanner.chew();
                let Some(target) = self.getcoord() else {
                    return;
                };
                match self.targetcheck(target) {
                    Some(c) => tcourse.push(c),
                    None => return,
                }
            }
        }
        self.game.ididit = true;
        // Loop for moving <n> torpedoes
        for i in 0..n {
            if self.game.condition != "docked" {
                self.game.torps -= 1;
            }
            let r1 = self.rnd_real();
            let r2 = self.rnd_real();
            let mut dispersion = (r1 + r2) * 0.5 - 0.5;
            if dispersion.abs() >= 0.47 {
                // misfire!
                dispersion *= self.rnd_real2(1.2, 2.2);
                if n > 0 {
                    let s = pf!("***TORPEDO NUMBER %d MISFIRES", i + 1);
                    self.prouts(&s);
                } else {
                    self.prouts("***TORPEDO MISFIRES.");
                }
                self.skip(1);
                if i < n {
                    self.prout("  Remainder of burst aborted.");
                }
                if self.withprob(0.2) {
                    self.prout("***Photon tubes damaged by misfire.");
                    self.game.damage[DPHOTON] = self.game.damfac * self.rnd_real2(1.0, 3.0);
                }
                let _ = dispersion;
                break;
            }
            if self.game.iscloaked {
                dispersion *= 1.2;
            } else if self.game.shldup || self.game.condition == "docked" {
                dispersion *= 1.0 + 0.0001 * self.game.shield;
            }
            let sector = self.game.sector;
            self.torpedo(sector, tcourse[i as usize], dispersion, i, n);
            if self.game.alldone || self.game.here().supernova {
                return;
            }
        }
        if self.game.unwon() <= 0 {
            self.finish(FWON);
        }
    }

    /// `overheat(rpow)`: check for phasers overheating.
    pub fn overheat(&mut self, rpow: f64) {
        if rpow > 1500.0 {
            let checkburn = (rpow - 1500.0) * 0.00038;
            if self.withprob(checkburn) {
                self.prout("Weapons officer Sulu-  \"Phasers overheated, sir.\"");
                let r = self.rnd_real2(1.0, 2.0);
                self.game.damage[DPHASER] = self.game.damfac * r * (1.0 + checkburn);
            }
        }
    }

    /// `checkshctrl(rpow)`: check shield control.
    pub fn checkshctrl(&mut self, rpow: f64) -> bool {
        self.skip(1);
        if self.withprob(0.998) {
            self.prout("Shields lowered.");
            return false;
        }
        // Something bad has happened
        self.prouts("***RED ALERT!  RED ALERT!");
        self.skip(2);
        let hit = rpow * self.game.shield / self.game.inshld;
        self.game.energy -= rpow + hit * 0.8;
        self.game.shield -= hit * 0.2;
        if self.game.energy <= 0.0 {
            self.prouts("Sulu-  \"Captain! Shield malf***********************\"");
            self.skip(1);
            self.stars();
            self.finish(FPHASER);
            return true;
        }
        self.prouts("Sulu-  \"Captain! Shield malfunction! Phaser fire contained!\"");
        self.skip(2);
        self.prout("Lt. Uhura-  \"Sir, all decks reporting damage.\"");
        let icas = self.rnd_integer(trunc(hit * 0.012) as f64);
        self.skip(1);
        self.fry(0.8 * hit);
        if icas != 0 {
            self.skip(1);
            self.prout("McCoy to bridge- \"Severe radiation burns, Jim.");
            let s = pf!("  %d casualties so far.\"", icas);
            self.prout(&s);
            self.game.casual += icas;
            self.game.state.crew -= icas;
        }
        self.skip(1);
        self.prout("Phaser energy dispersed by shields.");
        self.prout("Enemy unaffected.");
        self.overheat(rpow);
        true
    }

    /// `hittem(hits)`: register a phaser hit on Klingons and Romulans.
    pub fn hittem(&mut self, hits: &[f64]) {
        self.skip(1);
        // Snapshot targets because deadkl() removes destroyed enemies from the list.
        let targets: Vec<_> = self.game.enemies.clone();
        for (&wham, &enemy) in hits.iter().zip(targets.iter()) {
            if wham == 0.0 {
                continue;
            }
            let dustfac = self.rnd_real2(0.9, 1.0);
            let hit = wham * pypow(dustfac, self.game.pool[enemy].kdist);
            let kpini = self.game.pool[enemy].power;
            let mut kp = kpini.abs();
            // Python min(): the first of equals
            let cap = PHASEFAC * hit;
            if cap < kp {
                kp = cap;
            }
            if self.game.pool[enemy].power < 0.0 {
                self.game.pool[enemy].power -= -kp;
            } else {
                self.game.pool[enemy].power -= kp;
            }
            let kpow = self.game.pool[enemy].power;
            let w = self.game.pool[enemy].location;
            if hit > 0.005 {
                let s = pf!("%d unit hit on ", trunc(hit));
                self.proutn(&s);
            } else {
                self.proutn("Very small hit on ");
            }
            let ienm = self.game.q(w);
            self.proutn(&crmena(false, ienm, "sector", &w.to_string()));
            self.skip(1);
            if kpow == 0.0 {
                self.deadkl(w, ienm, w);
                if self.game.unwon() == 0 {
                    self.finish(FWON);
                }
                if self.game.alldone {
                    return;
                }
                continue;
            } else {
                // decide whether or not to emasculate klingon
                if kpow > 0.0 && self.withprob(0.9) && kpow <= self.rnd_real2(0.4, 0.8) * kpini {
                    let s = pf!("***Mr. Spock-  \"Captain, the vessel at Sector %s", w);
                    self.prout(&s);
                    self.prout("   has just lost its firepower.\"");
                    self.game.pool[enemy].power = -kpow;
                }
            }
        }
    }

    /// `phasers()`: fire phasers at bad guys.
    #[allow(unused_assignments, clippy::neg_cmp_op_on_partial_ord, clippy::needless_range_loop)]
    pub fn phasers(&mut self) {
        let mut hits: Vec<f64> = Vec::new();
        let mut kz: i64 = 0;
        // Python's irec starts as int 0 and becomes a float; "%d" prints both alike
        let mut irec: f64 = 0.0; // Cheating inhibitor
        let mut ifast = false;
        let mut no = false;
        let mut itarg = true;
        let mut msgflag = true;
        let mut rpow = 0.0;
        let mut automode = AutoMode::NotSet;
        let mut key = Tok::None;
        self.skip(1);
        // SR sensors and Computer are needed for automode
        if self.damaged(DSRSENS) || self.damaged(DCOMPTR) {
            itarg = false;
        }
        if self.game.condition == "docked" {
            self.prout("Phasers can't be fired through base shields.");
            self.scanner.chew();
            return;
        }
        if self.damaged(DPHASER) {
            self.prout("Phaser control damaged.");
            self.scanner.chew();
            return;
        }
        if self.game.shldup {
            if self.damaged(DSHCTRL) {
                self.prout("High speed shield control damaged.");
                self.scanner.chew();
                return;
            }
            if self.game.energy <= 200.0 {
                self.prout("Insufficient energy to activate high-speed shield control.");
                self.scanner.chew();
                return;
            }
            self.prout("Weapons Officer Sulu-  \"High-speed shield control enabled, sir.\"");
            ifast = true;
        }
        // Original code so convoluted, I re-did it all
        // (That was Tom Almy talking about the C code, I think -- ESR)
        while automode == AutoMode::NotSet {
            key = self.nexttok();
            if key == Tok::Alpha {
                if self.scanner.sees("manual") {
                    if self.game.enemies.is_empty() {
                        self.prout("There is no enemy present to select.");
                        self.scanner.chew();
                        key = Tok::Eol;
                        automode = AutoMode::Automatic;
                    } else {
                        automode = AutoMode::Manual;
                        key = self.nexttok();
                    }
                } else if self.scanner.sees("automatic") {
                    if !itarg && !self.game.enemies.is_empty() {
                        automode = AutoMode::ForceMan;
                    } else {
                        if self.game.enemies.is_empty() {
                            self.prout("Energy will be expended into space.");
                        }
                        automode = AutoMode::Automatic;
                        key = self.nexttok();
                    }
                } else if self.scanner.sees("no") {
                    no = true;
                } else {
                    self.huh();
                    return;
                }
            } else if key == Tok::Real {
                if self.game.enemies.is_empty() {
                    self.prout("Energy will be expended into space.");
                    automode = AutoMode::Automatic;
                } else if !itarg {
                    automode = AutoMode::ForceMan;
                } else {
                    automode = AutoMode::Automatic;
                }
            } else {
                // "IHEOL"
                if self.game.enemies.is_empty() {
                    self.prout("Energy will be expended into space.");
                    automode = AutoMode::Automatic;
                } else if !itarg {
                    automode = AutoMode::ForceMan;
                } else {
                    self.proutn("Manual or automatic? ");
                    self.scanner.chew();
                }
            }
        }
        let mut avail = self.game.energy;
        if ifast {
            avail -= 200.0;
        }
        match automode {
            AutoMode::Automatic => {
                if key == Tok::Alpha && self.scanner.sees("no") {
                    no = true;
                    key = self.nexttok();
                }
                if key != Tok::Real && !self.game.enemies.is_empty() {
                    let s = pf!("Phasers locked on target. Energy available: %.2f", avail);
                    self.prout(&s);
                }
                irec = 0.0;
                loop {
                    self.scanner.chew();
                    if kz == 0 {
                        for i in 0..self.game.enemies.len() {
                            let e = self.game.enemies[i];
                            let a = self.game.pool[e].power.abs() / (PHASEFAC * pypow(0.90, self.game.pool[e].kdist));
                            let r = self.rnd_real2(1.01, 1.06);
                            irec += a * r + 1.0;
                        }
                    }
                    kz = 1;
                    let s = pf!("%d units required. ", irec);
                    self.proutn(&s);
                    self.scanner.chew();
                    self.proutn("Units to fire= ");
                    key = self.nexttok();
                    if key != Tok::Real {
                        return;
                    }
                    rpow = self.scanner.real;
                    if rpow > avail {
                        let s = pf!("Energy available= %.2f", avail);
                        self.proutn(&s);
                        self.skip(1);
                        key = Tok::Eol;
                    }
                    if !(rpow > avail) {
                        break;
                    }
                }
                if rpow <= 0.0 {
                    // chicken out
                    self.scanner.chew();
                    return;
                }
                key = self.nexttok();
                if key == Tok::Alpha && self.scanner.sees("no") {
                    no = true;
                }
                if ifast {
                    self.game.energy -= 200.0; // Go and do it!
                    if self.checkshctrl(rpow) {
                        return;
                    }
                }
                self.scanner.chew();
                self.game.energy -= rpow;
                let mut extra = rpow;
                if !self.game.enemies.is_empty() {
                    extra = 0.0;
                    let mut powrem = rpow;
                    for i in 0..self.game.enemies.len() {
                        hits.push(0.0);
                        if powrem <= 0.0 {
                            continue;
                        }
                        let e = self.game.enemies[i];
                        hits[i] = self.game.pool[e].power.abs() / (PHASEFAC * pypow(0.90, self.game.pool[e].kdist));
                        let mut over = self.rnd_real2(1.01, 1.06) * hits[i];
                        let temp = powrem;
                        powrem -= hits[i] + over;
                        if powrem <= 0.0 && temp < hits[i] {
                            hits[i] = temp;
                        }
                        if powrem <= 0.0 {
                            over = 0.0;
                        }
                        extra += over;
                    }
                    if powrem > 0.0 {
                        extra += powrem;
                    }
                    self.hittem(&hits);
                    self.game.ididit = true;
                }
                if extra > 0.0 && !self.game.alldone {
                    if self.game.tholian.is_some() {
                        self.proutn("*** Tholian web absorbs ");
                        if !self.game.enemies.is_empty() {
                            self.proutn("excess ");
                        }
                        self.prout("phaser energy.");
                    } else {
                        let s = pf!("%d expended on empty space.", trunc(extra));
                        self.prout(&s);
                    }
                }
            }
            AutoMode::ForceMan => {
                self.scanner.chew();
                if self.damaged(DCOMPTR) {
                    self.prout("Battle computer damaged, manual fire only.");
                } else {
                    self.skip(1);
                    self.prouts("---WORKING---");
                    self.skip(1);
                    self.prout("Short-range-sensors-damaged");
                    self.prout("Insufficient-data-for-automatic-phaser-fire");
                    self.prout("Manual-fire-must-be-used");
                    self.skip(1);
                }
            }
            AutoMode::Manual => {
                rpow = 0.0;
                // Keep a slot for every target, including those we cannot locate.
                let nenemies = self.game.enemies.len();
                hits = vec![0.0; nenemies];
                for k in 0..nenemies {
                    let e = self.game.enemies[k];
                    let aim = self.game.pool[e].location;
                    let ienm = self.game.q(aim);
                    if msgflag {
                        let s = pf!("Energy available= %.2f", avail - 0.006);
                        self.proutn(&s);
                        self.skip(1);
                        msgflag = false;
                        rpow = 0.0;
                    }
                    if self.damaged(DSRSENS) && !(dist(self.game.sector, aim) < pypow(2.0, 0.5)) && (ienm == 'C' || ienm == 'S') {
                        let s = cramen(ienm).to_string() + " can't be located without short range scan.";
                        self.prout(&s);
                        self.scanner.chew();
                        key = Tok::Eol;
                        continue;
                    }
                    if key == Tok::Eol {
                        self.scanner.chew();
                        if itarg && k as i64 > kz {
                            let a = self.game.pool[e].power.abs() / (PHASEFAC * pypow(0.9, self.game.pool[e].kdist));
                            irec = a * self.rnd_real2(1.01, 1.06) + 1.0;
                        }
                        kz = k as i64;
                        self.proutn("(");
                        if !self.damaged(DCOMPTR) {
                            let s = pf!("%d", irec);
                            self.proutn(&s);
                        } else {
                            self.proutn("??");
                        }
                        self.proutn(")  ");
                        let s = pf!("units to fire at %s-  ", crmena(false, ienm, "sector", &aim.to_string()));
                        self.proutn(&s);
                        key = self.nexttok();
                    }
                    if key == Tok::Alpha && self.scanner.sees("no") {
                        no = true;
                        key = self.nexttok();
                        continue;
                    }
                    if key == Tok::Alpha {
                        self.huh();
                        return;
                    }
                    if key == Tok::Eol {
                        if k == 1 {
                            // Let me say I'm baffled by this
                            msgflag = true;
                        }
                        continue;
                    }
                    if self.scanner.real < 0.0 {
                        // abort out
                        self.scanner.chew();
                        return;
                    }
                    hits[k] = self.scanner.real;
                    rpow += self.scanner.real;
                    // If total requested is too much, inform and start over
                    if rpow > avail {
                        self.prout("Available energy exceeded -- try again.");
                        self.scanner.chew();
                        return;
                    }
                    key = self.nexttok(); // scan for next value
                }
                if rpow == 0.0 {
                    // zero energy -- abort
                    self.scanner.chew();
                    return;
                }
                if key == Tok::Alpha && self.scanner.sees("no") {
                    no = true;
                }
                self.game.energy -= rpow;
                self.scanner.chew();
                if ifast {
                    self.game.energy -= 200.0;
                    if self.checkshctrl(rpow) {
                        return;
                    }
                }
                self.hittem(&hits);
                self.game.ididit = true;
            }
            AutoMode::NotSet => {}
        }
        // Say shield raised or malfunction, if necessary
        if self.game.alldone {
            return;
        }
        if ifast {
            self.skip(1);
            if !no {
                if self.withprob(0.01) {
                    self.prout("Sulu-  \"Sir, the high-speed shield control has malfunctioned . . .");
                    self.prouts("         CLICK   CLICK   POP  . . .");
                    self.prout(" No response, sir!");
                    self.game.shldup = false;
                } else {
                    self.prout("Shields raised.");
                }
            } else {
                self.game.shldup = false;
            }
        }
        self.overheat(rpow);
    }

    /// `capture()`: the BSD trek CAPTURE command.
    pub fn capture(&mut self) {
        self.game.ididit = false; // Nothing if we fail
        self.game.optime = 0.0;

        // Make sure there is room in the brig
        if self.game.brigfree == 0 {
            self.prout("Security reports the brig is already full.");
            return;
        }

        if self.damaged(DRADIO) {
            self.prout("Uhura- \"We have no subspace radio communication, sir.\"");
            return;
        }

        if self.damaged(DTRANSP) {
            self.prout("Scotty- \"Transporter damaged, sir.\"");
            return;
        }

        // find out if there are any at all
        if self.game.klhere < 1 {
            self.prout("Uhura- \"Getting no response, sir.\"");
            return;
        }

        // if there is more than one Klingon, find out which one
        // Nah, just select the weakest one since it is most likely to
        // surrender (Tom Almy mod)
        let klingons: Vec<_> =
            self.game.enemies.iter().copied().filter(|&e| matches!(self.game.pool[e].etype, 'K' | 'C' | 'S')).collect();
        if klingons.is_empty() {
            self.prout("Uhura- \"Getting no response, sir.\"");
            return;
        }
        // Negative power marks lost firepower; its magnitude is remaining energy.
        // Python min(key=...): the first of equals
        let mut weakest = klingons[0];
        for &e in &klingons[1..] {
            if self.game.pool[e].power.abs() < self.game.pool[weakest].power.abs() {
                weakest = e;
            }
        }
        self.game.optime = 0.05; // This action will take some time
        self.game.ididit = true; //  So any others can strike back

        // check out that Klingon
        // DEVIATION: Python raises ZeroDivisionError when the weakest has
        // power 0.0; here x becomes inf and the Klingon surrenders.
        let mut x = self.game.energy / (self.game.pool[weakest].power.abs() * klingons.len() as f64);
        x *= 2.5; // would originally have been equivalent of 1.4,
        // but we want command to work more often, more humanely
        if self.rnd_real1(100.0) < x {
            // guess what, he surrendered!!!
            let wloc = self.game.pool[weakest].location;
            let s = pf!("Klingon captain at %s surrenders.", wloc);
            self.prout(&s);
            let mut i = self.rnd_integer(200.0);
            if i > 0 {
                let s = pf!("%d Klingons commit suicide rather than be taken captive.", 200 - i);
                self.prout(&s);
            }
            if i > self.game.brigfree {
                let s = pf!("%d Klingons die because there is no room for them in the brig.", i - self.game.brigfree);
                self.prout(&s);
                i = self.game.brigfree;
            }
            self.game.brigfree -= i;
            let s = pf!("%d captives taken", i);
            self.prout(&s);
            let et = self.game.pool[weakest].etype;
            let sector = self.game.sector;
            self.deadkl(wloc, et, sector);
            if self.game.unwon() <= 0 {
                self.finish(FWON);
            }
            return;
        }

        // big surprise, he refuses to surrender
        self.prout("Fat chance, captain!");
    }
}
