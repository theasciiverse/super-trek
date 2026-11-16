//! Player weapons and defences: cloak, doshield, randdevice, targetcheck, torps, overheat, checkshctrl, hittem, phasers, capture (`sst` 1224-1450, 1987-2549).

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn cloak(&mut self) {
        todo!("cloak")
    }

    pub fn doshield(&mut self, shraise: bool) {
        todo!("doshield")
    }

    pub fn randdevice(&mut self) -> usize {
        todo!("randdevice")
    }

    pub fn targetcheck(&mut self, w: Coord) -> Option<f64> {
        todo!("targetcheck")
    }

    pub fn torps(&mut self) {
        todo!("torps")
    }

    pub fn overheat(&mut self, rpow: f64) {
        todo!("overheat")
    }

    pub fn checkshctrl(&mut self, rpow: f64) -> bool {
        todo!("checkshctrl")
    }

    pub fn hittem(&mut self, hits: &[f64]) {
        todo!("hittem")
    }

    pub fn phasers(&mut self) {
        todo!("phasers")
    }

    pub fn capture(&mut self) {
        todo!("capture")
    }

}
