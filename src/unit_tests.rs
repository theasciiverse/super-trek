//! SST2K's own unit tests (tests/sst2k/testNN.tst that load the game's
//! source into Python and poke its internals), ported to Rust.
//!
//! Each test sets up the same state on an `Sst` as the Python script does
//! on its namespace, calls the same functions, makes the same assertions,
//! and prints the same lines; the lines are then compared with the
//! script's testNN.chk. Python's `contextlib.redirect_stdout` is a
//! capturing `Output`; `ns['game'] = Gamestate()` is `s.game =
//! Gamestate::default()` (the RNG state lives in the game in both); a
//! `replayfp = io.StringIO(text)` is a replay reader over `text`, and
//! `replayfp = None` is no replay with an input that is at EOF.
//!
//! test62 is in finish.rs and test83 (the saved-game loader) in save.rs.

use crate::consts::*;
use crate::coord::Coord;
use crate::io::{Input, LineReader, Output};
use crate::scanner::Tok;
use crate::sst::{Sst, SystemExit};
use crate::state::*;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub(crate) type Buf = Arc<Mutex<String>>;

pub(crate) struct Capture(pub Buf);
impl Output for Capture {
    fn write(&mut self, s: &str) {
        self.0.lock().unwrap().push_str(s);
    }
}

/// Typed lines, then EOF (`input()` raising EOFError).
pub(crate) struct Lines(pub Vec<String>);
impl Input for Lines {
    fn read_line(&mut self) -> Option<String> {
        if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
    }
}

/// A new `Sst` writing into a buffer, reading nothing from stdin.
pub(crate) fn new_sst() -> (Sst, Buf) {
    let buf = Arc::new(Mutex::new(String::new()));
    let mut s = Sst::new();
    s.io.out = Box::new(Capture(buf.clone()));
    s.io.input = Box::new(Lines(Vec::new()));
    s.io.slow = false;
    (s, buf)
}

/// `replayfp = io.StringIO(text)` (Some) or `replayfp = None` (None).
pub(crate) fn set_replay(s: &mut Sst, text: Option<&str>) {
    match text {
        Some(t) => {
            let bytes = t.as_bytes().to_vec();
            s.io.replay = Some(LineReader::new(Box::new(std::io::Cursor::new(bytes))));
            s.io.replay_started = true;
            s.io.replay_open = true;
        }
        None => {
            s.io.replay = None;
            s.io.replay_started = false;
            s.io.replay_open = false;
        }
    }
}

/// What is left unread in the replay file (`replayfp.read()`).
fn replay_rest(s: &mut Sst) -> String {
    let mut rest = String::new();
    if let Some(r) = s.io.replay.as_mut() {
        loop {
            let l = r.readline();
            if l.is_empty() {
                break;
            }
            rest.push_str(&l);
        }
    }
    rest
}

pub(crate) fn take(buf: &Buf) -> String {
    std::mem::take(&mut *buf.lock().unwrap())
}

/// `rnd.seed(n)`
pub(crate) fn seed(s: &mut Sst, n: i64) {
    s.rnd_seed_int(n.rem_euclid(crate::rng::LCG_M as i64));
}

pub(crate) fn sst2k(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sst2k").join(name)
}

pub(crate) fn read(name: &str) -> String {
    std::fs::read_to_string(sst2k(name)).unwrap()
}

/// The `# seed N` line of a log.
fn log_seed(log: &str) -> i64 {
    log.lines().filter_map(|l| l.strip_prefix("# seed ")).next_back().unwrap().trim().parse().unwrap()
}

/// The case lines of a log (not blank, not comments).
fn cases(log: &str) -> Vec<String> {
    log.lines().filter(|l| !l.is_empty() && !l.starts_with('#')).map(String::from).collect()
}

/// A fresh scratch folder, like `tempfile.TemporaryDirectory`. Its path
/// must survive the scanner's lowercasing, as in the Python.
pub(crate) fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("super-trek-unit-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let text = d.to_string_lossy();
    assert!(text == text.to_lowercase() && !text.contains(char::is_whitespace), "scratch path {text} would not survive the scanner");
    d
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn first_of(s: &Sst, etype: char) -> EnemyId {
    *s.game.enemies.iter().find(|&&e| s.game.pool[e].etype == etype).unwrap()
}

fn count_of(s: &Sst, etype: char) -> usize {
    s.game.enemies.iter().filter(|&&e| s.game.pool[e].etype == etype).count()
}

/// `game.state.galaxy[c.i][c.j]`
fn gal(s: &Sst, c: Coord) -> &Quadrant {
    s.game.state.gal(c)
}

fn gal_mut(s: &mut Sst, c: Coord) -> &mut Quadrant {
    s.game.state.gal_mut(c)
}

fn no_negative_counts(s: &Sst) -> bool {
    s.game.state.traverse().all(|(_, _, q)| q.klingons >= 0)
}

// ---------------------------------------------------------------------------

/// test65: strategic Supercommander movement leaves no ghost behind.
#[test]
fn supercommander_movement_matches_test65() {
    let log = read("test65.log");
    let setup = || {
        let (mut s, buf) = new_sst();
        set_replay(&mut s, Some(""));
        s.set_options(OPTION_ALL & !OPTION_CURSES);
        for line in log.lines() {
            if let Some(n) = line.strip_prefix("# seed ") {
                seed(&mut s, n.trim().parse().unwrap());
            } else if line.starts_with("# arguments ") {
                s.scanner.inqueue = line.split_whitespace().skip(2).map(String::from).collect();
            }
        }
        s.setup();
        s.game.quadrant = s.game.state.kscmdr;
        s.newqad();
        s.game.justin = false;
        (s, buf)
    };
    let mut out = String::new();
    for condition in ["green", "red", "docked"] {
        let (mut s, _buf) = setup();
        let source_quad = s.game.quadrant;
        let sc = first_of(&s, 'S');
        let old_sector = s.game.pool[sc].location;
        let mut others = Vec::new();
        if condition == "red" {
            others.push(s.newkling());
            s.game.klhere += 1;
            gal_mut(&mut s, source_quad).klingons += 1;
        }
        if condition == "docked" {
            s.game.state.baseq.push(source_quad);
            gal_mut(&mut s, source_quad).starbase = true;
            s.game.base = s.dropin(Some('B'));
        }
        s.game.condition = if condition == "docked" { "docked" } else { "red" }.into();
        s.game.ientesc = true;
        s.schedule(FSCDBAS, 1.0);
        let total = s.game.remkl();
        if condition == "docked" {
            assert!(s.movescom(Coord::new(6, 6), false));
        } else {
            s.supercommander();
        }
        let destination = s.game.state.kscmdr;
        assert!(destination != source_quad, "Supercommander did not leave");
        assert!(!s.game.enemies.contains(&sc), "Departed Supercommander remains a target");
        assert_eq!(s.game.q(old_sector), '.');
        assert!(s.game.enemies == others, "Departure changed other targets");
        assert_eq!(s.game.klhere, others.len() as i64);
        assert_eq!(gal(&s, source_quad).klingons, others.len() as i64);
        assert_eq!(gal(&s, destination).klingons, 1);
        assert!(s.game.remkl() == total && s.game.state.nscrem == 1);
        assert_eq!(s.game.condition, condition);
        assert!(!s.game.iscate && !s.game.ientesc && s.game.isatb == 0);
        assert!(!s.is_scheduled(FSCDBAS));
        out += &format!("Departure: no ghost, counts agree, condition {condition}.\n");

        // The ship can only be killed after pursuing it into its new quadrant.
        s.game.quadrant = destination;
        s.newqad();
        let sc = first_of(&s, 'S');
        let (loc, etype, sector) = (s.game.pool[sc].location, s.game.pool[sc].etype, s.game.sector);
        s.deadkl(loc, etype, sector);
        assert_eq!(s.game.remkl(), total - 1);
        assert_eq!(s.game.state.nscrem, 0);
        assert!(s.game.klhere == 0 && s.game.enemies.is_empty());
        assert_eq!(gal(&s, destination).klingons, 0);
        assert!(no_negative_counts(&s));
        out += "Pursuit: one kill, no negative galaxy counts.\n";
    }

    let (mut s, _buf) = setup();
    let total = s.game.remkl();
    let enemies = s.game.enemies.clone();
    let sector_map = s.game.quad;
    let here = s.game.quadrant;
    assert!(!s.movescom(here, false));
    assert!(s.game.enemies == enemies && s.game.quad == sector_map);
    assert!(s.game.remkl() == total && s.game.klhere == 1);
    out += "Stationary local move rejected without removing the Supercommander.\n";

    let (mut s, _buf) = setup();
    let origin = s.game.state.kscmdr;
    s.game.quadrant = Coord::new(6, 6);
    s.newqad();
    s.game.justin = false;
    let enemies = s.game.enemies.clone();
    let sector_map = s.game.quad;
    let local_count = s.game.klhere;
    let total = s.game.remkl();
    let here = s.game.quadrant;
    assert!(!s.movescom(here, false));
    assert!(s.game.state.kscmdr == origin && s.game.remkl() == total);
    assert!(s.game.enemies == enemies && s.game.quad == sector_map);
    assert_eq!(s.game.klhere, local_count);
    out += "Move into the Enterprise quadrant rejected without changing counts.\n";

    let destination = Coord::new(6, 5);
    let previous_count = gal(&s, destination).klingons;
    assert!(s.movescom(destination, false));
    assert!(s.game.state.kscmdr == destination && s.game.remkl() == total);
    assert_eq!(gal(&s, origin).klingons, 0);
    assert_eq!(gal(&s, destination).klingons, previous_count + 1);
    assert!(s.game.enemies == enemies && s.game.quad == sector_map);
    assert_eq!(s.game.klhere, local_count);
    out += "Remote movement updates galaxy counts without changing local targets.\n";
    assert_eq!(out, read("test65.chk"));
}

/// test69: enemy counts and the time budget agree across game settings.
#[test]
fn resource_budgets_match_test69() {
    let log = read("test69.log");
    let sd = log_seed(&log);
    let mut out = String::new();
    for line in cases(&log) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let (skill, length, year) = (parts[0], parts[1], parts[2]);
        let (mut s, _buf) = new_sst();
        set_replay(&mut s, Some("captain\n"));
        s.set_options(OPTION_ALL & !OPTION_CURSES);
        seed(&mut s, sd);
        s.scanner.inqueue = vec!["regular".into(), length.into(), skill.into(), year.into()];
        s.setup();
        let expected_time = match length {
            "short" => 7.0,
            "medium" => 14.0,
            "long" => 28.0,
            _ => 9999.0,
        };
        let total = s.game.remkl();
        assert_eq!(total, s.game.inkling + s.game.incom + s.game.inscom);
        assert_eq!(s.game.state.remtime, expected_time);
        let initial_resources = s.game.state.remres;
        assert_eq!(s.game.inresor, initial_resources);
        s.game.recompute();
        assert!((s.game.state.remtime - expected_time).abs() <= 1e-9);
        let date = s.game.state.date;
        s.game.optime = 0.01;
        s.events();
        assert!(!s.game.alldone);
        assert_eq!(s.game.remkl(), total, "Enemies changed during the timing check");
        assert!((s.game.state.date - date - 0.01).abs() <= 1e-9);
        if s.game.sandbox {
            assert_eq!(s.game.state.remtime, 9999.0);
            out += &format!("{line}: deadline remains unlimited.\n");
        } else {
            assert!((s.game.state.remtime - (expected_time - 0.01)).abs() <= 1e-9);
            let ratio = s.game.state.remres / initial_resources;
            assert!((ratio - (expected_time - 0.01) / expected_time).abs() <= 1e-9);
            out += &format!("{line}: {expected_time:.2} -> {:.2} stardates.\n", s.game.state.remtime);
        }
    }
    assert_eq!(out, read("test69.chk"));
}

/// test70: retreats at sector and galaxy edges, removed at once or deferred.
#[test]
fn retreat_destinations_match_test70() {
    let coord = |t: &str| Coord::new(t.as_bytes()[0] as i64 - 'a' as i64, t[1..].parse::<i64>().unwrap() - 1);
    let mut out = String::new();
    for line in cases(&read("test70.log")) {
        let p: Vec<&str> = line.split_whitespace().collect();
        let (origin, sector, row, column, destination) = (p[0], p[1], p[2], p[3], p[4]);
        for etype in ['K', 'C', 'S'] {
            for deferred in [false, true] {
                let (mut s, _buf) = new_sst();
                s.set_options(OPTION_TTY | OPTION_ALPHAMERIC);
                s.game.quadrant = coord(origin);
                s.game.sector = Coord::new(4, 4);
                s.game.ship = 'E';
                s.game.alive = true;
                s.game.energy = 5000.0;
                s.game.condition = "red".into();
                s.game.quad = [['.'; QS]; QS];
                s.game.quad[4][4] = 'E';
                let enemy = s.new_enemy(etype, Some(coord(sector)), 100.0);
                let mut others = Vec::new();
                if etype == 'K' {
                    // An ordinary retreat must leave its local commander alone.
                    others.push(s.new_enemy('C', Some(Coord::new(6, 6)), 1200.0));
                }
                if etype == 'K' || etype == 'C' {
                    s.game.state.kcmdr = vec![s.game.quadrant];
                } else {
                    s.game.state.kscmdr = s.game.quadrant;
                    s.game.state.nscrem = 1;
                }
                s.game.klhere = s.game.enemies.len() as i64;
                let here = s.game.quadrant;
                gal_mut(&mut s, here).klingons = s.game.klhere;
                let total = s.game.remkl();
                let old_map = s.game.quad;
                let look = Coord::new(row.parse().unwrap(), column.parse().unwrap());
                let moves = s.tryexit(enemy, look, false, deferred);
                let mut with_others = vec![enemy];
                with_others.extend(&others);
                let expected;
                if destination == "blocked" {
                    assert!(moves.is_empty(), "{line} {etype} {deferred}");
                    assert!(s.game.quad == old_map);
                    assert!(s.game.enemies == with_others);
                    assert!(s.game.pool[enemy].location == coord(sector) && !s.game.pool[enemy].pending_remove);
                    assert!(s.game.klhere == total && gal(&s, here).klingons == total);
                    expected = s.game.quadrant;
                } else {
                    expected = coord(destination);
                    assert!(moves.len() == 1 && moves[0].left);
                    assert_eq!(moves[0].goto, expected, "{line} {etype} {deferred}");
                    assert_eq!(moves[0].oldloc, coord(sector));
                    assert_eq!(s.game.q(coord(sector)), '.');
                    assert!(s.game.klhere == others.len() as i64 && gal(&s, here).klingons == others.len() as i64);
                    assert_eq!(gal(&s, expected).klingons, 1);
                    assert!(!s.game.pool[enemy].location.valid_sector());
                    assert_eq!(s.game.pool[enemy].pending_remove, deferred);
                    let mut want = if deferred { vec![enemy] } else { vec![] };
                    want.extend(&others);
                    assert!(s.game.enemies == want);
                }
                assert_eq!(s.game.remkl(), total);
                assert!(no_negative_counts(&s));
                if etype == 'K' {
                    assert!(s.game.state.kcmdr == vec![s.game.quadrant]);
                } else if etype == 'C' {
                    assert!(s.game.state.kcmdr == vec![expected]);
                } else {
                    assert!(s.game.state.kscmdr == expected && s.game.state.nscrem == 1);
                }
            }
        }
        out += &format!("{origin} {sector} via ({row},{column}): {destination}; counts and locations agree.\n");
    }
    assert_eq!(out, read("test70.chk"));
}

/// test72: Supercommander base attacks start, respect commander targets,
/// and end with the base destroyed.
#[test]
fn supercommander_base_attack_matches_test72() {
    let log = read("test72.log");
    let sd = log_seed(&log);
    let mut out = String::new();
    for case in cases(&log) {
        let (mut s, buf) = new_sst();
        seed(&mut s, sd);
        set_replay(&mut s, Some(""));
        s.set_options(OPTION_TTY | OPTION_ALPHAMERIC);
        let g = &mut s.game;
        g.skill = SKILL_GOOD;
        (g.inkling, g.incom, g.inscom) = (10, 3, 1);
        g.intime = 7.0;
        g.indate = 100.0;
        g.state.date = 103.1;
        g.state.remres = 140.0;
        g.state.crew = FULLCREW;
        (g.ship, g.alive, g.condition) = ('E', true, "green".into());
        (g.energy, g.shield, g.torps) = (5000.0, 2500.0, 10);
        (g.quadrant, g.sector) = (Coord::new(7, 7), Coord::new(4, 4));
        g.quad = [['.'; QS]; QS];
        g.quad[4][4] = 'E';
        g.state.kscmdr = Coord::new(2, 3);
        g.state.galaxy[2][3].klingons = 1;
        g.state.nscrem = 1;
        let base = Coord::new(3, 3);
        g.state.baseq = vec![base];
        g.battle = Coord::none();
        for event in 0..NEVENTS {
            s.unschedule(event);
        }
        s.schedule(FSCMOVE, 0.2777);
        if case == "radio-damaged" {
            s.game.damage[DRADIO] = 10.0;
        } else if case == "passive" {
            s.game.state.date = 101.0;
        } else if case.starts_with("commander-") {
            s.game.battle = if case == "commander-same-base" { base } else { Coord::new(0, 0) };
            let battle = s.game.battle;
            s.game.state.kcmdr = vec![battle];
            gal_mut(&mut s, battle).klingons = 1;
            s.game.state.baseq.push(if case == "commander-same-base" { Coord::new(4, 3) } else { battle });
            s.schedule(FCDBAS, 0.5);
        }
        for location in s.game.state.baseq.clone() {
            gal_mut(&mut s, location).starbase = true;
        }
        let battle = s.game.battle;
        let commander_deadline = s.scheduled(FCDBAS);
        take(&buf);
        s.supercommander();
        let output = take(&buf);
        if case == "commander-same-base" || case == "passive" {
            if case == "commander-same-base" {
                assert_eq!(s.game.state.kscmdr, base, "Expected passage through the besieged base");
            }
            assert!(s.game.isatb == 0 && !s.is_scheduled(FSCDBAS));
            assert_eq!(s.game.battle, battle);
            assert_eq!(s.scheduled(FCDBAS), commander_deadline);
            out += &format!("{case}: no Supercommander attack scheduled.\n");
            continue;
        }
        assert_eq!(s.game.state.kscmdr, base);
        assert_eq!(s.game.isatb, 1, "Supercommander reached the base without attacking");
        assert!(s.is_scheduled(FSCDBAS));
        assert_eq!(s.game.battle, battle);
        assert_eq!(s.scheduled(FCDBAS), commander_deadline);
        let delay = s.scheduled(FSCDBAS) - s.game.state.date;
        let extra = if case == "commander-elsewhere" { 0.5 } else { 0.0 };
        assert!(1.0 + extra <= delay && delay < 3.0 + extra);
        let warned = output.contains("under attack from the Klingon Super-commander");
        assert_eq!(warned, case != "radio-damaged");
        assert_eq!(s.game.iseenit, warned);
        // Run normal movement ticks and destruction events through the deadline.
        s.game.optime = delay + 0.001;
        s.events();
        assert!(!s.game.alldone);
        assert!(s.game.state.kscmdr == base && s.game.state.nscrem == 1);
        assert!(!gal(&s, base).starbase);
        assert!(!s.game.state.baseq.contains(&base));
        assert!(s.game.isatb == 0 && !s.is_scheduled(FSCDBAS));
        if case == "commander-elsewhere" {
            assert!(!s.is_scheduled(FCDBAS));
            assert!(!gal(&s, battle).starbase);
        }
        out += &format!(
            "{case}: attack scheduled; warning {}; base destroyed.\n",
            if warned { "sent" } else { "suppressed" }
        );
    }
    assert_eq!(out, read("test72.chk"));
}

/// `chart_text()`: the star chart as printed.
fn chart_text(s: &mut Sst, buf: &Buf) -> String {
    let before = take(buf);
    s.chart();
    let text = take(buf);
    buf.lock().unwrap().push_str(&before);
    text
}

/// Save the game to `file` and load it into a fresh game, as test73's and
/// test77's `roundtrip()`. `edit` changes the saved JSON first (the
/// legacy-save cases).
fn save_and_load(s: &mut Sst, file: &Path, edit: impl FnOnce(&mut serde_json::Value)) {
    s.scanner.inqueue = vec![path_str(file)];
    s.freeze(false);
    let mut doc: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    edit(&mut doc["game"]);
    std::fs::write(file, serde_json::to_vec(&doc).unwrap()).unwrap();
    s.game = Gamestate::default();
    s.set_options(OPTION_TTY);
    s.scanner.inqueue = vec![path_str(file)];
    assert!(!s.thaw());
}

/// test73: remote supernovae destroy the quadrant despite radio damage but
/// do not reveal reports that were never received.
#[test]
fn remote_supernova_matches_test73() {
    let log = read("test73.log");
    let sd = log_seed(&log);
    let dir = scratch("test73");
    let file = dir.join("game.trk");
    let mut out = String::new();
    let mut last: Option<(Sst, Buf)> = None;
    for case in cases(&log) {
        let (mut s, buf) = new_sst();
        seed(&mut s, sd);
        set_replay(&mut s, None);
        s.set_options(OPTION_TTY | OPTION_ALPHAMERIC);
        let g = &mut s.game;
        (g.ship, g.alive, g.energy) = ('E', true, 1000.0);
        // Keep the focused galaxy fixture, but supply a valid ship for save/load checks.
        (g.skill, g.length, g.passwd) = (SKILL_GOOD, 1, Some("captain".into()));
        (g.inshld, g.warpfac) = (2500.0, 5.0);
        g.quad = [['.'; QS]; QS];
        g.condition = if case == "docked" { "docked" } else { "green" }.into();
        (g.quadrant, g.sector) = (Coord::new(7, 0), Coord::new(4, 4));
        g.damage[DRADIO] = if case == "radio-working" { 0.0 } else { 5.0 };
        let destination = Coord::new(1, 6);
        {
            let q = g.state.gal_mut(destination);
            (q.stars, q.starbase, q.klingons, q.romulans) = (3, true, 2, 1);
        }
        g.state.baseq = vec![destination];
        g.state.kcmdr = vec![destination];
        g.state.nromrem = 1;
        let planet = Planet { quadrant: destination, pclass: Some("M".into()), ..Planet::default() };
        g.state.planets = vec![planet];
        g.state.gal_mut(destination).planet = Some(0);
        g.inplan = 1;
        // Leave another enemy alive so destruction cannot finish the game.
        g.state.galaxy[7][7].klingons = 1;
        let charted = case != "unexplored-base";
        g.state.gal_mut(destination).charted = charted;
        g.state.page_mut(destination).starbase = true;
        if charted {
            let page = g.state.page_mut(destination);
            (page.klingons, page.stars) = (Some(2), Some(3));
        }
        let before = chart_text(&mut s, &buf);
        take(&buf);
        s.supernova(if case == "induced" { Some(destination) } else { None });
        let output = take(&buf);
        let q = gal(&s, destination).clone();
        assert!(q.supernova, "Radio damage prevented recording the physical supernova");
        assert!(!q.starbase && !s.game.state.baseq.contains(&destination));
        assert!(q.planet.is_none() && s.game.state.planets[0].pclass.as_deref() == Some("destroyed"));
        assert!(q.klingons == 0 && q.romulans == 0 && s.game.state.nromrem == 0);
        assert!(!s.game.state.kcmdr.contains(&destination) && s.game.remkl() == 1);
        assert!(!s.game.alldone);
        let informed = case == "radio-working" || case == "docked";
        assert_eq!(output.contains("Supernova in Quadrant b7"), informed);
        if informed {
            assert!(chart_text(&mut s, &buf).contains("***"));
        } else {
            assert_eq!(chart_text(&mut s, &buf), before, "Chart revealed an unreported supernova");
        }
        if case == "radio-damaged" {
            save_and_load(&mut s, &file, |_| {});
            assert!(gal(&s, destination).supernova);
            assert_eq!(chart_text(&mut s, &buf), before, "Loading a save revealed an unreported supernova");
        }
        if case == "sensor-discovery" {
            s.game.quadrant = Coord::new(2, 6);
            take(&buf);
            s.lrscan(false);
            assert!(take(&buf).contains("***") && chart_text(&mut s, &buf).contains("***"));
        } else if case == "quadrant-entry" {
            s.game.quadrant = destination;
            s.newqad();
            assert!(s.game.base.is_none() && s.game.iplnet.is_none());
            assert!(s.game.quad.iter().flatten().all(|&c| c == '.'));
            assert!(chart_text(&mut s, &buf).contains("***"));
        } else {
            s.game.damage[DRADIO] = 0.0;
            assert!(chart_text(&mut s, &buf).contains("***"), "Repaired radio did not update the chart");
            s.game.quadrant = destination;
            s.newqad();
            assert!(s.game.base.is_none() && s.game.iplnet.is_none());
            assert!(s.game.quad.iter().flatten().all(|&c| c == '.'));
        }
        let expected_kills = if case == "induced" { (3, 1, 1) } else { (0, 0, 0) };
        assert_eq!((s.game.state.starkl, s.game.state.basekl, s.game.state.nplankl), expected_kills);
        out += &format!("{case}: quadrant destroyed; chart respects discovery; no base reappears.\n");
        last = Some((s, buf));
    }

    // Simulate saves predating the separate chart flag, including time-warp
    // history: the Python deletes the attribute from every chart page and
    // pickles; here the key is left out of every chart page of the saved
    // JSON.
    let (mut s, buf) = last.unwrap();
    s.game.damage[DRADIO] = 5.0;
    s.game.snapsht.galaxy[2][2].supernova = true;
    save_and_load(&mut s, &file, |g| {
        for snapshot in ["state", "snapsht"] {
            for row in g[snapshot]["chart"].as_array_mut().unwrap() {
                for page in row.as_array_mut().unwrap() {
                    page.as_object_mut().unwrap().remove("supernova");
                }
            }
        }
    });
    for snapshot in [&s.game.state, &s.game.snapsht] {
        for (i, j, quadrant) in snapshot.traverse() {
            assert_eq!(snapshot.chart[i][j].supernova, quadrant.supernova);
        }
    }
    assert!(chart_text(&mut s, &buf).contains("***"));
    out += "Legacy save: known supernovae preserved in the current state and time-warp history.\n";
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out, read("test73.chk"));
}

/// test74: a supernova that kills the Supercommander leaves no phantom at a1.
#[test]
fn supernova_supercommander_matches_test74() {
    let log = read("test74.log");
    let setup = || {
        let (mut s, buf) = new_sst();
        set_replay(&mut s, Some(""));
        s.set_options(OPTION_ALL & !OPTION_CURSES);
        for line in log.lines() {
            if let Some(n) = line.strip_prefix("# seed ") {
                seed(&mut s, n.trim().parse().unwrap());
            } else if line.starts_with("# arguments ") {
                s.scanner.inqueue = line.split_whitespace().skip(2).map(String::from).collect();
            }
        }
        s.setup();
        (s, buf)
    };
    let mut out = String::new();
    for case in cases(&log) {
        let (mut s, _buf) = setup();
        let destination = s.game.state.kscmdr;
        assert!(destination == Coord::new(2, 1) && s.game.state.nscrem == 1);
        let origin = s.game.quadrant;
        s.game.quadrant = destination;
        s.newqad();
        assert_eq!(count_of(&s, 'S'), 1);
        s.game.quadrant = origin;
        s.newqad();
        // Let the real RNG select the Supercommander's quadrant for scheduled events.
        for i in 0..GS {
            for j in 0..GS {
                if Coord::new(i as i64, j as i64) != destination {
                    s.game.state.galaxy[i][j].stars = 0;
                }
            }
        }
        let total = s.game.remkl();
        let destroyed = gal(&s, destination).klingons;
        s.game.isatb = 1;
        s.schedule(FSCDBAS, 1.0);
        assert!(s.is_scheduled(FSCMOVE));
        if case == "radio-damaged" {
            s.game.damage[DRADIO] = 5.0;
        }
        s.supernova(if case == "induced" { Some(destination) } else { None });
        assert_eq!(s.game.state.nscrem, 0);
        assert!(!s.game.state.kscmdr.valid_quadrant(), "Dead Supercommander still has a quadrant");
        assert!(!s.is_scheduled(FSCMOVE));
        assert!(!s.is_scheduled(FSCDBAS));
        assert!(s.game.isatb == 0 && !s.game.iscate);
        assert!(gal(&s, destination).supernova);
        assert_eq!(s.game.remkl(), total - destroyed);
        if case == "stale-location" {
            // Older saved games can still carry the obsolete a1 coordinates.
            s.game.state.kscmdr = Coord::new(0, 0);
        }
        s.game.quadrant = Coord::new(0, 0);
        s.newqad();
        let klingons: Vec<EnemyId> =
            s.game.enemies.iter().copied().filter(|&e| "KCS".contains(s.game.pool[e].etype)).collect();
        let mut types: Vec<char> = klingons.iter().map(|&e| s.game.pool[e].etype).collect();
        types.sort();
        assert_eq!(types, vec!['C', 'K', 'K', 'K', 'K']);
        assert!(s.game.klhere == s.game.state.galaxy[0][0].klingons && s.game.klhere == klingons.len() as i64);
        assert!(s.game.state.nscrem == 0 && !s.game.iscate);
        let target = *klingons.iter().find(|&&e| s.game.pool[e].etype == 'K').unwrap();
        let (loc, etype, sector) = (s.game.pool[target].location, s.game.pool[target].etype, s.game.sector);
        s.deadkl(loc, etype, sector);
        assert!(s.game.state.nscrem == 0 && s.game.remkl() == total - destroyed - 1);
        assert!(s.game.klhere == 4 && s.game.state.galaxy[0][0].klingons == 4);
        assert!(s.game.state.kcmdr.contains(&Coord::new(0, 0)));
        out += &format!("{case}: Supercommander removed; a1 keeps ordinary ships and its commander; counts agree.\n");
    }

    // Row and column zero are valid locations for a living Supercommander.
    let (mut s, _buf) = setup();
    let old = s.game.state.kscmdr;
    gal_mut(&mut s, old).klingons -= 1;
    s.game.state.kscmdr = Coord::new(0, 0);
    s.game.state.galaxy[0][0].klingons += 1;
    s.game.quadrant = Coord::new(0, 0);
    s.newqad();
    assert_eq!(s.game.state.nscrem, 1);
    assert_eq!(count_of(&s, 'S'), 1);
    assert_eq!(count_of(&s, 'C'), 1);
    out += "Living Supercommander at a1 is still placed alongside its commander.\n";
    assert_eq!(out, read("test74.chk"));
}

/// The `start(...)` of test75 to test77: seed, then set up a regular short
/// game from queued arguments.
fn start(s: &mut Sst, sd: i64, skill: &str, year: &str) {
    seed(s, sd);
    s.scanner.chew();
    s.scanner.inqueue = vec!["regular".into(), "short".into(), skill.into(), year.into()];
    set_replay(s, Some("captain\n"));
    s.setup();
}

/// `visit(location)`
fn visit(s: &mut Sst, location: Coord) {
    s.game.quadrant = location;
    s.newqad();
}

/// test75: a new game clears the previous game's Supercommander. The
/// Python also checks that the play-again loop reuses the same game and
/// snapshot objects (`is`); here there is one `Sst` throughout, so that
/// holds by construction.
#[test]
fn supercommander_restart_matches_test75() {
    let log = read("test75.log");
    let initial_seed = log_seed(&log);
    let mut out = String::new();
    for line in cases(&log) {
        let p: Vec<&str> = line.split_whitespace().collect();
        let (sd, skill, year): (i64, &str, &str) = (p[0].parse().unwrap(), p[1], p[2]);
        let (mut s, _buf) = new_sst();
        s.set_options(OPTION_ALL & !OPTION_CURSES);
        start(&mut s, initial_seed, "good", "2026");
        let previous = s.game.state.kscmdr;
        assert!(s.game.state.nscrem == 1 && previous.valid_quadrant());
        assert!(s.is_scheduled(FSCMOVE));
        visit(&mut s, previous);
        assert_eq!(count_of(&s, 'S'), 1);

        start(&mut s, sd, skill, year);
        assert!(s.game.inscom == 0 && s.game.state.nscrem == 0);
        assert!(!s.game.state.kscmdr.valid_quadrant(), "New game retained the old Supercommander location");
        assert!(!s.is_scheduled(FSCMOVE));
        assert!(!s.is_scheduled(FSCDBAS));
        assert!(s.game.isatb == 0 && !s.game.iscate);
        let total = s.game.remkl();
        assert_eq!(total, s.game.inkling + s.game.incom);
        visit(&mut s, previous);
        assert_eq!(count_of(&s, 'S'), 0);
        let quadrant_klingons = gal(&s, previous).klingons;
        if skill == "fair" {
            assert!(quadrant_klingons > 0, "Reproduction needs Klingons at the old location");
        }
        let local_klingons = s.game.enemies.iter().filter(|&&e| "KCS".contains(s.game.pool[e].etype)).count() as i64;
        assert!(local_klingons == s.game.klhere && s.game.klhere == quadrant_klingons);
        assert!(s.game.remkl() == total && s.game.state.nscrem == 0);
        assert!(!s.game.iscate);
        out += &format!("Good -> {skill} {year}: location cleared; no Supercommander or scheduled attacks; counts agree.\n");

        if year == "2026" {
            start(&mut s, initial_seed, "good", "2026");
            assert!(s.game.state.nscrem == 1 && s.game.state.kscmdr.valid_quadrant());
            assert!(s.is_scheduled(FSCMOVE));
            let k = s.game.state.kscmdr;
            visit(&mut s, k);
            assert_eq!(count_of(&s, 'S'), 1);
            out += &format!("{skill} -> Good: a new Supercommander is allocated and placed.\n");
        }
    }
    assert_eq!(out, read("test75.chk"));
}

/// test76: a new mission clears cloak, brig and scoring state; a saved
/// mission keeps them. (`ns['game'] is game` holds by construction here.)
#[test]
fn mission_restart_matches_test76() {
    let log = read("test76.log");
    let sd = log_seed(&log);
    let (mut s, buf) = new_sst();
    let fresh = |s: &mut Sst| {
        s.game = Gamestate::default();
        s.thing = Thingy::default();
        s.set_options(OPTION_ALL & !(OPTION_CURSES | OPTION_COLOR));
        s.game.cdebug = true;
    };
    let score_text = |s: &mut Sst, buf: &Buf| {
        take(buf);
        s.score();
        take(buf)
    };
    let docking = |s: &mut Sst, buf: &Buf| {
        let b = s.game.state.baseq[0];
        visit(s, b);
        // Position the Enterprise in an empty sector adjacent to the actual base.
        let sec = s.game.sector;
        s.game.set_q(sec, '.');
        let base = s.game.base;
        let mut adjacent = Vec::new();
        for i in 0.max(base.i - 1)..QUADSIZE.min(base.i + 2) {
            for j in 0.max(base.j - 1)..QUADSIZE.min(base.j + 2) {
                if s.game.q(Coord::new(i, j)) == '.' {
                    adjacent.push(Coord::new(i, j));
                }
            }
        }
        assert!(!adjacent.is_empty());
        s.game.sector = adjacent[0];
        s.game.set_q(adjacent[0], 'E');
        take(buf);
        s.dock(true);
        let output = take(buf);
        assert_eq!(s.game.condition, "docked", "New Enterprise could not dock");
        assert!(!output.contains("captured Klingons transferred"));
        assert!(s.game.brigfree == 400 && s.game.kcaptured == 0);
    };
    let dir = scratch("test76");
    let mut out = String::new();
    for line in cases(&log) {
        let p: Vec<&str> = line.split_whitespace().collect();
        let (ship, capacity, free, year) = (p[0].chars().next().unwrap(), p[1], p[2], p[3]);
        fresh(&mut s);
        start(&mut s, sd, "good", year);
        let expected_options = s.game.options;
        let expected_rng = s.game.lcg_x;
        let expected_score = score_text(&mut s, &buf);

        fresh(&mut s);
        start(&mut s, sd, "good", "2026");
        s.game.ship = ship;
        (s.game.brigcapacity, s.game.brigfree) = (capacity.parse().unwrap(), free.parse().unwrap());
        s.game.iscloaked = ship == 'E';
        s.game.isviolreported = true;
        (s.game.ncviol, s.game.kcaptured) = (2, 42);
        (s.game.score, s.game.perdate) = (9000.0, 2.5);
        let g = &s.game;
        let saved_fields = (g.ship, g.brigcapacity, g.brigfree, g.iscloaked, g.isviolreported, g.ncviol, g.kcaptured, g.score, g.perdate);

        let filename = path_str(&dir.join("mission.trk"));
        s.scanner.chew();
        s.scanner.inqueue = vec![filename.clone()];
        s.freeze(false);

        start(&mut s, sd, "good", year);
        assert!(s.game.ship == 'E' && s.game.alive);
        assert!(!s.game.iscloaked, "New mission inherited the cloak");
        assert!(!s.game.isviolreported);
        assert!(s.game.brigcapacity == 400 && s.game.brigfree == 400, "New Enterprise inherited the old brig");
        assert!(s.game.kcaptured == 0 && s.game.ncviol == 0, "New mission inherited score credits or penalties");
        assert!(s.game.score == 0.0 && s.game.perdate == 0.0);
        assert!(s.game.options == expected_options && s.game.cdebug);
        assert_eq!(s.game.lcg_x, expected_rng);
        assert_eq!(s.badpoints(), 0.0);
        assert_eq!(score_text(&mut s, &buf), expected_score);
        docking(&mut s, &buf);
        out += &format!("{ship} -> Enterprise {year}: uncloaked; empty 400-place brig; clean score; docking succeeds.\n");

        // The saved-game branch of setup must retain the resumed mission.
        s.scanner.chew();
        s.scanner.inqueue = vec!["saved".into(), filename.clone()];
        set_replay(&mut s, None);
        s.setup();
        assert!(s.game.thawed);
        let g = &s.game;
        let loaded = (g.ship, g.brigcapacity, g.brigfree, g.iscloaked, g.isviolreported, g.ncviol, g.kcaptured, g.score, g.perdate);
        assert_eq!(loaded, saved_fields, "Loading a saved game reset a field");
        out += &format!("Saved {ship}: cloak, brig, and scoring state preserved.\n");
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out, read("test76.chk"));
}

/// test77: Wayback restores features across games while keeping explicit
/// options and saved-game settings.
#[test]
fn wayback_restart_matches_test77() {
    let log = read("test77.log");
    let sd = log_seed(&log);
    let defaults = OPTION_ALL & !OPTION_CURSES;
    let years: Vec<String> = cases(&log);
    let (mut s, _buf) = new_sst();
    let fresh = |s: &mut Sst, options: i64| {
        s.game = Gamestate::default();
        s.thing = Thingy::default();
        s.set_options(options);
    };
    let options = |s: &mut Sst, command: &str| {
        s.scanner.chew();
        s.scanner.inqueue = command.split_whitespace().map(String::from).chain(["\n".to_string()]).collect();
        s.goptions();
    };
    let mut out = String::new();

    // Compare reused game state with an independently initialized game for each year.
    let mut expected = std::collections::HashMap::new();
    for year in &years {
        fresh(&mut s, defaults);
        start(&mut s, sd, "good", year);
        let g = &s.game;
        expected.insert(year.clone(), (g.options, g.state.nscrem, g.inplan, g.remkl(), g.lcg_x));
    }
    fresh(&mut s, defaults);
    for year in &years {
        start(&mut s, sd, "good", year);
        let g = &s.game;
        let actual = (g.options, g.state.nscrem, g.inplan, g.remkl(), g.lcg_x);
        assert_eq!(actual, expected[year], "Wayback {year} differs from a fresh game");
        assert_eq!(s.game.options & OPTION_IOMODES, OPTION_TTY);
        out += &format!("Wayback {year}: options, generated galaxy, and RNG match a fresh game.\n");
    }

    // A player preference that predates Wayback must not be restored automatically.
    fresh(&mut s, defaults & !OPTION_COLOR);
    start(&mut s, sd, "good", "1973");
    start(&mut s, sd, "good", "2026");
    assert_eq!(s.game.options, defaults & !OPTION_COLOR);
    assert!(s.game.options & OPTION_ALPHAMERIC != 0);
    out += "Initial color preference preserved while modern features return.\n";

    // Clearing an already hidden feature is still an explicit preference.
    fresh(&mut s, defaults);
    start(&mut s, sd, "good", "1973");
    options(&mut s, "clear color alphameric");
    start(&mut s, sd, "good", "2026");
    let without_display = defaults & !(OPTION_COLOR | OPTION_ALPHAMERIC);
    assert_eq!(s.game.options, without_display);
    options(&mut s, "set alphameric");
    start(&mut s, sd, "good", "1973");
    start(&mut s, sd, "good", "2026");
    assert_eq!(s.game.options, defaults & !OPTION_COLOR);
    out += "OPTIONS changes survive Wayback switches, including changes to hidden features.\n";

    let dir = scratch("test77");
    fresh(&mut s, defaults & !OPTION_COLOR);
    start(&mut s, sd, "good", "1973");
    let restricted = s.game.options;
    let filename = path_str(&dir.join("wayback.trk"));
    s.scanner.chew();
    s.scanner.inqueue = vec![filename.clone()];
    s.freeze(false);
    start(&mut s, sd, "good", "2026");
    s.scanner.chew();
    s.scanner.inqueue = vec!["saved".into(), filename.clone()];
    set_replay(&mut s, None);
    s.setup();
    assert!(s.game.thawed && s.game.options == restricted);
    start(&mut s, sd, "good", "2026");
    assert_eq!(s.game.options, defaults & !OPTION_COLOR);
    out += "Saved historical game keeps its rules and restores modern options for the next game.\n";

    // Older saves lack the suppression mask; their active choices remain
    // valid. The Python deletes the attribute before pickling; here the
    // key is left out of the saved JSON.
    options(&mut s, "clear alphameric");
    let legacy_options = s.game.options;
    s.scanner.inqueue = vec![filename.clone()];
    s.freeze(false);
    let path = dir.join("wayback.trk");
    let mut doc: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    doc["game"].as_object_mut().unwrap().remove("wayback_disabled").unwrap();
    std::fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
    fresh(&mut s, defaults);
    s.scanner.inqueue = vec!["saved".into(), filename.clone()];
    set_replay(&mut s, None);
    s.setup();
    assert!(s.game.thawed && s.game.options == legacy_options);
    start(&mut s, sd, "good", "2026");
    assert_eq!(s.game.options, legacy_options);
    out += "Legacy saves load and preserve their recorded option choices.\n";
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out, read("test77.chk"));
}

/// Python's `float(text)` for the accepted spellings of test79.log.
fn py_float_expected(text: &str) -> f64 {
    text.parse().unwrap()
}

/// Run the game as `sst -t -s SEED ARGS...` with `input` typed, the way
/// main() does, in this process. Returns the output and the exit code.
fn run_game(sd: i64, args: &[&str], input: &[&str]) -> (String, i32) {
    let (mut s, buf) = new_sst();
    s.io.input = Box::new(Lines(input.iter().map(|l| l.to_string()).collect()));
    s.set_options((OPTION_ALL & !OPTION_IOMODES) | OPTION_TTY);
    seed(&mut s, sd);
    for a in args {
        s.scanner.append(a);
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| crate::play(&mut s)));
    let code = match result {
        Ok(()) => 0,
        Err(payload) => match payload.downcast_ref::<SystemExit>() {
            Some(SystemExit(code)) => *code,
            None => std::panic::resume_unwind(payload),
        },
    };
    (take(&buf), code)
}

/// test79: nonfinite numbers are words, finite ones keep their value, and
/// the tournament prompt recovers from a nonfinite number. The Python runs
/// `../sst` as a subprocess; here the game runs in this process through
/// main's play loop, and "no stderr" is "no panic other than SystemExit".
#[test]
fn finite_input_matches_test79() {
    let log = read("test79.log");
    let sd = log_seed(&log);
    let (mut s, _buf) = new_sst();
    let (mut accept, mut reject) = (0, 0);
    for line in cases(&log) {
        let p: Vec<&str> = line.split_whitespace().collect();
        let (mode, token) = (p[0], p[1]);
        s.scanner.inqueue = vec![token.into(), "0.25".into(), "\n".into()];
        let kind = s.nexttok();
        if mode == "reject" {
            assert!(kind == Tok::Alpha && s.scanner.type_ == Tok::Alpha, "Accepted {token} as numeric");
            assert!(s.scanner.real.is_nan());
            assert_eq!(s.scanner.token, token.to_lowercase());
        } else {
            let want = py_float_expected(p[2]);
            assert!(kind == Tok::Real && s.scanner.type_ == Tok::Real);
            assert!(s.scanner.real.is_finite() && s.scanner.real == want);
            assert_eq!(s.scanner.real.is_sign_negative(), want.is_sign_negative());
        }
        assert!(s.nexttok() == Tok::Real && s.scanner.real == 0.25);
        assert_eq!(s.nexttok(), Tok::Eol);
        if mode == "reject" {
            reject += 1;
        } else {
            accept += 1;
        }
    }
    let mut out = String::new();
    out += &format!("{reject} nonfinite spellings rejected; following input remains readable.\n");
    out += &format!("{accept} finite values preserved, including signed zero and the float limit.\n");

    for token in ["nan", "inf", "1e999"] {
        let (stdout, code) =
            run_game(sd, &["tournament", token], &["1", "short good 2026", "report", "quit", "n", "n"]);
        assert_eq!(code, 0);
        assert!(stdout.contains("Beg your pardon, Captain?"));
        assert!(stdout.contains("This is tournament game 1."));
        assert!(stdout.contains("May the Great Bird of the Galaxy"));
    }
    out += "Tournament prompt rejects nonfinite IDs and accepts a replacement.\n";

    let (_, code) = run_game(sd, &["tournament", "nan"], &[]);
    assert_eq!(code, 0);
    out += "EOF at the tournament-number retry exits cleanly.\n";
    assert_eq!(out, read("test79.chk"));
}

/// test81: REQUEST item prefixes, live counts, Sandbox time, and REQUEST
/// refused in screen mode.
#[test]
fn request_matches_test81() {
    let (mut s, buf) = new_sst();
    s.set_options(OPTION_TTY | OPTION_ALPHAMERIC);
    let g = &mut s.game;
    g.alive = true;
    (g.quadrant, g.sector) = (Coord::new(1, 2), Coord::new(4, 5));
    (g.state.date, g.state.remtime) = (2456.7, 4.25);
    (g.warpfac, g.energy, g.torps) = (6.5, 1234.5, 7);
    (g.shield, g.inshld, g.shldup) = (1250.0, 2500.0, true);
    g.state.galaxy[0][0].klingons = 3;
    g.state.galaxy[2][2].klingons = 4;
    g.inbase = 5;
    g.state.baseq = vec![Coord::new(0, 0), Coord::new(2, 2), Coord::new(3, 3)];

    let request = |s: &mut Sst, tokens: &[&str]| {
        s.scanner.inqueue = tokens.iter().map(|t| t.to_string()).chain(["\n".to_string()]).collect();
        take(&buf);
        s.request();
        take(&buf)
    };
    let field = |label: &str, value: &str| format!("{label:<14}{value}\n");

    let mut out = String::new();
    let mut count = 0;
    for line in cases(&read("test81.log")) {
        let mut parts = line.splitn(3, ' ');
        let (name, shortest, expected) = (parts.next().unwrap(), parts.next().unwrap(), parts.next().unwrap());
        let (label, value) = expected.split_once('|').unwrap();
        for length in shortest.len()..=name.len() {
            for token in [name[..length].to_string(), name[..length].to_uppercase()] {
                let actual = request(&mut s, &[&token]);
                assert_eq!(actual, field(label, value), "{token}");
                count += 1;
            }
        }
    }
    out += &format!("{count} lowercase and uppercase REQUEST prefixes return the correct single field.\n");

    for token in ["b", "system", "datejunk", "stardate", "1", "?"] {
        let output = request(&mut s, &[token]);
        assert!(output.starts_with("UNRECOGNIZED REQUEST. Legal requests are:\n"), "{output}");
        assert!(output.contains("time, bases.") && !output.contains("system"));
        assert_eq!(request(&mut s, &["e"]), field("Energy", "1234.50"));
    }
    assert_eq!(request(&mut s, &["\n", "energy"]), "Information desired? ".to_string() + &field("Energy", "1234.50"));
    out += "Invalid items list the documented names; missing items prompt and input recovers.\n";

    s.game.state.remtime = 2.5;
    assert_eq!(request(&mut s, &["ti"]), field("Time Left", "2.50"));
    s.game.state.baseq.pop();
    assert_eq!(request(&mut s, &["ba"]), field("Bases Left", "2"));
    s.game.state.baseq.clear();
    assert_eq!(request(&mut s, &["bases"]), field("Bases Left", "0"));
    s.game.state.galaxy[0][0].klingons = 0;
    assert_eq!(request(&mut s, &["k"]), field("Klingons Left", "4"));
    s.game.sandbox = true;
    assert_eq!(request(&mut s, &["time"]), field("Time Left", "Unlimited"));
    assert_eq!(request(&mut s, &["date"]), field("Stardate", "2456.7"));
    out += "Time and enemy/base counts follow current state; Sandbox reports unlimited time.\n";

    // Command dispatch in screen mode: REQUEST and REQ are not commands
    // there. The Python stubs out `request` with a spy and checks it is
    // never called; here its output (the Energy field) must not appear.
    //
    // The Python's second case types the CURSES command in TTY mode to
    // switch to screen mode first. This port always runs SST2K's TTY mode
    // and its CURSES command does nothing by design (see makemoves), so
    // that case has no equivalent and is left out.
    s.set_options(OPTION_CURSES);
    s.game.alldone = false;
    s.io.input = Box::new(Lines(vec!["request energy".into(), "req ti".into(), "quit".into()]));
    set_replay(&mut s, None);
    take(&buf);
    s.makemoves();
    let output = take(&buf);
    assert!(!output.contains("Energy") && !output.contains("Time Left"), "{output}");
    assert_eq!(output.matches("Beg your pardon, Captain?").count(), 2, "{output}");
    assert!(s.game.options & OPTION_CURSES != 0);
    assert!(s.game.options & OPTION_TTY == 0);
    out += "Screen mode suppresses REQUEST and REQ, including after switching from TTY mode.\n";
    assert_eq!(out, read("test81.chk"));
}

/// test82: the winning citation, accepted, refused, or not offered.
#[test]
fn citation_matches_test82() {
    let dir = scratch("test82");
    let mut out = String::new();
    for line in cases(&read("test82.log")) {
        let p: Vec<&str> = line.split_whitespace().collect();
        let (case, skill, elapsed, alive, thawed, debug, answer, expected) = (p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7]);
        let (mut s, buf) = new_sst();
        s.set_options(OPTION_TTY);
        s.game.skill = match skill {
            "novice" => SKILL_NOVICE,
            "fair" => SKILL_FAIR,
            "good" => SKILL_GOOD,
            "expert" => SKILL_EXPERT,
            _ => SKILL_EMERITUS,
        };
        s.game.ship = 'E';
        (s.game.alive, s.game.thawed, s.game.idebug) = (alive == "1", thawed == "1", debug == "1");
        s.game.indate = 1000.0;
        s.game.state.date = s.game.indate + elapsed.parse::<f64>().unwrap();
        // All 20 initial enemies have been destroyed; no penalties incurred.
        (s.game.inkling, s.game.incom, s.game.inscom) = (17, 2, 1);
        let filename = dir.join(format!("{case}.txt"));
        let captain = format!("Captain {case}");
        let replay = format!("{answer}\n{}\n{captain}\n", filename.display());
        set_replay(&mut s, Some(&replay));
        s.finish(FWON);
        let transcript = take(&buf);
        let prompted = "Do you want your Commodore Emeritus Citation printed?";
        assert!(s.game.gamewon && s.game.alldone, "{case}");
        assert_eq!(transcript.contains(prompted), expected == "printed" || expected == "declined", "{case}");
        assert_eq!(transcript.contains("You cannot get a citation, so..."), expected == "blocked", "{case}");
        assert_eq!(filename.exists(), expected == "printed", "{case}");
        assert_eq!(transcript.contains("File or device name for your plaque:"), expected == "printed", "{case}");

        if ["printed", "declined", "blocked"].contains(&expected) {
            let promotion = if skill == "expert" {
                "promotes you to Commodore Emeritus."
            } else {
                "Now you can retire and write your own Star Trek game!"
            };
            assert!(transcript.contains(promotion), "{case}");
            if expected != "blocked" {
                assert!(transcript.find(promotion) < transcript.find(prompted), "{case}");
                assert!(transcript.find(prompted) < transcript.find("Your score --"), "{case}");
            }
        }
        if expected == "printed" {
            let plaque = std::fs::read_to_string(&filename).unwrap();
            assert!(plaque.contains(&captain) && plaque.contains("Commodore Emeritus"), "{case}");
            assert!(plaque.contains(if skill == "expert" { "Expert level" } else { "Emeritus level" }), "{case}");
            assert!(plaque.contains(&format!("Your score:  {}", s.game.score.trunc() as i64)), "{case}");
            assert!(plaque.contains(&format!("Klingons per stardate:  {:.2}", s.game.perdate)), "{case}");
            assert!(transcript.find("Your score --") < transcript.find("File or device name"), "{case}");
            assert_eq!(replay_rest(&mut s), "", "{case}");
        } else if expected == "declined" {
            assert_eq!(replay_rest(&mut s), format!("{}\n{captain}\n", filename.display()), "{case}");
        } else {
            assert_eq!(replay_rest(&mut s), replay, "{case}");
        }
        out += &format!("{case}: citation {expected}.\n");
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(out, read("test82.chk"));
}
