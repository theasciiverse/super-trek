//! Reports and scans: attackreport, report, lrscan, damagereport, rechart, chart, sectscan, status, request, srscan, eta (`sst` 5810-6400).

#![allow(unused_imports, unused_variables)]

use crate::consts::*;
use crate::coord::{Coord, Vec2};
use crate::pf;
use crate::py::*;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit, TrekError, cramen, crmena};
use crate::state::*;

impl Sst {
    pub fn attackreport(&mut self, curt: bool) {
        todo!("attackreport")
    }

    pub fn report(&mut self) {
        todo!("report")
    }

    pub fn lrscan(&mut self, silent: bool) {
        todo!("lrscan")
    }

    pub fn damagereport(&mut self) {
        todo!("damagereport")
    }

    pub fn rechart(&mut self) {
        todo!("rechart")
    }

    pub fn chart(&mut self) {
        todo!("chart")
    }

    pub fn sectscan(&mut self, good_scan: bool, i: i64, j: i64) {
        todo!("sectscan")
    }

    pub fn status(&mut self, req: i64) {
        todo!("status")
    }

    pub fn request(&mut self) {
        todo!("request")
    }

    pub fn srscan(&mut self) {
        todo!("srscan")
    }

    pub fn eta(&mut self) {
        todo!("eta")
    }

}
