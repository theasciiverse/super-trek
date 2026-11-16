//! Super Trek: Super Star Trek ported from ESR's SST2K. See CLAUDE.md and NOTICE.

mod ai;
mod battle;
mod battle_player;
mod commands;
mod consts;
mod coord;
mod events;
mod finish;
mod io;
mod moving;
mod nova;
mod planets;
mod py;
mod reports;
mod rescue;
mod rng;
mod save;
mod scanner;
mod setup;
mod sst;
mod state;
mod ui;
#[cfg(test)]
mod unit_tests;

use consts::*;
use sst::{Sst, SystemExit};
use std::io::{IsTerminal, Write};

/// Python's `getopt.getopt(args, "cr:s:txV")`.
fn getopt(args: &[String]) -> Result<(Vec<(char, String)>, Vec<String>), String> {
    let mut opts = Vec::new();
    let mut k = 0;
    while k < args.len() && args[k].starts_with('-') && args[k] != "-" {
        if args[k] == "--" {
            k += 1;
            break;
        }
        if let Some(long) = args[k].strip_prefix("--") {
            let name = long.split('=').next().unwrap_or("");
            return Err(format!("option --{name} not recognized"));
        }
        let s: Vec<char> = args[k][1..].chars().collect();
        k += 1;
        let mut p = 0;
        while p < s.len() {
            let c = s[p];
            p += 1;
            match c {
                'c' | 't' | 'x' | 'V' => opts.push((c, String::new())),
                'r' | 's' => {
                    let val = if p < s.len() {
                        let v: String = s[p..].iter().collect();
                        p = s.len();
                        v
                    } else if k < args.len() {
                        k += 1;
                        args[k - 1].clone()
                    } else {
                        return Err(format!("option -{c} requires argument"));
                    };
                    opts.push((c, val));
                }
                _ => return Err(format!("option -{c} not recognized")),
            }
        }
    }
    Ok((opts, args[k..].to_vec()))
}

fn input_log_path() -> Option<std::path::PathBuf> {
    let base = match std::env::var_os("XDG_CACHE_HOME") {
        Some(d) if !d.is_empty() => std::path::PathBuf::from(d),
        _ => {
            #[cfg(windows)]
            let home = std::env::var_os("LOCALAPPDATA").map(std::path::PathBuf::from)?;
            #[cfg(not(windows))]
            let home = std::path::PathBuf::from(std::env::var_os("HOME")?).join(".cache");
            home
        }
    };
    let dir = base.join("super-trek");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join("input.log"))
}

fn exit_with(sst: &mut Sst, code: i32) -> ! {
    sst.io.out.flush();
    if let Some(f) = sst.io.logfp.as_mut() {
        let _ = f.flush();
    }
    std::process::exit(code);
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("off")).format_timestamp_millis().init();
    let mut argv: Vec<String> = std::env::args().skip(1).collect();
    // `--tty` (this port's own switch) keeps the plain line mode
    let mut line_mode = false;
    argv.retain(|a| {
        if a == "--tty" {
            line_mode = true;
            false
        } else {
            true
        }
    });
    let mut sst = Sst::new();
    sst.io.rng_trace = std::env::var_os("SUPER_TREK_RNG_TRACE").is_some();
    sst.set_options((OPTION_ALL & !OPTION_IOMODES) | OPTION_TTY);
    let mut seed_mod: i64 = (std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        % rng::LCG_M as u64) as i64;
    let mut seed_text = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_default();
    let (opts, mut arguments) = match getopt(&argv) {
        Ok(v) => v,
        Err(e) => {
            println!("{e}");
            std::process::exit(1);
        }
    };
    for (switch, val) in opts {
        match switch {
            'r' => {
                let f = match std::fs::File::open(&val) {
                    Ok(f) => f,
                    Err(_) => {
                        sst.io.out.flush();
                        eprint!("sst: can't open replay file {val}\n");
                        std::process::exit(1);
                    }
                };
                let mut reader = io::LineReader::new(Box::new(f) as Box<dyn std::io::Read + Send>);
                let ok = loop {
                    let line = reader.readline();
                    if line.is_empty() {
                        break false;
                    }
                    let line = line.trim_matches(py::py_isspace).to_string();
                    sst.io.out.write(&format!("{line}\n"));
                    if line == "#" {
                        break true;
                    }
                    if line.starts_with("# seed") {
                        let parts = py::py_split(&line);
                        if parts.len() != 3 {
                            break false;
                        }
                        match py::py_int_mod(&parts[2], rng::LCG_M as i64) {
                            Some(v) => {
                                seed_mod = v;
                                seed_text = parts[2].clone();
                            }
                            None => break false,
                        }
                    } else if line.starts_with("# arguments") {
                        arguments.extend(py::py_split(&line).into_iter().skip(2));
                    }
                };
                if !ok {
                    sst.io.out.flush();
                    eprint!("sst: replay file {val} is ill-formed\n");
                    std::process::exit(1);
                }
                sst.io.replay = Some(reader);
                sst.io.replay_started = true;
                sst.io.replay_open = true;
                line_mode = true;
                let o = (sst.game.options | OPTION_TTY) & !OPTION_CURSES;
                sst.set_options(o);
            }
            's' => match py::py_int_mod(&val, rng::LCG_M as i64) {
                Some(v) => {
                    seed_mod = v;
                    seed_text = val.trim().to_string();
                }
                None => {
                    eprintln!("sst: invalid seed {val}");
                    std::process::exit(1);
                }
            },
            't' => {
                line_mode = true;
                let o = (sst.game.options | OPTION_TTY) & !OPTION_CURSES;
                sst.set_options(o);
            }
            'x' => sst.game.idebug = true,
            'c' => sst.game.cdebug = true,
            'V' => {
                println!("Super Trek {} (SST2K {})", env!("CARGO_PKG_VERSION"), VERSION);
                std::process::exit(0);
            }
            _ => unreachable!(),
        }
    }
    // where to save the input in case of bugs
    if let Some(path) = input_log_path()
        && let Ok(mut f) = std::fs::File::create(&path)
    {
        let user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_else(|_| "player".into());
        let _ = write!(
            f,
            "# seed {seed_text}\n# arguments {}\n# SST2K version {VERSION}\n# recorded by {user} with Super Trek {}\n#\n",
            arguments.join(" "),
            env!("CARGO_PKG_VERSION")
        );
        sst.io.logfp = Some(f);
    }
    sst.rnd_seed_int(seed_mod);
    log::info!(target: "start", "seed {seed_text} arguments {arguments:?}");
    for a in &arguments {
        sst.scanner.append(a);
    }
    // iostart(): rows come from stdout only, so pipes never page
    if std::io::stdout().is_terminal()
        && let Some(r) = terminal_rows()
    {
        sst.io.rows = r;
    }
    if !line_mode && std::io::stdin().is_terminal() && std::io::stdout().is_terminal() {
        let (cols, rows) = ratatui::crossterm::terminal::size().unwrap_or((0, 0));
        if cols >= 80 && rows >= 24 {
            log::info!(target: "start", "interface mode {cols}x{rows}");
            let code = ui::run(sst, play);
            std::process::exit(code);
        }
        sst.prout("Terminal is not wide enough for fullscreen.");
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| play(&mut sst)));
    match result {
        Ok(()) => exit_with(&mut sst, 0),
        Err(payload) => {
            if let Some(SystemExit(code)) = payload.downcast_ref::<SystemExit>() {
                exit_with(&mut sst, *code);
            }
            sst.io.out.flush();
            std::panic::resume_unwind(payload);
        }
    }
}

/// `os.get_terminal_size()` on stdout (called only when stdout is a terminal).
fn terminal_rows() -> Option<i64> {
    ratatui::crossterm::terminal::size().ok().map(|(_, rows)| rows as i64)
}

/// The play-again loop of SST2K's main program.
fn play(sst: &mut Sst) {
    loop {
        sst.clrscr();
        sst.prelim();
        sst.setup();
        if sst.game.alldone {
            sst.score();
            sst.game.alldone = false;
        } else {
            sst.makemoves();
        }
        if sst.io.replay_started {
            break;
        }
        sst.skip(1);
        if sst.game.options & OPTION_TTY != 0 {
            sst.stars();
        }
        sst.skip(1);
        if sst.game.tourn.is_some_and(|t| t != 0) && sst.game.alldone {
            sst.proutn("Do you want your score recorded?");
            if sst.ja() {
                sst.scanner.chew();
                sst.scanner.push("\n");
                sst.freeze(false);
            }
        }
        sst.scanner.chew();
        sst.proutn("Do you want to play again? ");
        if !sst.ja() {
            break;
        }
    }
    sst.skip(1);
    sst.prout("May the Great Bird of the Galaxy roost upon your home planet.");
}
