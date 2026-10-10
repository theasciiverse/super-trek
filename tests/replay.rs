//! Replays ESR's recorded SST2K games (tests/sst2k/) through the port and
//! compares the output with SST2K's own, byte for byte.
//!
//! - testNN.log + testNN.chk: run `super-trek -r testNN.log`.
//! - testNN.tst that runs the game with flags: run the script with the port
//!   in place of `../sst`.
//! - testNN.tst that is a Python unit test of SST2K internals: not runnable
//!   here; those are ported as Rust unit tests next to the code they test.
//!
//! It is a ratchet: every game listed in tests/passing.txt must match, and the
//! rest are reported, so progress is locked in as the translation advances.

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_super-trek");

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/sst2k")
}

enum Case {
    Log(String),
    Script(String),
    Python(String),
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
            Ok(s) if s.contains("<<'PY'") || s.contains("import ") => Case::Python(n),
            Ok(_) => Case::Script(n),
        })
        .collect()
}

fn run(case: &Case) -> Option<(String, Vec<u8>)> {
    let out = match case {
        Case::Log(n) => Command::new(BIN).current_dir(dir()).args(["-r", &format!("{n}.log")]).output().ok()?,
        Case::Script(n) => {
            let script = std::fs::read_to_string(dir().join(format!("{n}.tst"))).ok()?;
            let script = script.replace("python3 -u ../sst", BIN).replace("python3 ../sst", BIN).replace("../sst", BIN);
            Command::new("sh").current_dir(dir()).args(["-c", &script]).output().ok()?
        }
        Case::Python(_) => return None,
    };
    let name = match case {
        Case::Log(n) | Case::Script(n) | Case::Python(n) => n.clone(),
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
    let (mut ok, mut total, mut python, mut regressions) = (Vec::new(), 0, 0, Vec::new());
    for case in cases() {
        let Some((name, got)) = run(&case) else {
            python += 1;
            continue;
        };
        total += 1;
        let want = std::fs::read(dir().join(format!("{name}.chk"))).unwrap();
        if got == want {
            ok.push(name);
        } else if passing.contains(&name) {
            regressions.push(name);
        }
    }
    println!("{} of {total} replayable recorded games match ({python} Python unit tests ported separately)", ok.len());
    let new: Vec<_> = ok.iter().filter(|n| !passing.contains(n)).collect();
    if !new.is_empty() {
        println!("newly matching, add to tests/passing.txt: {new:?}");
    }
    assert!(regressions.is_empty(), "recorded games that matched before no longer do: {regressions:?}");
}
