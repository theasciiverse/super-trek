//! Saved games: freeze and thaw (`sst` 6458-6807), in this port's own versioned JSON format.

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn freeze(&mut self, boss: bool) {
        todo!("freeze")
    }

    pub fn thaw(&mut self) -> bool {
        todo!("thaw")
    }

}
