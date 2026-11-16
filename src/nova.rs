//! Novas and supernovas (`sst` 3103-3356).

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::sst::{Sst, crmena};

impl Sst {
    /// `nova(nov)`: star goes nova.
    pub fn nova(&mut self, nov: Coord) {
        const NCOURSE: [f64; 10] = [0.0, 10.5, 12.0, 1.5, 9.0, 0.0, 3.0, 7.5, 6.0, 4.5];
        let mut bump = Coord::new(0, 0);
        if self.withprob(0.05) {
            // Wow! We've supernova'ed
            let q = self.game.quadrant;
            self.supernova(Some(q));
            return;
        }
        // handle initial nova
        self.game.set_q(nov, '.');
        // BSDTREK is the PLANETS bit: a known quirk, kept (one extra RNG call)
        if self.game.options & OPTION_BSDTREK != 0 && self.withprob(0.25) {
            self.game.set_q(nov, ' ');
        }
        let s = crmena(false, '*', "sector", &nov.to_string()) + " novas.";
        self.prout(&s);
        self.game.here_mut().stars -= 1;
        self.game.state.starkl += 1;
        // Set up queue to recursively trigger adjacent stars
        let mut hits: Vec<Coord> = vec![nov];
        let mut kount: i64 = 0;
        while let Some(start) = hits.pop() {
            for oi in -1..=1 {
                for oj in -1..=1 {
                    if oj == 0 && oi == 0 {
                        continue;
                    }
                    let neighbor = start.add(Coord::new(oi, oj));
                    if !neighbor.valid_sector() {
                        continue;
                    }
                    let iquad = self.game.q(neighbor);
                    match iquad {
                        // Empty space ends reaction
                        '.' | '?' | ' ' | 'T' | '#' => {}
                        '*' => {
                            // Affect another star
                            if self.withprob(0.05) {
                                // This star supernovas
                                let q = self.game.quadrant;
                                self.supernova(Some(q));
                                return;
                            } else {
                                hits.push(neighbor);
                                self.game.here_mut().stars -= 1;
                                self.game.state.starkl += 1;
                                let s = crmena(true, '*', "sector", &neighbor.to_string());
                                self.proutn(&s);
                                self.prout(" novas.");
                                self.game.set_q(neighbor, '.');
                                kount += 1;
                            }
                        }
                        'P' | '@' => {
                            // Destroy planet
                            self.game.here_mut().planet = None;
                            if iquad == 'P' {
                                self.game.state.nplankl += 1;
                            } else {
                                self.game.state.nworldkl += 1;
                            }
                            let s = crmena(true, 'B', "sector", &neighbor.to_string()) + " destroyed.";
                            self.prout(&s);
                            // DEVIATION: Python raises AttributeError if
                            // game.iplnet is None here; we skip the update.
                            if let Some(p) = self.game.iplnet_mut() {
                                p.pclass = Some("destroyed".into());
                            }
                            self.game.iplnet = None;
                            self.game.plnet.invalidate();
                            if self.game.landed {
                                self.finish(FPNOVA);
                                return;
                            }
                            self.game.set_q(neighbor, '.');
                        }
                        'B' => {
                            // Destroy base
                            self.game.here_mut().starbase = false;
                            let q = self.game.quadrant;
                            self.game.state.baseq.retain(|x| *x != q);
                            self.game.base.invalidate();
                            self.game.state.basekl += 1;
                            self.newcnd();
                            let s = crmena(true, 'B', "sector", &neighbor.to_string()) + " destroyed.";
                            self.prout(&s);
                            self.game.set_q(neighbor, '.');
                        }
                        'E' | 'F' => {
                            // Buffet ship
                            self.prout("***Starship buffeted by nova.");
                            if self.game.shldup {
                                if self.game.shield >= 2000.0 {
                                    self.game.shield -= 2000.0;
                                } else {
                                    let diff = 2000.0 - self.game.shield;
                                    self.game.energy -= diff;
                                    self.game.shield = 0.0;
                                    self.game.shldup = false;
                                    self.prout("***Shields knocked out.");
                                    let r = self.rnd_real();
                                    self.game.damage[DSHIELD] += 0.005 * self.game.damfac * r * diff;
                                }
                            } else {
                                self.game.energy -= 2000.0;
                            }
                            if self.game.energy <= 0.0 {
                                self.finish(FNOVA);
                                return;
                            }
                            // add in course nova contributes to kicking starship
                            if let Some(&last) = hits.last() {
                                bump = bump.add(self.game.sector.sub(last).sgn());
                            }
                        }
                        'K' => {
                            // kill klingon
                            self.deadkl(neighbor, iquad, neighbor);
                        }
                        'C' | 'S' | 'R' => {
                            // Damage/destroy big enemies
                            let target = self.game.enemies.iter().copied().find(|&e| self.game.pool[e].location == neighbor);
                            if let Some(target) = target {
                                self.game.pool[target].power -= 800.0; // If firepower is lost, die
                                if self.game.pool[target].power <= 0.0 {
                                    self.deadkl(neighbor, iquad, neighbor);
                                    continue; // neighbor loop
                                }
                                // Else enemy gets flung by the blast wave
                                let newc = neighbor.add(neighbor).sub(start);
                                let s = crmena(true, iquad, "sector", &neighbor.to_string()) + " damaged";
                                self.proutn(&s);
                                if !newc.valid_sector() {
                                    // can't leave quadrant
                                    self.skip(1);
                                    continue;
                                }
                                let iquad1 = self.game.q(newc);
                                if iquad1 == ' ' {
                                    let s = ", blasted into ".to_string() + &crmena(false, ' ', "sector", &newc.to_string());
                                    self.proutn(&s);
                                    self.skip(1);
                                    self.deadkl(neighbor, iquad, newc);
                                    continue;
                                }
                                if iquad1 != '.' {
                                    // can't move into something else
                                    self.skip(1);
                                    continue;
                                }
                                let s = pf!(", buffeted to Sector %s", newc);
                                self.prout(&s);
                                self.game.set_q(neighbor, '.');
                                self.game.set_q(newc, iquad);
                                self.enemy_move(target, Some(newc));
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        // Starship affected by nova -- kick it away.
        let mut dist = kount as f64 * 0.1;
        let idx = 3 * (bump.i + 1) + bump.j + 2;
        let direc = if (-10..10).contains(&idx) {
            // Python list indexing: a negative index counts from the end
            NCOURSE[idx.rem_euclid(10) as usize]
        } else {
            // DEVIATION: Python raises IndexError when two or more buffets
            // push bump outside the table (`sst` 3236); we use the direction
            // of bump's sign instead.
            let b = bump.sgn();
            NCOURSE[(3 * (b.i + 1) + b.j + 2) as usize]
        };
        if direc == 0.0 {
            dist = 0.0;
        }
        if dist == 0.0 {
            return;
        }
        let mut scourse = self.course(direc, dist);
        self.game.optime = scourse.time(4.0);
        self.skip(1);
        self.prout("Force of nova displaces starship.");
        self.imove(&mut scourse, true);
        self.game.optime = scourse.time(4.0);
    }

    /// `supernova(w)`: star goes supernova. `None` picks a star at random.
    pub fn supernova(&mut self, w: Option<Coord>) {
        let mut num: i64;
        let mut npdead: i64 = 0;
        let mut nq: Coord;
        if let Some(wc) = w {
            nq = wc;
        } else {
            // Scheduled supernova -- select star at random.
            let mut nstars: i64 = 0;
            for (_, _, q) in self.game.state.traverse() {
                nstars += q.stars;
            }
            if nstars == 0 {
                return; // nothing to supernova exists
            }
            num = self.rnd_integer(nstars as f64) + 1;
            // the loop variables keep their last values when nothing breaks
            nq = Coord::new(GALSIZE - 1, GALSIZE - 1);
            'found: for i in 0..GALSIZE {
                for j in 0..GALSIZE {
                    num -= self.game.state.gal(Coord::new(i, j)).stars;
                    if num <= 0 {
                        nq = Coord::new(i, j);
                        break 'found;
                    }
                }
            }
            if self.game.idebug {
                self.proutn("=== Super nova here?");
                if self.ja() {
                    nq = self.game.quadrant;
                }
            }
        }
        if nq != self.game.quadrant || self.game.justin {
            // it isn't here, or we just entered (treat as enroute)
            if self.communicating() {
                self.skip(1);
                let s = pf!("Message from Starfleet Command       Stardate %.2f", self.game.state.date);
                self.prout(&s);
                let s = pf!("     Supernova in Quadrant %s; caution advised.", nq);
                self.prout(&s);
            }
        } else {
            // we are in the quadrant!
            num = self.rnd_integer(self.game.state.gal(nq).stars as f64) + 1;
            let mut ns = Coord::new(QUADSIZE - 1, QUADSIZE - 1);
            'found: for i in 0..QUADSIZE {
                for j in 0..QUADSIZE {
                    if self.game.quad[i as usize][j as usize] == '*' {
                        num -= 1;
                        if num == 0 {
                            ns = Coord::new(i, j);
                            break 'found;
                        }
                    }
                }
            }
            self.skip(1);
            self.prouts("***RED ALERT!  RED ALERT!");
            self.skip(1);
            let s = pf!("***Incipient supernova detected at Sector %s", ns);
            self.prout(&s);
            let di = ns.i - self.game.sector.i;
            let dj = ns.j - self.game.sector.j;
            // integer ** 2 is exact in Python
            if (di * di + dj * dj) as f64 <= 2.1 {
                self.proutn("Emergency override attempts t");
                self.prouts("***************");
                self.skip(1);
                self.stars();
                self.game.alldone = true;
            }
        }
        // destroy any Klingons in supernovaed quadrant
        self.game.state.gal_mut(nq).klingons = 0;
        if nq == self.game.state.kscmdr {
            // did in the Supercommander!
            self.game.state.nscrem = 0;
            self.game.isatb = 0;
            self.game.state.kscmdr.invalidate();
            self.game.iscate = false;
            self.unschedule(FSCMOVE);
            self.unschedule(FSCDBAS);
        }
        self.game.state.kcmdr.retain(|c| *c != nq);
        if self.game.state.kcmdr.is_empty() {
            self.unschedule(FTBEAM);
        }
        // destroy Romulans and planets in supernovaed quadrant
        let nrmdead = self.game.state.gal(nq).romulans;
        self.game.state.gal_mut(nq).romulans = 0;
        self.game.state.nromrem -= nrmdead;
        // Destroy planets
        for lp in 0..self.game.inplan as usize {
            if self.game.state.planets[lp].quadrant == nq {
                self.game.state.planets[lp].pclass = Some("destroyed".into());
                npdead += 1;
            }
        }
        // Destroy any base in supernovaed quadrant
        self.game.state.baseq.retain(|x| *x != nq);
        // If starship caused supernova, tally up destruction
        if w.is_some() {
            self.game.state.starkl += self.game.state.gal(nq).stars;
            self.game.state.basekl += self.game.state.gal(nq).starbase as i64;
            self.game.state.nplankl += npdead;
        }
        // Destruction is physical; only knowledge of it depends on communications.
        let g = self.game.state.gal_mut(nq);
        g.starbase = false;
        g.planet = None;
        g.supernova = true;
        if self.game.quadrant == nq || self.communicating() {
            self.game.state.page_mut(nq).supernova = true;
        }
        // If supernova destroys last Klingons give special message
        if self.game.unwon() == 0 && nq != self.game.quadrant {
            self.skip(2);
            if w.is_none() {
                self.prout("Lucky you!");
            }
            let s = pf!("A supernova in %s has just destroyed the last Klingons.", nq);
            self.proutn(&s);
            self.finish(FWON);
            return;
        }
        // if some Klingons remain, continue or die in supernova
        if self.game.alldone {
            self.finish(FSNOVAED);
        }
    }
}
