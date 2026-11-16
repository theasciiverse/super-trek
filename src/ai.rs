//! Enemy movement: welcoming is in setup.rs; tryexit, movebaddy, moveklings, movescom, supercommander, movetholian (`sst` 704-1218).

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn tryexit(&mut self, enemy: EnemyId, look: Coord, irun: bool, defer_remove: bool) -> Vec<EnemyMove> {
        todo!("tryexit")
    }

    pub fn movebaddy(&mut self, enemy: EnemyId) -> Vec<EnemyMove> {
        todo!("movebaddy")
    }

    pub fn moveklings(&mut self) -> Vec<EnemyMove> {
        todo!("moveklings")
    }

    pub fn movescom(&mut self, iq: Coord, avoid: bool) -> bool {
        todo!("movescom")
    }

    pub fn supercommander(&mut self) {
        todo!("supercommander")
    }

    pub fn movetholian(&mut self) {
        todo!("movetholian")
    }

}
