//! The command loop and its small commands: makemoves, listCommands,
//! helpme, goptions, debugme (`sst` 6402-6440, 7509-7774, 7963-8047).

use crate::consts::*;
use crate::coord::Coord;
use crate::pf;
use crate::py::pyround;
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit};

/// SST2K's help text (`sst.doc`), generated once from `doc/sst-doc.xml`.
pub const SST_DOC: &str = include_str!("../assets/sst.doc");

impl Sst {
    /// `listCommands()`
    pub fn list_commands(&mut self) {
        self.prout("LEGAL COMMANDS ARE:");
        let mut emitted = 0;
        for &(key, opt) in COMMANDS {
            if key == "AM" {
                continue;
            }
            if opt == 0 || (opt & self.game.options) != 0 {
                let s = pf!("%-12s ", key);
                self.proutn(&s);
                emitted += 1;
                if emitted % 5 == 4 {
                    self.skip(1);
                }
            }
        }
        self.skip(1);
    }

    /// `helpme()`: browse on-line help (from the embedded sst.doc).
    pub fn helpme(&mut self) {
        let mut key = self.nexttok();
        loop {
            if key == Tok::Eol {
                self.proutn("Help on what command? ");
                key = self.nexttok();
            }
            if key == Tok::Eol {
                return;
            }
            let up = self.scanner.token.to_uppercase();
            if COMMANDS.iter().any(|(c, _)| *c == up) || up == "ABBREV" {
                break;
            }
            self.skip(1);
            self.list_commands();
            key = Tok::Eol;
            self.scanner.chew();
            self.skip(1);
        }
        let cmd = self.scanner.token.to_uppercase();
        let mut lines = SST_DOC.split_inclusive('\n');
        loop {
            let Some(linebuf) = lines.next() else {
                self.prout("Spock- \"Captain, there is no information on that command.\"");
                return;
            };
            if let Some(rest) = linebuf.strip_prefix("%% ")
                && cmd.to_uppercase() == rest.trim()
            {
                break;
            }
        }
        self.skip(1);
        self.prout("Spock- \"Captain, I've found the following information:\"");
        self.skip(1);
        for linebuf in lines.by_ref() {
            if linebuf.contains("******") {
                break;
            }
            self.proutn(linebuf);
        }
    }

    /// `goptions()`: show, set or clear options.
    pub fn goptions(&mut self) {
        let mode = self.nexttok();
        if mode == Tok::Eol {
            let mut active: Vec<&str> =
                OPTION_NAMES.iter().filter(|(k, v, _)| (v & self.game.options) != 0 && *k != "ALL").map(|(k, _, _)| *k).collect();
            active.sort();
            let s = active.join(" ");
            self.prout(&s);
        } else if self.scanner.token == "set" || self.scanner.token == "clear" {
            let mode = self.scanner.token.clone();
            let mut changemask = 0i64;
            loop {
                self.nexttok();
                if self.scanner.type_ == Tok::Eol {
                    break;
                }
                match option_lookup(&self.scanner.token.to_uppercase()) {
                    Some(v) => changemask |= v,
                    None => {
                        let s = format!("No such option as {}", self.scanner.token);
                        self.prout(&s);
                    }
                }
            }
            if mode == "set" {
                let o = self.game.options | changemask;
                self.set_options(o);
            } else {
                let o = self.game.options & !changemask;
                self.set_options(o);
            }
            self.game.wayback_disabled &= !changemask;
            self.prout("Acknowledged, Captain.");
        } else {
            self.huh();
        }
        self.scanner.chew();
        self.skip(1);
    }

    /// `makemoves()`: the command-interpretation loop.
    pub fn makemoves(&mut self) {
        loop {
            // command loop
            self.drawmaps(1);
            let mut hitme;
            let cmd: &str;
            loop {
                // get a command
                hitme = false;
                self.game.optime = 0.0;
                self.game.justin = false;
                self.scanner.chew();
                self.clrscr();
                self.log_state("prompt");
                if let Some(f) = self.io.on_prompt.as_mut() {
                    f(&self.game);
                }
                self.proutn("COMMAND> ");
                if self.nexttok() == Tok::Eol {
                    continue;
                } else if self.scanner.token.is_empty() {
                    continue;
                }
                self.game.ididit = false;
                self.clrscr();
                self.clrscr();
                let mut abandon_passed = false;
                let mut found = ("", 0i64);
                let up = self.scanner.token.to_uppercase();
                for &(c, opt) in COMMANDS {
                    found = (c, opt);
                    if c == "ABANDON" {
                        abandon_passed = true;
                    }
                    if c == "REQUEST" && self.scanner.token.chars().count() < 3 {
                        continue;
                    }
                    if c == up || (!abandon_passed && c.starts_with(up.as_str())) {
                        break;
                    }
                }
                let (c, opt) = found;
                if c.is_empty() {
                    self.list_commands();
                    continue;
                } else if opt != 0 && (opt & self.game.options) == 0 {
                    self.huh();
                } else {
                    cmd = c;
                    break;
                }
            }
            log::debug!(target: "command", "{cmd}");
            match cmd {
                "SRSCAN" => self.srscan(),
                "STATUS" => self.status(0),
                "REQUEST" => self.request(),
                "LRSCAN" => self.lrscan(false),
                "PHASERS" => {
                    self.phasers();
                    if self.game.ididit {
                        self.checkviol();
                        hitme = true;
                    }
                }
                "TORPEDO" | "PHOTONS" => {
                    self.torps();
                    if self.game.ididit {
                        self.checkviol();
                        hitme = true;
                    }
                }
                "MOVE" | "AM" => {
                    if cmd == "AM" {
                        self.scanner.push("AUTO");
                    }
                    self.warp(None, false);
                }
                "SHIELDS" => {
                    self.doshield(false);
                    if self.game.ididit {
                        hitme = true;
                        self.game.shldchg = false;
                    }
                }
                "DOCK" => {
                    self.dock(true);
                    if self.game.ididit {
                        self.attack(false);
                    }
                }
                "DAMAGES" => self.damagereport(),
                "CHART" => self.makechart(),
                "IMPULSE" => self.impulse(),
                "REST" => {
                    self.wait();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "WARP" => self.setwarp(),
                "SENSORS" => self.sensor(),
                "ORBIT" => {
                    self.orbit();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "TRANSPORT" => self.beam(),
                "MINE" => {
                    self.mine();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "CRYSTALS" => {
                    self.usecrystals();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "SHUTTLE" => {
                    self.shuttle();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "PLANETS" => self.survey(),
                "REPORT" => self.report(),
                "COMPUTER" => self.eta(),
                "COMMANDS" => self.list_commands(),
                "EMEXIT" => {
                    self.clrscr();
                    self.freeze(true);
                    std::panic::resume_unwind(Box::new(SystemExit(1)));
                }
                "PROBE" => {
                    self.probe();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "ABANDON" => self.abandon(),
                "DESTRUCT" => self.selfdestruct(),
                "SAVE" => {
                    self.freeze(false);
                    self.clrscr();
                    if self.game.skill > SKILL_GOOD {
                        self.prout("WARNING--Saved games produce no plaques!");
                    }
                }
                "DEATHRAY" => {
                    self.deathray();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "CAPTURE" => self.capture(),
                "CLOAK" => self.cloak(),
                // "DEBUG" falls through: SST2K tests for "DEBUGCMD" (kept)
                "MAYDAY" => {
                    self.mayday();
                    if self.game.ididit {
                        hitme = true;
                    }
                }
                "QUIT" => self.game.alldone = true,
                "HELP" => self.helpme(),
                "SCORE" => self.score(),
                // CURSES switches SST2K to its curses screen; this port's
                // screen is the UI around the line mode, so it does nothing.
                "CURSES" => {}
                "OPTIONS" => self.goptions(),
                _ => {}
            }
            loop {
                if self.game.alldone {
                    break;
                }
                if self.game.optime != 0.0 {
                    self.events();
                    if self.game.alldone {
                        break;
                    }
                }
                if self.game.here().supernova {
                    self.atover(false);
                    continue;
                }
                if hitme && !self.game.justin {
                    self.attack(true);
                    if self.game.alldone {
                        break;
                    }
                    if self.game.here().supernova {
                        self.atover(false);
                        hitme = true;
                        continue;
                    }
                }
                break;
            }
            if self.game.alldone {
                break;
            }
        }
        if self.game.idebug {
            self.prout("=== Ending");
        }
    }

    fn checkviol(&mut self) {
        if self.game.irhere != 0 && self.game.state.date >= ALGERON && !self.game.isviolreported && self.game.iscloaked {
            self.prout("The Romulan ship discovers you are breaking the Treaty of Algeron!");
            self.game.ncviol += 1;
            self.game.isviolreported = true;
        }
    }

    /// `debugme()`: reachable only through the dead DEBUG command; kept.
    #[allow(dead_code)]
    pub fn debugme(&mut self) {
        self.proutn("Reset levels? ");
        if self.ja() {
            self.game.energy = self.game.energy.max(self.game.inenrg);
            self.game.shield = self.game.inshld;
            self.game.torps = self.game.intorps;
            self.game.lsupres = self.game.inlsr;
        }
        self.proutn("Reset damage? ");
        if self.ja() {
            for i in 0..NDEVICES {
                if self.game.damage[i] > 0.0 {
                    self.game.damage[i] = 0.0;
                }
            }
        }
        self.proutn("Toggle debug flag? ");
        if self.ja() {
            self.game.idebug = !self.game.idebug;
            if self.game.idebug {
                self.prout("Debug output ON");
            } else {
                self.prout("Debug output OFF");
            }
        }
        self.proutn("Cause selective damage? ");
        if self.ja() {
            for i in 0..NDEVICES {
                let s = pf!("Kill %s?", DEVICE[i]);
                self.proutn(&s);
                self.scanner.chew();
                let key = self.nexttok();
                if key == Tok::Alpha && self.scanner.sees("y") {
                    self.game.damage[i] = 10.0;
                }
            }
        }
        self.proutn("Examine/change events? ");
        if self.ja() {
            let legends = [
                "",
                "Supernova       ",
                "T Beam          ",
                "Snapshot        ",
                "Base Attack     ",
                "Base Destroy    ",
                "SC Move         ",
                "SC Base Destroy ",
                "Probe Move      ",
                "Distress Call   ",
                "Enslavement     ",
                "Klingon Build   ",
            ];
            let mut w = Coord::none();
            for i in 1..NEVENTS {
                self.proutn(legends[i]);
                if self.is_scheduled(i) {
                    let s = pf!("%.2f", self.scheduled(i) - self.game.state.date);
                    self.proutn(&s);
                    if i == FENSLV || i == FREPRO {
                        let s = pf!(" in %s", self.game.future[i].quadrant);
                        self.proutn(&s);
                    }
                } else {
                    self.proutn("never");
                }
                self.proutn("? ");
                self.scanner.chew();
                let key = self.nexttok();
                if key == Tok::Real {
                    let r = self.scanner.real;
                    self.schedule(i, r);
                    if i == FENSLV || i == FREPRO {
                        self.scanner.chew();
                        self.proutn("In quadrant- ");
                        let key = self.nexttok();
                        if key != Tok::Eol {
                            if key != Tok::Real {
                                let s = pf!("Event %d canceled, no x coordinate.", i);
                                self.prout(&s);
                                self.unschedule(i);
                                continue;
                            }
                            w.i = pyround(self.scanner.real);
                            let key = self.nexttok();
                            if key != Tok::Real {
                                let s = pf!("Event %d canceled, no y coordinate.", i);
                                self.prout(&s);
                                self.unschedule(i);
                                continue;
                            }
                            w.j = pyround(self.scanner.real);
                            self.game.future[i].quadrant = w;
                        }
                    }
                }
            }
            self.scanner.chew();
        }
        self.proutn("Induce supernova here? ");
        if self.ja() {
            self.game.here_mut().supernova = true;
            self.atover(true);
        }
    }
}
