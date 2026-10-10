//! The game's only source of randomness, ported exactly from SST2K's
//! `randomizer`: a linear congruential generator "tested against Knuth
//! vol. 2 by the authors of ADVENT". Every recorded test game starts from a
//! seed, so reproducing this bit for bit is what lets the port replay them.

const LCG_A: u64 = 1093;
const LCG_C: u64 = 221_587;
const LCG_M: u64 = 1_048_576;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    x: u64,
}

impl Rng {
    /// `randomizer.seed(n)`: `lcg_x = n % LCG_M`.
    pub fn seed(n: u64) -> Self {
        Rng { x: n % LCG_M }
    }

    /// `randomizer.random()`: returns the old state over `LCG_M`, then steps.
    pub fn random(&mut self) -> f64 {
        let old = self.x;
        self.x = (LCG_A * self.x + LCG_C) % LCG_M;
        old as f64 / LCG_M as f64
    }

    /// `randomizer.withprob(p)`
    pub fn withprob(&mut self, p: f64) -> bool {
        self.random() < p
    }

    /// `randomizer.integer(n)`: from `[0, n)`.
    pub fn integer(&mut self, n: i64) -> i64 {
        (self.random() * n as f64) as i64
    }

    /// `randomizer.integer(lo, hi)`: from `[lo, hi)`.
    pub fn integer_between(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.random() * (hi - lo) as f64) as i64
    }

    /// `randomizer.real(n)`: from `[0, n)`.
    pub fn real(&mut self, n: f64) -> f64 {
        self.random() * n
    }

    /// `randomizer.real(lo, hi)`: from `[lo, hi)`.
    pub fn real_between(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.random() * (hi - lo)
    }

    /// `randomizer.getrngstate()` / `setrngstate()`, used by saved games.
    pub fn state(&self) -> u64 {
        self.x
    }

    pub fn set_state(&mut self, x: u64) {
        self.x = x;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Reference values printed by SST2K 2.10's own randomizer, loaded from
    // references/super-star-trek/sst and seeded with test01's seed.
    #[test]
    fn matches_sst2k() {
        let mut r = Rng::seed(1_556_084_761);
        assert_eq!(r.state(), 1_046_553);
        assert_eq!([r.random(), r.random(), r.random()], [0.9980707168579102, 0.1026153564453125, 0.3699064254760742]);
        assert_eq!((0..5).map(|_| r.integer(10)).collect::<Vec<_>>(), [5, 5, 6, 9, 3]);
        assert_eq!((0..5).map(|_| r.integer_between(3, 9)).collect::<Vec<_>>(), [4, 6, 4, 8, 4]);
        assert_eq!([r.real(2.5), r.real(2.5)], [0.9581279754638672, 0.26218175888061523]);
        assert_eq!([r.real_between(1.0, 4.0), r.real_between(1.0, 4.0)], [3.5115604400634766, 1.769526481628418]);
        assert_eq!((0..6).map(|_| r.withprob(0.3)).collect::<Vec<_>>(), [false, true, false, false, false, false]);
        assert_eq!(r.state(), 25_158);
    }
}
