//! Saved games: freeze and thaw (`sst` 6458-6807), in this port's own versioned JSON format.
//!
//! SST2K pickles its `game` object. This port does not read or write
//! pickles: it saves `{"format": "super-trek-save", "version": 1, "game": ...}`
//! with serde, and checks a loaded game the way SST2K's `validate_game()`
//! does (plus the index checks Rust needs), so a bad file is refused with
//! SST2K's "invalid or unsafe saved game" message instead of crashing.
//!
//! A bare file name is kept in the save folder ($XDG_STATE_HOME/super-trek,
//! else ~/.local/state/super-trek; %LOCALAPPDATA%\super-trek\saves on
//! Windows); a name with a directory part is used as given.

use crate::consts::*;
use crate::coord::{Coord, NONE};
use crate::finish::SHIP_NONE;
use crate::pf;
use crate::scanner::Tok;
use crate::sst::Sst;
use crate::state::*;
use serde_json::Value;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

const SAVE_FORMAT: &str = "super-trek-save";
const SAVE_VERSION: i64 = 1;
/// `SAVE_MAX_BYTES`
const SAVE_MAX_BYTES: u64 = 1024 * 1024;

/// Fields that hold NaN for Python's None; JSON writes them as null.
const POOL_NAN_FIELDS: [&str; 3] = ["kdist", "kavgd", "power"];

impl Sst {
    /// `freeze(boss)`: save game.
    pub fn freeze(&mut self, boss: bool) {
        if boss {
            self.scanner.push("emsave.trk");
        }
        let mut key = self.nexttok();
        if key == Tok::Eol {
            self.proutn("File name: ");
            key = self.nexttok();
        }
        if key != Tok::Alpha {
            self.huh();
            return;
        }
        if !self.scanner.token.contains('.') {
            self.scanner.token.push_str(".trk");
        }
        let name = self.scanner.token.clone();
        let path = save_path(&name, true);
        let data = encode_game(&self.game);
        let ok = match std::fs::File::create(&path) {
            Ok(mut fp) => {
                // DEVIATION: Python crashes if writing the opened file fails;
                // here that is reported like a failed open.
                fp.write_all(&data).is_ok()
            }
            Err(_) => false,
        };
        if !ok {
            let s = pf!("Can't freeze game as file %s", name);
            self.prout(&s);
            return;
        }
        log::debug!(target: "save", "froze game to {}", path.display());
        self.scanner.chew();
    }

    /// `thaw()`: retrieve saved game. True on failure.
    pub fn thaw(&mut self) -> bool {
        let mut key = self.nexttok();
        if key == Tok::Eol {
            self.proutn("File name: ");
            key = self.nexttok();
        }
        if key != Tok::Alpha {
            self.huh();
            return true;
        }
        if !self.scanner.token.contains('.') {
            self.scanner.token.push_str(".trk");
        }
        let name = self.scanner.token.clone();
        let path = save_path(&name, false);
        let data = match read_saved(&path) {
            Ok(d) => d,
            Err(_) => {
                let s = pf!("Can't thaw game in %s", name);
                self.prout(&s);
                return true;
            }
        };
        let restored = match decode_game(&data) {
            Ok(g) => g,
            Err(why) => {
                log::debug!(target: "save", "refused {}: {why}", path.display());
                let s = pf!("Can't thaw game in %s: invalid or unsafe saved game", name);
                self.prout(&s);
                self.scanner.chew();
                return true;
            }
        };
        self.game = restored;
        self.sync_options();
        self.scanner.chew();
        false
    }
}

/// The folder bare save names go in.
fn save_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("XDG_STATE_HOME")
        && !d.is_empty()
        && Path::new(&d).is_absolute()
    {
        return Some(PathBuf::from(d).join("super-trek"));
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA").filter(|d| !d.is_empty())?;
        Some(PathBuf::from(base).join("super-trek").join("saves"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME").filter(|d| !d.is_empty())?;
        Some(PathBuf::from(home).join(".local").join("state").join("super-trek"))
    }
}

/// Where a typed save name lives: a bare file name goes in the save folder
/// (created when `create` is set), anything with a directory part is used
/// as given.
fn save_path(name: &str, create: bool) -> PathBuf {
    let p = Path::new(name);
    let mut comps = p.components();
    let bare = matches!((comps.next(), comps.next()), (Some(Component::Normal(_)), None));
    if !bare {
        return p.to_path_buf();
    }
    match save_dir() {
        Some(dir) => {
            if create {
                let _ = std::fs::create_dir_all(&dir);
            }
            dir.join(name)
        }
        None => p.to_path_buf(),
    }
}

/// The file's bytes, read up to one byte past SAVE_MAX_BYTES (as
/// `read_saved_pickle` does) so decode_game can refuse an oversized file.
fn read_saved(path: &Path) -> std::io::Result<Vec<u8>> {
    let fp = std::fs::File::open(path)?;
    let mut data = Vec::new();
    fp.take(SAVE_MAX_BYTES + 1).read_to_end(&mut data)?;
    Ok(data)
}

/// The save file's bytes for a game.
fn encode_game(game: &Gamestate) -> Vec<u8> {
    let game = compact_pool(game);
    // NaN floats (Python's None) become null here; decode_game restores them.
    let doc = serde_json::json!({
        "format": SAVE_FORMAT,
        "version": SAVE_VERSION,
        "game": serde_json::to_value(&game).unwrap_or(Value::Null),
    });
    let mut out = serde_json::to_vec(&doc).unwrap_or_default();
    out.push(b'\n');
    out
}

/// Keep only the enemies the game can still reach (the live list and the
/// Tholian), renumbered in order. Pickle saves exactly the reachable
/// objects, so nothing observable changes, and saves stay small however
/// long the game has run.
fn compact_pool(game: &Gamestate) -> Gamestate {
    let mut g = game.clone();
    let mut map: Vec<Option<usize>> = vec![None; game.pool.len()];
    let mut pool = Vec::new();
    let reachable = game.enemies.iter().copied().chain(game.tholian);
    for id in reachable {
        if id < map.len() && map[id].is_none() {
            map[id] = Some(pool.len());
            pool.push(game.pool[id].clone());
        }
    }
    g.enemies = game.enemies.iter().map(|&id| map.get(id).copied().flatten().unwrap_or(id)).collect();
    g.tholian = game.tholian.map(|id| map.get(id).copied().flatten().unwrap_or(id));
    g.pool = pool;
    g
}

/// Parse and check a save file; Err names the reason (for the log only).
fn decode_game(data: &[u8]) -> Result<Gamestate, String> {
    if data.len() as u64 > SAVE_MAX_BYTES {
        return Err("saved game is too large".into());
    }
    let mut doc: Value = serde_json::from_slice(data).map_err(|e| e.to_string())?;
    if doc.get("format").and_then(Value::as_str) != Some(SAVE_FORMAT) {
        return Err("not a Super Trek save".into());
    }
    if doc.get("version").and_then(Value::as_i64) != Some(SAVE_VERSION) {
        return Err("unknown save version".into());
    }
    let mut g = doc.get_mut("game").map(Value::take).ok_or("no game")?;
    // null in a NaN-able float field stands for NaN
    let mut pool_nans: Vec<(usize, &str)> = Vec::new();
    let mut event_nans: Vec<usize> = Vec::new();
    if let Some(pool) = g.get_mut("pool").and_then(Value::as_array_mut) {
        for (k, e) in pool.iter_mut().enumerate() {
            for f in POOL_NAN_FIELDS {
                if let Some(v) = e.get_mut(f)
                    && v.is_null()
                {
                    *v = Value::from(0.0);
                    pool_nans.push((k, f));
                }
            }
        }
    }
    if let Some(future) = g.get_mut("future").and_then(Value::as_array_mut) {
        for (k, ev) in future.iter_mut().enumerate() {
            if let Some(v) = ev.get_mut("date")
                && v.is_null()
            {
                *v = Value::from(0.0);
                event_nans.push(k);
            }
        }
    }
    let mut game: Gamestate = serde_json::from_value(g).map_err(|e| e.to_string())?;
    for (k, f) in pool_nans {
        let e = &mut game.pool[k];
        match f {
            "kdist" => e.kdist = f64::NAN,
            "kavgd" => e.kavgd = f64::NAN,
            _ => e.power = f64::NAN,
        }
    }
    for k in event_nans {
        game.future[k].date = f64::NAN;
    }
    validate_game(&game)?;
    Ok(game)
}

/// `coordinate(value, size, optional)` of `validate_game()`.
fn coordinate(c: Coord, size: i64, optional: bool) -> bool {
    if c.i == NONE && c.j == NONE {
        return optional;
    }
    [c.i, c.j].iter().all(|&v| v != NONE && 0 <= v && v < size)
}

/// `validate_game(saved)`: check dimensions, coordinates and indices before
/// the saved state can be used by the game.
fn validate_game(g: &Gamestate) -> Result<(), String> {
    let bad = |why: &str| Err(why.to_string());
    // Python's schema check: passwd is a str
    if g.passwd.is_none() {
        return bad("Invalid fields for Gamestate");
    }
    if !(SKILL_NOVICE..=SKILL_EMERITUS).contains(&g.skill)
        || ![1, 2, 4].contains(&g.length)
        || !['E', 'F', SHIP_NONE].contains(&g.ship)
        || !["green", "yellow", "red", "docked", "dead"].contains(&g.condition.as_str())
        || !coordinate(g.quadrant, GALSIZE, false)
        || !coordinate(g.sector, QUADSIZE, false)
        || g.future.len() != NEVENTS
        || g.inshld <= 0.0
        || !(1.0 <= g.warpfac && g.warpfac <= 10.0)
    {
        return bad("Invalid ship state");
    }
    if g.quad.iter().flatten().any(|&s| !". EFKCRST?BP@*#X".contains(s)) {
        return bad("Invalid sector map");
    }
    if ![g.base, g.plnet].iter().all(|&c| coordinate(c, QUADSIZE, true)) {
        return bad("Invalid local coordinates");
    }
    for snapshot in [&g.state, &g.snapsht] {
        let matrix = |n: usize, rows: &[usize]| n == GS && rows.iter().all(|&r| r == GS);
        let grows: Vec<usize> = snapshot.galaxy.iter().map(Vec::len).collect();
        let crows: Vec<usize> = snapshot.chart.iter().map(Vec::len).collect();
        if !matrix(snapshot.galaxy.len(), &grows) || !matrix(snapshot.chart.len(), &crows) {
            return bad("Invalid galaxy dimensions");
        }
        if !coordinate(snapshot.kscmdr, GALSIZE, true)
            || !snapshot.baseq.iter().chain(&snapshot.kcmdr).all(|&c| coordinate(c, GALSIZE, false))
            || !snapshot.planets.iter().all(|p| coordinate(p.quadrant, GALSIZE, false))
        {
            return bad("Invalid galaxy coordinates");
        }
        if snapshot.galaxy.iter().flatten().any(|q| q.klingons < 0 || q.romulans < 0 || q.stars < 0) {
            return bad("Negative quadrant count");
        }
        // Python shares Planet objects; here they are indices
        if snapshot.galaxy.iter().flatten().any(|q| q.planet.is_some_and(|k| k >= snapshot.planets.len())) {
            return bad("Invalid planet index");
        }
    }
    if g.iplnet.is_some_and(|k| k >= g.state.planets.len()) {
        return bad("Invalid planet index");
    }
    if !coordinate(g.battle, GALSIZE, true) {
        return bad("Invalid battle coordinates");
    }
    if !g.future.iter().all(|ev| coordinate(ev.quadrant, GALSIZE, true)) {
        return bad("Invalid event coordinates");
    }
    // Python shares Enemy objects; here they are ids into the pool
    for &id in g.enemies.iter().chain(g.tholian.iter()) {
        let Some(enemy) = g.pool.get(id) else {
            return bad("Invalid enemy id");
        };
        if !['K', 'C', 'S', 'R', 'T', '?'].contains(&enemy.etype) || !coordinate(enemy.location, QUADSIZE, true) {
            return bad("Invalid enemy");
        }
    }
    // A probe in flight is read every turn; SST2K would crash on a missing
    // one or one outside the galaxy.
    if g.future[FDSPROB].date != FOREVER && !g.future[FDSPROB].date.is_nan() {
        match &g.probe {
            Some(p) if p.location.quadrant().valid_quadrant() => {}
            _ => return bad("Invalid probe"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::{Input, Output};
    use std::sync::{Arc, Mutex};

    struct Capture(Arc<Mutex<String>>);
    impl Output for Capture {
        fn write(&mut self, s: &str) {
            self.0.lock().unwrap().push_str(s);
        }
    }
    struct Lines(Vec<String>);
    impl Input for Lines {
        fn read_line(&mut self) -> Option<String> {
            if self.0.is_empty() { None } else { Some(self.0.remove(0)) }
        }
    }

    fn quiet(lines: &[&str]) -> (Sst, Arc<Mutex<String>>) {
        let buf = Arc::new(Mutex::new(String::new()));
        let mut s = Sst::new();
        s.io.out = Box::new(Capture(buf.clone()));
        s.io.input = Box::new(Lines(lines.iter().map(|l| l.to_string()).collect()));
        s.io.slow = false;
        (s, buf)
    }

    fn scratch(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("super-trek-save-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// A set-up game, plus a Tholian that has left the quadrant (NaN
    /// distances, still reachable as `game.tholian`) and an unreachable
    /// pool entry that the save leaves out.
    fn set_up_game() -> Sst {
        let (mut s, _) = quiet(&["regular short good 1980", "secret"]);
        s.rnd_seed_int(1234);
        s.setup();
        s.game.probe = Some(Course::new(3.0, 2.5, s.game.quadrant.to_vec2().scale(10.0)));
        s.game.future[FTBEAM].quadrant = Coord::new(2, 3);
        let gone = s.new_enemy('K', None, 200.0);
        s.enemy_move(gone, None);
        let th = s.new_enemy('T', None, 500.0);
        s.enemy_move(th, None);
        s.game.tholian = Some(th);
        s
    }

    #[test]
    fn freeze_thaw_round_trip() {
        let mut s = set_up_game();
        let dir = scratch("roundtrip");
        let file = dir.join("game.trk");
        std::fs::write(&file, encode_game(&s.game)).unwrap();
        let data = read_saved(&file).unwrap();
        let back = decode_game(&data).expect("valid save");
        // the reachable state is the same; serialized forms compare equal
        let a = serde_json::to_string(&compact_pool(&s.game)).unwrap();
        let b = serde_json::to_string(&back).unwrap();
        assert_eq!(a, b);
        assert_eq!(back.lcg_x, s.game.lcg_x);
        let th = &back.pool[back.tholian.unwrap()];
        assert!(th.etype == 'T' && th.kdist.is_nan() && th.kavgd.is_nan() && th.power == 500.0);
        assert_eq!(back.pool.len(), s.game.enemies.len() + 1);
        assert_eq!(back.passwd.as_deref(), Some("secret"));
        let nans = |g: &Gamestate| g.future.iter().map(|e| e.date.is_nan()).collect::<Vec<_>>();
        assert_eq!(nans(&back), nans(&s.game));
        assert_eq!(back.probe.as_ref().map(|p| p.angle), s.game.probe.as_ref().map(|p| p.angle));

        // and through the commands, with a name that has a directory part
        let plain = |p: &str| p == p.to_lowercase() && !p.contains('.') && !p.contains(char::is_whitespace);
        if plain(&dir.to_string_lossy()) {
            let stem = dir.join("viacmd");
            let typed = stem.to_string_lossy().into_owned();
            s.scanner.inqueue = vec![typed.clone(), "\n".into()];
            s.freeze(false);
            assert!(dir.join("viacmd.trk").exists());
            let (mut t, out) = quiet(&[]);
            t.scanner.inqueue = vec![typed.clone() + ".trk", "\n".into()];
            assert!(!t.thaw());
            assert_eq!(serde_json::to_string(&t.game).unwrap(), a);
            assert!(t.scanner.inqueue.is_empty());
            assert_eq!(out.lock().unwrap().as_str(), "");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrupt_saves_are_refused() {
        let s = set_up_game();
        let good = encode_game(&s.game);
        assert!(decode_game(&good).is_ok());
        let mut doc: Value = serde_json::from_slice(&good).unwrap();

        let refuse = |v: &Value| decode_game(&serde_json::to_vec(v).unwrap()).is_err();
        // not JSON, truncated, wrong format, wrong version
        assert!(decode_game(b"\x80\x04pickle").is_err());
        assert!(decode_game(&good[..good.len() / 2]).is_err());
        let mut v = doc.clone();
        v["format"] = "pickle".into();
        assert!(refuse(&v));
        let mut v = doc.clone();
        v["version"] = 2.into();
        assert!(refuse(&v));
        // a sector out of range, a bad map symbol, a bad skill
        let mut v = doc.clone();
        v["game"]["sector"]["i"] = 10.into();
        assert!(refuse(&v));
        let mut v = doc.clone();
        v["game"]["quad"][0][0] = "Z".into();
        assert!(refuse(&v));
        let mut v = doc.clone();
        v["game"]["skill"] = 9.into();
        assert!(refuse(&v));
        // an enemy id outside the pool, a planet index outside the list
        let mut v = doc.clone();
        v["game"]["enemies"] = serde_json::json!([999]);
        assert!(refuse(&v));
        let mut v = doc.clone();
        v["game"]["state"]["galaxy"][0][0]["planet"] = 999.into();
        assert!(refuse(&v));
        // a missing galaxy row, a negative count
        let mut v = doc.clone();
        v["game"]["snapsht"]["galaxy"].as_array_mut().unwrap().pop();
        assert!(refuse(&v));
        let mut v = doc.clone();
        v["game"]["state"]["galaxy"][1][1]["klingons"] = (-1).into();
        assert!(refuse(&v));
        // too large
        let mut big = good.clone();
        big.resize(SAVE_MAX_BYTES as usize + 1, b' ');
        assert!(decode_game(&big).is_err());

        // through thaw: the message, the scanner chewed, game untouched
        let dir = scratch("corrupt");
        let file = dir.join("bad.trk");
        doc["game"]["quadrant"]["j"] = 8.into();
        std::fs::write(&file, serde_json::to_vec(&doc).unwrap()).unwrap();
        let typed = file.to_string_lossy().into_owned();
        let d = dir.to_string_lossy();
        if d == d.to_lowercase() && !d.contains('.') && !d.contains(char::is_whitespace) {
            let (mut t, out) = quiet(&[]);
            t.scanner.inqueue = vec![typed.clone(), "extra".into(), "\n".into()];
            assert!(t.thaw());
            assert_eq!(*out.lock().unwrap(), format!("Can't thaw game in {typed}: invalid or unsafe saved game\n"));
            assert!(t.scanner.inqueue.is_empty());
            assert!(t.game.passwd.is_none());
            // a missing file: the plain message, no chew
            let (mut t, out) = quiet(&[]);
            let missing = dir.join("nothere").to_string_lossy().into_owned();
            t.scanner.inqueue = vec![missing.clone(), "extra".into(), "\n".into()];
            assert!(t.thaw());
            assert_eq!(*out.lock().unwrap(), format!("Can't thaw game in {missing}.trk\n"));
            assert_eq!(t.scanner.inqueue.len(), 2);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bare_names_go_to_the_save_folder() {
        assert_eq!(save_path("a/b.trk", false), PathBuf::from("a/b.trk"));
        assert_eq!(save_path("../b.trk", false), PathBuf::from("../b.trk"));
        if let Some(d) = save_dir() {
            assert_eq!(save_path("emsave.trk", false), d.join("emsave.trk"));
        }
    }
}
