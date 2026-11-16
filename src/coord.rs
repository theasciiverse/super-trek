//! `Coord` from `sst` (164-279), split in two: `Coord` for grid positions
//! (integers, possibly "None"), `Vec2` for the float positions of course
//! arithmetic.

use crate::consts::{GALSIZE, OPTION_ALPHAMERIC, QUADSIZE};
use crate::py::{pyround, pypow};
use serde::{Deserialize, Serialize};
use std::cell::Cell;

/// Python's `None` in a coordinate.
pub const NONE: i64 = i64::MIN;

thread_local! {
    /// Mirror of `game.options`, read by `Coord.__str__` as the Python reads
    /// the global. Kept current by `Sst::set_options`.
    pub static OPTIONS: Cell<i64> = const { Cell::new(0) };
}

pub fn letterize(n: i64) -> char {
    char::from_u32(('a' as i64 + n - 1) as u32).unwrap_or('?')
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Coord {
    pub i: i64,
    pub j: i64,
}

impl Default for Coord {
    fn default() -> Self {
        Coord::none()
    }
}

impl Coord {
    pub const fn new(i: i64, j: i64) -> Self {
        Coord { i, j }
    }
    /// `Coord()`: both parts None.
    pub const fn none() -> Self {
        Coord { i: NONE, j: NONE }
    }
    pub fn is_none(&self) -> bool {
        self.i == NONE || self.j == NONE
    }
    pub fn valid_quadrant(&self) -> bool {
        !self.is_none() && self.i >= 0 && self.i < GALSIZE && self.j >= 0 && self.j < GALSIZE
    }
    pub fn valid_sector(&self) -> bool {
        !self.is_none() && self.i >= 0 && self.i < QUADSIZE && self.j >= 0 && self.j < QUADSIZE
    }
    pub fn invalidate(&mut self) {
        self.i = NONE;
        self.j = NONE;
    }
    pub fn add(self, o: Coord) -> Coord {
        Coord::new(self.i + o.i, self.j + o.j)
    }
    pub fn sub(self, o: Coord) -> Coord {
        Coord::new(self.i - o.i, self.j - o.j)
    }
    pub fn mul(self, k: i64) -> Coord {
        Coord::new(self.i * k, self.j * k)
    }
    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.i as f64, self.j as f64)
    }
    /// `distance()` from the origin; integer squares are exact in Python.
    pub fn distance(self) -> f64 {
        ((self.i * self.i + self.j * self.j) as f64).sqrt()
    }
    /// `bearing()` of an integer delta.
    pub fn bearing(self) -> f64 {
        1.90985 * (self.j as f64).atan2(self.i as f64)
    }
    pub fn sgn(self) -> Coord {
        Coord::new(self.i.signum(), self.j.signum())
    }
    /// `quadrant()` of an absolute integer position: `// QUADSIZE`.
    pub fn quadrant(self) -> Coord {
        Coord::new(self.i.div_euclid(QUADSIZE), self.j.div_euclid(QUADSIZE))
    }
    /// `sector()`: `% QUADSIZE`.
    pub fn sector(self) -> Coord {
        Coord::new(self.i.rem_euclid(QUADSIZE), self.j.rem_euclid(QUADSIZE))
    }
    pub fn modq(self, m: i64) -> Coord {
        Coord::new(self.i.rem_euclid(m), self.j.rem_euclid(m))
    }
    pub fn iu(&self) -> usize {
        self.i as usize
    }
    pub fn ju(&self) -> usize {
        self.j as usize
    }
}

impl std::fmt::Display for Coord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            return write!(f, "Nowhere");
        }
        if OPTIONS.with(|o| o.get()) & OPTION_ALPHAMERIC != 0 {
            write!(f, "{}{}", letterize(self.i + 1), self.j + 1)
        } else {
            write!(f, "{} - {}", self.i + 1, self.j + 1)
        }
    }
}

/// A Coord holding floats (course positions and deltas).
#[derive(Clone, Copy, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub i: f64,
    pub j: f64,
}

impl Vec2 {
    pub const fn new(i: f64, j: f64) -> Self {
        Vec2 { i, j }
    }
    pub fn add(self, o: Vec2) -> Vec2 {
        Vec2::new(self.i + o.i, self.j + o.j)
    }
    pub fn sub(self, o: Vec2) -> Vec2 {
        Vec2::new(self.i - o.i, self.j - o.j)
    }
    /// `k * v` (`__rmul__`): each part times k.
    pub fn scale(self, k: f64) -> Vec2 {
        Vec2::new(self.i * k, self.j * k)
    }
    pub fn div(self, k: f64) -> Vec2 {
        Vec2::new(self.i / k, self.j / k)
    }
    pub fn roundtogrid(self) -> Coord {
        Coord::new(pyround(self.i), pyround(self.j))
    }
    pub fn distance(self) -> f64 {
        (pypow(self.i, 2.0) + pypow(self.j, 2.0)).sqrt()
    }
    pub fn bearing(self) -> f64 {
        1.90985 * self.j.atan2(self.i)
    }
    pub fn quadrant(self) -> Coord {
        self.roundtogrid().quadrant()
    }
    pub fn sector(self) -> Coord {
        self.roundtogrid().sector()
    }
}

/// `(a - b).distance()` with Python's arithmetic: both integer coords.
pub fn dist(a: Coord, b: Coord) -> f64 {
    a.sub(b).distance()
}
