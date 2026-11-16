//! The game's only source of randomness, ported exactly from SST2K's
//! `randomizer`: a linear congruential generator "tested against Knuth
//! vol. 2 by the authors of ADVENT". Every recorded test game starts from a
//! seed, so reproducing this bit for bit is what lets the port replay them.
//!
//! The state is `game.lcg_x`, as in the Python, so saves carry it. It is a
//! float: integer seeds keep it an exact integer, and a tournament number
//! (a float token) seeds it with a float, as Python does.

use crate::py::{pymod, trunc};
use crate::sst::Sst;

pub const LCG_A: f64 = 1093.0;
pub const LCG_C: f64 = 221_587.0;
pub const LCG_M: f64 = 1_048_576.0;

/// `randomizer.random()` on a state.
pub fn step(x: &mut f64) -> f64 {
    let old = *x;
    *x = pymod(LCG_A * *x + LCG_C, LCG_M);
    old / LCG_M
}

impl Sst {
    pub fn rnd_random(&mut self) -> f64 {
        let v = step(&mut self.game.lcg_x);
        if self.io.rng_trace {
            log::trace!(target: "rng", "{v}");
        }
        v
    }

    /// `rnd.withprob(p)`
    pub fn withprob(&mut self, p: f64) -> bool {
        self.rnd_random() < p
    }

    /// `rnd.integer(n)`: from `[0, n)`.
    pub fn rnd_integer(&mut self, n: f64) -> i64 {
        let v = self.rnd_random();
        trunc(v * n)
    }

    /// `rnd.integer(lo, hi)`: from `[lo, hi)`.
    pub fn rnd_integer2(&mut self, lo: f64, hi: f64) -> i64 {
        let v = self.rnd_random();
        trunc(lo + trunc(v * (hi - lo)) as f64)
    }

    /// `rnd.real()`: from `[0, 1)`.
    pub fn rnd_real(&mut self) -> f64 {
        self.rnd_random()
    }

    /// `rnd.real(n)`: from `[0, n)`.
    pub fn rnd_real1(&mut self, n: f64) -> f64 {
        self.rnd_random() * n
    }

    /// `rnd.real(lo, hi)`: from `[lo, hi)`.
    pub fn rnd_real2(&mut self, lo: f64, hi: f64) -> f64 {
        let v = self.rnd_random();
        lo + v * (hi - lo)
    }

    /// `rnd.seed(n)` with a float (the tournament number).
    pub fn rnd_seed_float(&mut self, n: f64) {
        self.game.lcg_x = pymod(n, LCG_M);
    }

    /// `rnd.seed(n)` with an integer already reduced modulo LCG_M.
    pub fn rnd_seed_int(&mut self, n_mod: i64) {
        self.game.lcg_x = n_mod as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values printed by SST2K 2.10's own randomizer, loaded from
    // references/super-star-trek/sst and seeded with test01's seed.
    #[test]
    fn matches_sst2k() {
        let mut s = Sst::new();
        s.rnd_seed_int(1_556_084_761 % 1_048_576);
        assert_eq!(s.game.lcg_x, 1_046_553.0);
        assert_eq!([s.rnd_random(), s.rnd_random(), s.rnd_random()], [0.9980707168579102, 0.1026153564453125, 0.3699064254760742]);
        assert_eq!((0..5).map(|_| s.rnd_integer(10.0)).collect::<Vec<_>>(), [5, 5, 6, 9, 3]);
        assert_eq!((0..5).map(|_| s.rnd_integer2(3.0, 9.0)).collect::<Vec<_>>(), [4, 6, 4, 8, 4]);
        assert_eq!([s.rnd_real1(2.5), s.rnd_real1(2.5)], [0.9581279754638672, 0.26218175888061523]);
        assert_eq!([s.rnd_real2(1.0, 4.0), s.rnd_real2(1.0, 4.0)], [3.5115604400634766, 1.769526481628418]);
        assert_eq!((0..6).map(|_| s.withprob(0.3)).collect::<Vec<_>>(), [false, true, false, false, false, false]);
        assert_eq!(s.game.lcg_x, 25_158.0);
    }
}
