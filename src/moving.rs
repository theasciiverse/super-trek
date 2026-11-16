//! Movement: imove, dock, cartesian, getcourse, course, impulse, warp, setwarp (`sst` 4320-4940).

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    /// `cartesian()`: the ship's absolute position.
    pub fn cartesian(&self) -> Coord {
        self.game.quadrant.mul(QUADSIZE).add(self.game.sector)
    }

    /// `course(bearing, distance)` with the default origin (the ship).
    pub fn course(&self, bearing: f64, distance: f64) -> Course {
        Course::new(bearing, distance, self.cartesian().to_vec2())
    }

    pub fn imove(&mut self, icourse: &mut Course, noattack: bool) {
        todo!("imove")
    }

    pub fn dock(&mut self, verbose: bool) {
        todo!("dock")
    }

    pub fn getcourse(&mut self, isprobe: bool) -> Result<Course, TrekError> {
        todo!("getcourse")
    }

    pub fn impulse(&mut self) {
        todo!("impulse")
    }

    pub fn warp(&mut self, wcourse: Option<Course>, involuntary: bool) {
        todo!("warp")
    }

    pub fn setwarp(&mut self) {
        todo!("setwarp")
    }

}
