//! Python semantics the game's output and arithmetic depend on.
//!
//! - `pf!`: Python's `%` string formatting, run at run time on format
//!   strings copied verbatim from `sst`.
//! - `pypow`: every `**` and `math.pow`. It goes through `black_box` so that
//!   LLVM cannot turn `pow(x, 2.0)` into `x * x` in release builds.
//! - `pyround`: Python 3 `round()`, which rounds ties to even.
//! - `pymod`/`pyfloordiv`: Python `%` and `//` on numbers (floor semantics).
//! - `py_float`/`py_int`: Python `float()` and `int()` parsing of tokens.
//! - `float_repr`: Python's `str()`/`repr()` of a float.

use std::fmt::Write as _;

/// One argument of a `%` format.
#[derive(Debug, Clone)]
pub enum PyArg {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
}

impl From<i64> for PyArg {
    fn from(v: i64) -> Self {
        PyArg::Int(v)
    }
}
impl From<i32> for PyArg {
    fn from(v: i32) -> Self {
        PyArg::Int(v as i64)
    }
}
impl From<usize> for PyArg {
    fn from(v: usize) -> Self {
        PyArg::Int(v as i64)
    }
}
impl From<f64> for PyArg {
    fn from(v: f64) -> Self {
        PyArg::Float(v)
    }
}
impl From<bool> for PyArg {
    fn from(v: bool) -> Self {
        PyArg::Bool(v)
    }
}
impl From<&str> for PyArg {
    fn from(v: &str) -> Self {
        PyArg::Str(v.to_string())
    }
}
impl From<String> for PyArg {
    fn from(v: String) -> Self {
        PyArg::Str(v)
    }
}
impl From<&String> for PyArg {
    fn from(v: &String) -> Self {
        PyArg::Str(v.clone())
    }
}
impl From<char> for PyArg {
    fn from(v: char) -> Self {
        PyArg::Str(v.to_string())
    }
}
impl From<crate::coord::Coord> for PyArg {
    fn from(v: crate::coord::Coord) -> Self {
        PyArg::Str(v.to_string())
    }
}
impl From<&crate::coord::Coord> for PyArg {
    fn from(v: &crate::coord::Coord) -> Self {
        PyArg::Str(v.to_string())
    }
}
impl<T: Into<PyArg>> From<Option<T>> for PyArg {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(x) => x.into(),
            None => PyArg::None,
        }
    }
}

/// `pf!("fmt %d", a, b)` is Python's `"fmt %d" % (a, b)`.
#[macro_export]
macro_rules! pf {
    ($fmt:expr) => { $crate::py::pyfmt($fmt, &[]) };
    ($fmt:expr, $($arg:expr),+ $(,)?) => {
        $crate::py::pyfmt($fmt, &[$($crate::py::PyArg::from($arg)),+])
    };
}

fn pad(s: String, width: usize, left: bool, zero: bool) -> String {
    let n = s.chars().count();
    if n >= width {
        return s;
    }
    let fill = width - n;
    if left {
        let mut s = s;
        s.extend(std::iter::repeat_n(' ', fill));
        s
    } else if zero {
        let (sign, digits) = match s.strip_prefix('-') {
            Some(rest) => ("-", rest.to_string()),
            None => match s.strip_prefix('+') {
                Some(rest) => ("+", rest.to_string()),
                None => ("", s.clone()),
            },
        };
        format!("{sign}{}{digits}", "0".repeat(fill))
    } else {
        format!("{}{s}", " ".repeat(fill))
    }
}

/// Python's `int(x)` for a float: truncate toward zero.
pub fn trunc(x: f64) -> i64 {
    if !x.is_finite() {
        panic!("Python: cannot convert float {x} to integer");
    }
    x.trunc() as i64
}

/// Python's `str()` of a value as `%s` shows it.
pub fn py_str(a: &PyArg) -> String {
    match a {
        PyArg::Int(v) => v.to_string(),
        PyArg::Float(v) => float_repr(*v),
        PyArg::Str(s) => s.clone(),
        PyArg::Bool(b) => if *b { "True" } else { "False" }.to_string(),
        PyArg::None => "None".to_string(),
    }
}

/// Python's `%` operator on a string.
pub fn pyfmt(fmt: &str, args: &[PyArg]) -> String {
    let mut out = String::new();
    let mut it = fmt.chars().peekable();
    let mut argi = 0;
    while let Some(c) = it.next() {
        if c != '%' {
            out.push(c);
            continue;
        }
        let mut left = false;
        let mut zero = false;
        let mut plus = false;
        let mut space = false;
        while let Some(&f) = it.peek() {
            match f {
                '-' => left = true,
                '0' => zero = true,
                '+' => plus = true,
                ' ' => space = true,
                '#' => {}
                _ => break,
            }
            it.next();
        }
        let mut width = 0usize;
        while let Some(&d) = it.peek() {
            if let Some(v) = d.to_digit(10) {
                width = width * 10 + v as usize;
                it.next();
            } else {
                break;
            }
        }
        let mut prec: Option<usize> = None;
        if it.peek() == Some(&'.') {
            it.next();
            let mut p = 0usize;
            while let Some(&d) = it.peek() {
                if let Some(v) = d.to_digit(10) {
                    p = p * 10 + v as usize;
                    it.next();
                } else {
                    break;
                }
            }
            prec = Some(p);
        }
        let conv = it.next().expect("Python: incomplete format");
        if conv == '%' {
            out.push('%');
            continue;
        }
        let arg = args.get(argi).unwrap_or_else(|| panic!("Python: not enough arguments for format string {fmt:?}"));
        argi += 1;
        let sign = |s: String, neg: bool| -> String {
            if neg {
                s
            } else if plus {
                format!("+{s}")
            } else if space {
                format!(" {s}")
            } else {
                s
            }
        };
        let body = match conv {
            's' | 'r' => {
                let mut s = py_str(arg);
                if let Some(p) = prec {
                    s = s.chars().take(p).collect();
                }
                pad(s, width, left, false)
            }
            'd' | 'i' => {
                let v = match arg {
                    PyArg::Int(v) => *v,
                    PyArg::Float(f) => trunc(*f),
                    PyArg::Bool(b) => *b as i64,
                    _ => panic!("Python: %d format: a real number is required, not {arg:?}"),
                };
                pad(sign(v.to_string(), v < 0), width, left, zero)
            }
            'f' | 'F' => {
                let v = match arg {
                    PyArg::Int(v) => *v as f64,
                    PyArg::Float(f) => *f,
                    PyArg::Bool(b) => *b as i64 as f64,
                    _ => panic!("Python: %f format: a real number is required, not {arg:?}"),
                };
                let p = prec.unwrap_or(6);
                let s = if v.is_nan() {
                    "nan".to_string()
                } else if v.is_infinite() {
                    if v > 0.0 { "inf".to_string() } else { "-inf".to_string() }
                } else {
                    format!("{v:.p$}")
                };
                let neg = s.starts_with('-');
                pad(sign(s, neg), width, left, zero && v.is_finite())
            }
            'c' => {
                let s = match arg {
                    PyArg::Str(s) => s.clone(),
                    PyArg::Int(v) => char::from_u32(*v as u32).map(String::from).unwrap_or_default(),
                    _ => panic!("Python: %c requires int or char"),
                };
                pad(s, width, left, false)
            }
            other => panic!("pyfmt: unsupported conversion %{other} in {fmt:?}"),
        };
        out.push_str(&body);
    }
    if argi != args.len() {
        panic!("Python: not all arguments converted during string formatting in {fmt:?}");
    }
    out
}

/// Python's `repr()`/`str()` of a float: shortest round-trip digits,
/// scientific notation outside 1e-4 <= |x| < 1e16.
pub fn float_repr(x: f64) -> String {
    if x.is_nan() {
        return "nan".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "inf".into() } else { "-inf".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    // Rust's {:e} gives the shortest round-trip digits as d.ddde[-]x
    let e = format!("{:e}", x);
    let (mant, exp) = e.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    let neg = mant.starts_with('-');
    let mant = mant.trim_start_matches('-');
    let digits: String = mant.chars().filter(|c| *c != '.').collect();
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    if (-4..16).contains(&exp) {
        if exp >= 0 {
            let e = exp as usize;
            if digits.len() > e + 1 {
                s.push_str(&digits[..e + 1]);
                s.push('.');
                s.push_str(&digits[e + 1..]);
            } else {
                s.push_str(&digits);
                s.push_str(&"0".repeat(e + 1 - digits.len()));
                s.push_str(".0");
            }
        } else {
            s.push_str("0.");
            s.push_str(&"0".repeat((-exp - 1) as usize));
            s.push_str(&digits);
        }
    } else {
        s.push_str(&digits[..1]);
        if digits.len() > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        let _ = write!(s, "e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    s
}

/// Every `**` and `math.pow` in `sst`: libm's pow, never strength-reduced.
#[inline(never)]
pub fn pypow(x: f64, y: f64) -> f64 {
    x.powf(std::hint::black_box(y))
}

/// Python 3 `round(x)` returning an int: ties to even.
pub fn pyround(x: f64) -> i64 {
    trunc(x.round_ties_even())
}

/// Python `a % b` on floats (result takes the sign of `b`).
pub fn pymod(a: f64, b: f64) -> f64 {
    let m = a % b;
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) { m + b } else { m }
    } else {
        0.0f64.copysign(b)
    }
}

/// Python `a // b` on floats.
pub fn pyfloordiv(a: f64, b: f64) -> f64 {
    let m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 && ((b < 0.0) != (m < 0.0)) {
        div -= 1.0;
    }
    if div != 0.0 {
        let floordiv = div.floor();
        if div - floordiv > 0.5 { floordiv + 1.0 } else { floordiv }
    } else {
        0.0f64.copysign(a / b)
    }
}

/// Python's whitespace for `str.split()`, which includes U+001C..U+001F.
pub fn py_isspace(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// Python `str.split()` with no arguments.
pub fn py_split(s: &str) -> Vec<String> {
    s.split(py_isspace).filter(|t| !t.is_empty()).map(String::from).collect()
}

/// Strip Python digit-group underscores ("1_000"); None if they are misplaced.
fn strip_underscores(s: &str) -> Option<String> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    for (k, &c) in cs.iter().enumerate() {
        if c == '_' {
            let ok = k > 0 && k + 1 < cs.len() && cs[k - 1].is_ascii_digit() && cs[k + 1].is_ascii_digit();
            if !ok {
                return None;
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Python `float(token)`; None where Python raises ValueError.
pub fn py_float(tok: &str) -> Option<f64> {
    let t = tok.trim_matches(py_isspace);
    let lower = t.to_ascii_lowercase();
    let body = lower.trim_start_matches(['+', '-']);
    if lower.len() - body.len() > 1 {
        return None;
    }
    if matches!(body, "inf" | "infinity" | "nan") {
        let v = if body == "nan" { f64::NAN } else { f64::INFINITY };
        return Some(if lower.starts_with('-') { -v } else { v });
    }
    let s = strip_underscores(t)?;
    // grammar: [sign] (digits [. [digits]] | . digits) [(e|E) [sign] digits]
    let b = s.as_bytes();
    let mut k = 0;
    if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
        k += 1;
    }
    let d0 = k;
    while k < b.len() && b[k].is_ascii_digit() {
        k += 1;
    }
    let mut ndig = k - d0;
    if k < b.len() && b[k] == b'.' {
        k += 1;
        let f0 = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        ndig += k - f0;
    }
    if ndig == 0 {
        return None;
    }
    if k < b.len() && (b[k] == b'e' || b[k] == b'E') {
        k += 1;
        if k < b.len() && (b[k] == b'+' || b[k] == b'-') {
            k += 1;
        }
        let e0 = k;
        while k < b.len() && b[k].is_ascii_digit() {
            k += 1;
        }
        if k == e0 {
            return None;
        }
    }
    if k != b.len() {
        return None;
    }
    s.parse::<f64>().ok()
}

/// Python `int(text)` reduced modulo `m` (for RNG seeds of any size).
/// None where Python raises ValueError.
pub fn py_int_mod(text: &str, m: i64) -> Option<i64> {
    let t = text.trim_matches(py_isspace);
    let (neg, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let body = strip_underscores(body)?;
    if body.is_empty() || !body.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut r: i64 = 0;
    for c in body.bytes() {
        r = (r * 10 + (c - b'0') as i64) % m;
    }
    Some(if neg { (m - r) % m } else { r })
}

/// Python `repr()` of a str, as "Can't interpret" shows it.
pub fn str_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') { '"' } else { '\'' };
    let mut out = String::new();
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if is_printable(c) => out.push(c),
            c => {
                let v = c as u32;
                if v < 0x100 {
                    let _ = write!(out, "\\x{v:02x}");
                } else if v < 0x10000 {
                    let _ = write!(out, "\\u{v:04x}");
                } else {
                    let _ = write!(out, "\\U{v:08x}");
                }
            }
        }
    }
    out.push(quote);
    out
}

/// Approximation of Python's `str.isprintable()` for repr: control
/// characters, separators other than space, and unassigned/format chars
/// are escaped.
fn is_printable(c: char) -> bool {
    if c == ' ' {
        return true;
    }
    if c.is_control() || (c.is_whitespace()) {
        return false;
    }
    let v = c as u32;
    // format characters (Cf) commonly met, and surrogates/private use
    !matches!(v, 0xAD | 0x600..=0x605 | 0x61C | 0x6DD | 0x70F | 0x200B..=0x200F | 0x202A..=0x202E
        | 0x2060..=0x2064 | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0xE000..=0xF8FF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_python() {
        assert_eq!(pf!("%d", 3.9), "3");
        assert_eq!(pf!("%d", -3.9), "-3");
        assert_eq!(pf!("%5d|%-5d|%05d", 42i64, 42i64, -42i64), "   42|42   |-0042");
        assert_eq!(pf!("%.2f %6.2f %2.0f", 0.125, 3.14159, 2.5), "0.12   3.14  2");
        assert_eq!(pf!("%s %s %s", 5.0, 1e-5, 1e16), "5.0 1e-05 1e+16");
        assert_eq!(pf!("%s", 0.1 + 0.2), "0.30000000000000004");
        assert_eq!(pf!("%-12s|%.4s", "AB", "ABCDEFG"), "AB          |ABCD");
        assert_eq!(pf!("100%%"), "100%");
        assert_eq!(pf!("%s", true), "True");
        assert_eq!(float_repr(123456789012345.6), "123456789012345.6");
        assert_eq!(float_repr(1234567890123456.0), "1234567890123456.0");
        assert_eq!(float_repr(0.0001), "0.0001");
        assert_eq!(float_repr(1.5e300), "1.5e+300");
    }

    #[test]
    fn parses_like_python() {
        assert_eq!(py_float("1_000.5"), Some(1000.5));
        assert_eq!(py_float("1__0"), None);
        assert_eq!(py_float(".5"), Some(0.5));
        assert_eq!(py_float("5."), Some(5.0));
        assert_eq!(py_float("."), None);
        assert_eq!(py_float("1e5"), Some(1e5));
        assert_eq!(py_float("1e"), None);
        assert!(py_float("-Infinity").unwrap().is_infinite());
        assert!(py_float("nan").unwrap().is_nan());
        assert_eq!(py_float("0x10"), None);
        assert_eq!(py_float("a9"), None);
        assert_eq!(py_int_mod("1556084761", 1048576), Some(1556084761 % 1048576));
        assert_eq!(py_int_mod("-1", 1048576), Some(1048575));
        assert_eq!(py_int_mod("1.5", 1048576), None);
        assert_eq!(pyround(2.5), 2);
        assert_eq!(pyround(3.5), 4);
        assert_eq!(pyround(-2.5), -2);
        assert_eq!(pymod(-1.0, 10.0), 9.0);
        assert_eq!(pyfloordiv(-1.0, 10.0), -1.0);
        assert_eq!(str_repr("it's"), "\"it's\"");
        assert_eq!(str_repr("a\tb"), "'a\\tb'");
    }
}
