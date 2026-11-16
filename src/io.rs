//! The I/O layer of `sst` (3891-4315) in teletype mode.
//!
//! The game always runs with SST2K's TTY semantics; the curses window calls
//! (`setwnd`, `clreol`, `textcolor`, ...) are the no-ops they are in TTY
//! mode. Output goes to an `Output` sink (stdout, or the UI) and input comes
//! from the replay file, then from an `Input` source.

use crate::consts::*;
use crate::sst::{Sst, SystemExit};
use std::io::{BufRead, Read, Write};

/// Where the game's text goes.
pub trait Output: Send {
    fn write(&mut self, s: &str);
    fn flush(&mut self) {}
}

/// Where typed lines come from (after any replay file). `None` is EOF.
pub trait Input: Send {
    fn read_line(&mut self) -> Option<String>;
}

pub struct StdoutOutput {
    out: std::io::BufWriter<std::io::Stdout>,
}

impl StdoutOutput {
    pub fn new() -> Self {
        StdoutOutput { out: std::io::BufWriter::with_capacity(1 << 16, std::io::stdout()) }
    }
}

impl Default for StdoutOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl Output for StdoutOutput {
    fn write(&mut self, s: &str) {
        let _ = self.out.write_all(s.as_bytes());
    }
    fn flush(&mut self) {
        let _ = self.out.flush();
    }
}

/// Reads lines with Python's universal newlines: "\n", "\r" and "\r\n"
/// all end a line and come back as "\n". `readline()` keeps the "\n".
pub struct LineReader<R: Read> {
    inner: std::io::BufReader<R>,
}

impl<R: Read> LineReader<R> {
    pub fn new(r: R) -> Self {
        LineReader { inner: std::io::BufReader::new(r) }
    }

    /// Python `readline()`: "" at EOF, the line with its "\n" otherwise.
    pub fn readline(&mut self) -> String {
        let mut bytes = Vec::new();
        loop {
            let buf = match self.inner.fill_buf() {
                Ok(b) => b,
                Err(_) => break,
            };
            if buf.is_empty() {
                break;
            }
            if let Some(pos) = buf.iter().position(|&c| c == b'\n' || c == b'\r') {
                let c = buf[pos];
                bytes.extend_from_slice(&buf[..pos]);
                self.inner.consume(pos + 1);
                if c == b'\r' {
                    // swallow the \n of a \r\n
                    if let Ok(b) = self.inner.fill_buf()
                        && b.first() == Some(&b'\n')
                    {
                        self.inner.consume(1);
                    }
                }
                bytes.push(b'\n');
                break;
            }
            let n = buf.len();
            bytes.extend_from_slice(buf);
            self.inner.consume(n);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

pub struct StdinInput {
    reader: LineReader<std::io::Stdin>,
}

impl StdinInput {
    pub fn new() -> Self {
        StdinInput { reader: LineReader::new(std::io::stdin()) }
    }
}

impl Default for StdinInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Input for StdinInput {
    /// Python `input()`: the line without its "\n"; None at EOF.
    fn read_line(&mut self) -> Option<String> {
        let l = self.reader.readline();
        if l.is_empty() {
            return None;
        }
        Some(l.strip_suffix('\n').map(String::from).unwrap_or(l))
    }
}

pub struct Io {
    /// terminal rows for paging; 0 means never page (pipes, files, the UI)
    pub rows: i64,
    pub linecount: i64,
    /// the replay file (`replayfp`); `replay_started` is "replayfp is truthy",
    /// which stays true after Python closes it at EOF
    pub replay: Option<LineReader<Box<dyn Read + Send>>>,
    pub replay_started: bool,
    pub replay_open: bool,
    /// SST2K's input log (`logfp`)
    pub logfp: Option<std::fs::File>,
    pub out: Box<dyn Output>,
    pub input: Box<dyn Input>,
    /// `prouts` types slowly unless replaying; off with SUPER_TREK_FAST
    pub slow: bool,
    pub rng_trace: bool,
    /// called before each command prompt (the UI takes its snapshot here)
    pub on_prompt: Option<Box<dyn FnMut(&crate::state::Gamestate) + Send>>,
}

impl Default for Io {
    fn default() -> Self {
        Io {
            rows: 0,
            linecount: 0,
            replay: None,
            replay_started: false,
            replay_open: false,
            logfp: None,
            out: Box::new(StdoutOutput::new()),
            input: Box::new(StdinInput::new()),
            slow: std::env::var_os("SUPER_TREK_FAST").is_none(),
            rng_trace: false,
            on_prompt: None,
        }
    }
}

impl Sst {
    /// `skip(i)`: skip lines, pausing if the screen would scroll.
    pub fn skip(&mut self, i: i64) {
        for _ in 0..i {
            self.io.linecount += 1;
            if self.io.rows != 0 && self.io.linecount >= self.io.rows {
                self.pause_game();
            } else {
                self.io.out.write("\n");
            }
        }
    }

    /// `proutn`: text with no line feed.
    pub fn proutn(&mut self, s: &str) {
        self.io.out.write(s);
    }

    /// `prout`: text and a line feed.
    pub fn prout(&mut self, s: &str) {
        self.proutn(s);
        self.skip(1);
    }

    /// `prouts`: emit slowly.
    pub fn prouts(&mut self, s: &str) {
        let slow = self.io.slow && !self.io.replay_open;
        for c in s.chars() {
            if slow {
                self.io.out.flush();
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
            let mut b = [0u8; 4];
            self.proutn(c.encode_utf8(&mut b));
        }
        if slow {
            self.io.out.flush();
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
    }

    pub fn pause_game(&mut self) {
        let prompt = if self.game.skill > SKILL_FAIR { "[CONTINUE?]" } else { "[PRESS ENTER TO CONTINUE]" };
        self.io.out.write("\n");
        self.proutn(prompt);
        if !self.io.replay_started {
            self.io.out.flush();
            let linein = self.io.input.read_line().unwrap_or_default();
            if !linein.is_empty() {
                let tokens = crate::py::py_split(&linein);
                if !tokens.is_empty() {
                    let mut q = tokens;
                    q.push("\n".into());
                    q.append(&mut self.scanner.inqueue);
                    self.scanner.inqueue = q;
                }
            }
        }
        let n = self.io.rows.max(0) as usize;
        self.io.out.write(&"\n".repeat(n));
        self.io.linecount = 0;
    }

    /// `cgetline()`: a line of input, with its "\n" ("" at replay EOF).
    pub fn cgetline(&mut self) -> String {
        let linein;
        if self.io.replay_open {
            loop {
                let l = self.io.replay.as_mut().unwrap().readline();
                self.proutn(&l);
                if l.is_empty() {
                    self.prout("*** Replay finished");
                    self.io.replay_open = false;
                    self.io.replay = None;
                    linein = l;
                    break;
                } else if !l.starts_with('#') {
                    linein = l;
                    break;
                }
            }
        } else {
            self.io.out.flush();
            match self.io.input.read_line() {
                Some(l) => linein = l + "\n",
                None => {
                    self.prout("");
                    std::panic::resume_unwind(Box::new(SystemExit(0)));
                }
            }
        }
        log::debug!(target: "input", "{:?}", linein);
        if let Some(f) = self.io.logfp.as_mut() {
            let _ = f.write_all(linein.as_bytes());
        }
        linein
    }

    pub fn clrscr(&mut self) {
        self.io.linecount = 0;
    }

    pub fn announce(&mut self) {
        self.skip(1);
        self.prouts("[ANNOUNCEMENT ARRIVING...]");
        self.skip(1);
    }

    /// `waitfor()`: nothing on a teletype.
    pub fn waitfor(&mut self) {}

    /// `drawmaps(mode)`: nothing on a teletype.
    pub fn drawmaps(&mut self, _mode: i64) {}

    /// `boom(w)`: nothing on a teletype.
    pub fn boom(&mut self, _w: crate::coord::Coord) {}

    /// `warble()`: teleportation effect.
    pub fn warble(&mut self) {
        self.prouts("     . . . . .     ");
    }

    /// `tracktorpedo(w, step, i, n, iquad)` in TTY mode.
    pub fn tracktorpedo(&mut self, w: crate::coord::Coord, step: i64, i: i64, n: i64, _iquad: char) {
        if step == 1 {
            if n != 1 {
                self.skip(1);
                let s = crate::pf!("Track for torpedo number %d-  ", i + 1);
                self.proutn(&s);
            } else {
                self.skip(1);
                self.proutn("Torpedo track- ");
            }
        } else if step == 4 || step == 9 {
            self.skip(1);
        }
        let s = crate::pf!("%s   ", w);
        self.proutn(&s);
    }

    /// `makechart()`
    pub fn makechart(&mut self) {
        self.chart();
        if self.game.options & OPTION_TTY != 0 {
            self.skip(1);
        }
    }

    /// `prstat(txt, data)`
    pub fn prstat(&mut self, txt: &str, data: &str) {
        self.proutn(txt);
        let pad = NSYM.saturating_sub(txt.chars().count());
        self.proutn(&" ".repeat(pad));
        self.proutn(data);
        self.skip(1);
    }

    /// `stars()`: a line of stars.
    pub fn stars(&mut self) {
        self.prouts("******************************************************");
        self.skip(1);
    }
}
