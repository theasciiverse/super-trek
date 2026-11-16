//! Replays ESR's recorded SST2K games (tests/sst2k/) through the port and
//! compares the output with SST2K's own, byte for byte.
//!
//! - testNN.log + testNN.chk: run `super-trek -r testNN.log`.
//! - testNN.tst that runs the game (with flags, or from a small Python
//!   driver that starts it as a subprocess): run the script with the port
//!   in place of `../sst`.
//! - testNN.tst that loads SST2K's internals into Python (`exec(compile`):
//!   not runnable here; those are ported as Rust unit tests.
//!
//! It is a ratchet: every game listed in tests/passing.txt must match, and
//! the rest are reported, so progress is locked in as the translation
//! advances. SUPER_TREK_REPLAY=testNN runs just that one and prints a diff.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_super-trek");

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sst2k")
}

enum Case {
    Log(String),
    Script(String),
    Unit(String),
}

fn cases() -> Vec<Case> {
    let mut names: Vec<String> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|n| n.strip_suffix(".chk").map(String::from))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|n| match std::fs::read_to_string(dir().join(format!("{n}.tst"))) {
            Err(_) => Case::Log(n),
            Ok(s) if s.contains("exec(compile") => Case::Unit(n),
            Ok(_) => Case::Script(n),
        })
        .collect()
}

fn cache_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("super-trek-test-cache-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&d);
    d
}

fn run(case: &Case) -> Option<(String, Vec<u8>)> {
    let cache = cache_dir();
    let out = match case {
        Case::Log(n) => Command::new(BIN)
            .current_dir(dir())
            .env("SUPER_TREK_FAST", "1")
            .env("XDG_CACHE_HOME", &cache)
            .args(["-r", &format!("{n}.log")])
            .output()
            .ok()?,
        Case::Script(n) => {
            let script = std::fs::read_to_string(dir().join(format!("{n}.tst"))).ok()?;
            let bin = format!("'{BIN}'");
            let script = script
                .replace("sys.executable, '-u', '../sst'", &bin)
                .replace("'../sst'", &bin)
                .replace("python3 -u ../sst", BIN)
                .replace("python3 ../sst", BIN);
            Command::new("sh")
                .current_dir(dir())
                .env("SUPER_TREK_FAST", "1")
                .env("XDG_CACHE_HOME", &cache)
                .args(["-c", &script])
                .output()
                .ok()?
        }
        Case::Unit(_) => return None,
    };
    let name = match case {
        Case::Log(n) | Case::Script(n) | Case::Unit(n) => n.clone(),
    };
    let mut bytes = out.stdout;
    if matches!(case, Case::Log(_)) {
        // the Makefile captures `../sst -r x.log >x.chk 2>&1`
        bytes.extend(out.stderr);
    }
    Some((name, bytes))
}

#[test]
fn recorded_games() {
    let passing: Vec<String> = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/passing.txt"))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(String::from)
        .collect();
    let only = std::env::var("SUPER_TREK_REPLAY").ok();
    let (mut ok, mut failing, mut total, mut units, mut regressions) = (Vec::new(), Vec::new(), 0, 0, Vec::new());
    for case in cases() {
        if let Some(o) = &only {
            let n = match &case {
                Case::Log(n) | Case::Script(n) | Case::Unit(n) => n,
            };
            if n != o {
                continue;
            }
        }
        let Some((name, got)) = run(&case) else {
            units += 1;
            continue;
        };
        total += 1;
        let want = std::fs::read(dir().join(format!("{name}.chk"))).unwrap();
        if got == want {
            ok.push(name);
        } else {
            if only.is_some() {
                let p = std::env::temp_dir().join(format!("{name}.got"));
                std::fs::write(&p, &got).unwrap();
                println!("output written to {}", p.display());
            }
            if passing.contains(&name) {
                regressions.push(name.clone());
            }
            failing.push(name);
        }
    }
    let _ = std::fs::remove_dir_all(cache_dir());
    println!("{} of {total} replayable recorded games match ({units} Python unit tests ported separately)", ok.len());
    println!("failing: {failing:?}");
    let new: Vec<_> = ok.iter().filter(|n| !passing.contains(n)).collect();
    if !new.is_empty() {
        println!("newly matching, add to tests/passing.txt: {new:?}");
    }
    assert!(regressions.is_empty(), "recorded games that matched before no longer do: {regressions:?}");
}
