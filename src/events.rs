//! The event queue: cancelrest, events (with tractorbeam and destroybase), wait (`sst` 2577-3100). schedule/unschedule/... are in sst.rs.

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn cancelrest(&mut self) -> bool {
        todo!("cancelrest")
    }

    pub fn events(&mut self) {
        todo!("events")
    }

    pub fn wait(&mut self) {
        todo!("wait")
    }

}
