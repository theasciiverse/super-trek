//! `sstscanner`, `ja` and `huh` from `sst` (7844-7961).

use crate::consts::OPTION_ALPHAMERIC;
use crate::coord::Coord;
use crate::py::{py_float, py_split, pyround};
use crate::sst::Sst;

/// Token types; `Eof` is the `None` nexttok returns at the end of a replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tok {
    #[default]
    None,
    Eol,
    Real,
    Alpha,
    Eof,
}

#[derive(Clone, Debug, Default)]
pub struct Scanner {
    /// `scanner.type` (Tok::None before the first token)
    pub type_: Tok,
    /// `scanner.token`; `token_none` is true where Python holds None
    pub token: String,
    pub token_none: bool,
    /// `scanner.real`; NaN where Python holds None
    pub real: f64,
    pub inqueue: Vec<String>,
}

impl Scanner {
    pub fn new() -> Self {
        Scanner { type_: Tok::None, token: String::new(), token_none: true, real: 0.0, inqueue: Vec::new() }
    }
    pub fn append(&mut self, tok: &str) {
        self.inqueue.push(tok.to_string());
    }
    pub fn push(&mut self, tok: &str) {
        self.inqueue.insert(0, tok.to_string());
    }
    pub fn waiting(&self) -> bool {
        !self.inqueue.is_empty()
    }
    /// `chew()`: demand input for the next scan.
    pub fn chew(&mut self) {
        self.inqueue.clear();
        self.real = f64::NAN;
        self.token.clear();
        self.token_none = true;
    }
    /// `sees(s)`: s starts with the token.
    pub fn sees(&self, s: &str) -> bool {
        s.starts_with(self.token.as_str())
    }
    /// `int()`: round the token value to the nearest integer.
    pub fn int(&self) -> i64 {
        pyround(self.real)
    }
}

/// Python `int(text)` for short tokens.
pub fn py_int(text: &str) -> Option<i64> {
    let t = text.trim_matches(crate::py::py_isspace);
    let (neg, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let cs: Vec<char> = body.chars().collect();
    if cs.is_empty() {
        return None;
    }
    let mut v: i64 = 0;
    for (k, &c) in cs.iter().enumerate() {
        if c == '_' {
            if k == 0 || k + 1 == cs.len() || !cs[k - 1].is_ascii_digit() || !cs[k + 1].is_ascii_digit() {
                return None;
            }
            continue;
        }
        let d = c.to_digit(10)? as i64;
        v = v.saturating_mul(10).saturating_add(d);
    }
    Some(if neg { -v } else { v })
}

impl Sst {
    /// `scanner.nexttok()`
    pub fn nexttok(&mut self) -> Tok {
        self.scanner.real = 0.0;
        self.scanner.token.clear();
        self.scanner.token_none = false;
        while self.scanner.inqueue.is_empty() {
            let sline = self.cgetline();
            // In TTY mode curwnd and prompt_window are both None, so this
            // always runs: the line count restarts at every line typed.
            self.clrscr();
            self.clrscr();
            if sline.is_empty() {
                return Tok::Eof;
            }
            let lead = sline.trim_start_matches(crate::py::py_isspace);
            let mut q = py_split(lead);
            q.push("\n".into());
            self.scanner.inqueue = q;
        }
        let tok = self.scanner.inqueue.remove(0);
        self.scanner.token = tok;
        if self.scanner.token == "\n" {
            self.scanner.type_ = Tok::Eol;
            return Tok::Eol;
        }
        if let Some(v) = py_float(&self.scanner.token)
            && v.is_finite()
        {
            self.scanner.real = v;
            self.scanner.type_ = Tok::Real;
            return Tok::Real;
        }
        self.scanner.token = self.scanner.token.to_lowercase();
        self.scanner.type_ = Tok::Alpha;
        self.scanner.real = f64::NAN;
        Tok::Alpha
    }

    /// `scanner.getcoord()`: None after complaining.
    pub fn getcoord(&mut self) -> Option<Coord> {
        let mut s = Coord::none();
        self.nexttok();
        if self.game.options & OPTION_ALPHAMERIC != 0 && self.scanner.type_ == Tok::Alpha {
            let cs: Vec<char> = self.scanner.token.chars().collect();
            // token[0] in "abcdefghij" and token[1] in "0123456789"
            if cs.is_empty() || !"abcdefghij".contains(cs[0]) {
                // fall through to the numeric check
            } else if cs.len() < 2 {
                self.huh();
                return None;
            } else if "0123456789".contains(cs[1]) {
                let rest: String = cs[1..].iter().collect();
                match py_int(&rest) {
                    Some(v) => {
                        s.i = cs[0] as i64 - 'a' as i64;
                        s.j = v - 1;
                        return Some(s);
                    }
                    None => {
                        self.huh();
                        return None;
                    }
                }
            }
        }
        if self.scanner.type_ != Tok::Real {
            self.huh();
            return None;
        }
        s.i = self.scanner.int() - 1;
        self.nexttok();
        if self.scanner.type_ != Tok::Real {
            self.huh();
            return None;
        }
        s.j = self.scanner.int() - 1;
        Some(s)
    }

    /// `ja()`: yes-or-no confirmation.
    pub fn ja(&mut self) -> bool {
        self.scanner.chew();
        loop {
            self.nexttok();
            if self.scanner.token == "y" {
                return true;
            }
            if self.scanner.token == "n" {
                return false;
            }
            self.scanner.chew();
            self.proutn("Please answer with \"y\" or \"n\": ");
        }
    }

    /// `huh()`: complain about unparseable input.
    pub fn huh(&mut self) {
        self.scanner.chew();
        self.skip(1);
        self.prout("Beg your pardon, Captain?");
    }
}
