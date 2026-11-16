//! Enemy-side battle: collision, torpedo, fry, attack, deadkl (`sst` 1451-1986).

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn collision(&mut self, rammed: bool, enemy: EnemyId) {
        todo!("collision")
    }

    pub fn torpedo(&mut self, origin: Coord, bearing: f64, dispersion: f64, number: i64, nburst: i64) -> Option<f64> {
        todo!("torpedo")
    }

    pub fn fry(&mut self, hit: f64) {
        todo!("fry")
    }

    pub fn attack(&mut self, torps_ok: bool) {
        todo!("attack")
    }

    pub fn deadkl(&mut self, w: Coord, etype: char, mv: Coord) {
        todo!("deadkl")
    }

}
