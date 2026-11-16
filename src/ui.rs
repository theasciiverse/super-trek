//! The Ratatui interface: SST2K's line-mode game runs unchanged on its own
//! thread, and this shell draws the bridge around it. The panels (short
//! range scan, status, long range scan, star chart) are drawn from a copy of
//! the game taken at each command prompt, reading only what the ship knows;
//! drawing them never changes the game, so a game played here replays
//! exactly in line mode from its input log.

use crate::consts::*;
use crate::coord::{Coord, letterize};
use crate::io::{Input, Output};
use crate::sst::{Sst, SystemExit};
use crate::state::Gamestate;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::{DefaultTerminal, Frame};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

const GREEN: Color = Color::Rgb(0x33, 0xff, 0x66);
const DIM: Color = Color::Rgb(0x1f, 0x99, 0x3d);
const AMBER: Color = Color::Rgb(0xff, 0xb0, 0x00);
const RED: Color = Color::Rgb(0xff, 0x55, 0x55);
const CYAN: Color = Color::Rgb(0x55, 0xdd, 0xff);
const MAGENTA: Color = Color::Rgb(0xdd, 0x77, 0xff);
const WHITE: Color = Color::Rgb(0xee, 0xee, 0xee);

enum Msg {
    Text(String),
    Snap(Box<Gamestate>),
    Exit(i32),
}

struct ChanOut(Sender<Msg>);

impl Output for ChanOut {
    fn write(&mut self, s: &str) {
        let _ = self.0.send(Msg::Text(s.to_string()));
    }
}

struct ChanIn(Receiver<String>);

impl Input for ChanIn {
    fn read_line(&mut self) -> Option<String> {
        self.0.recv().ok()
    }
}

struct App {
    lines: Vec<String>,
    partial: String,
    input: String,
    history: Vec<String>,
    hist_pos: Option<usize>,
    scroll: usize,
    snap: Option<Box<Gamestate>>,
    finished: Option<i32>,
}

impl App {
    fn push_text(&mut self, s: &str) {
        for c in s.chars() {
            if c == '\n' {
                let l = std::mem::take(&mut self.partial);
                self.lines.push(l);
            } else {
                self.partial.push(c);
            }
        }
        if self.lines.len() > 5000 {
            self.lines.drain(..1000);
        }
    }
}

/// Run the game inside the interface. `play` is the play-again loop of
/// `main`, run on the game thread with a channel for output and input.
pub fn run(mut sst: Sst, play: fn(&mut Sst)) -> i32 {
    let (tx, rx) = channel::<Msg>();
    let (in_tx, in_rx) = channel::<String>();
    sst.io.out = Box::new(ChanOut(tx.clone()));
    sst.io.input = Box::new(ChanIn(in_rx));
    sst.io.rows = 0;
    let snap_tx = tx.clone();
    sst.io.on_prompt = Some(Box::new(move |g: &Gamestate| {
        let _ = snap_tx.send(Msg::Snap(Box::new(g.clone())));
    }));
    let exit_tx = tx;
    let game = std::thread::spawn(move || {
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| play(&mut sst)));
        if let Some(f) = sst.io.logfp.as_mut() {
            use std::io::Write;
            let _ = f.flush();
        }
        let code = match r {
            Ok(()) => 0,
            Err(p) => match p.downcast_ref::<SystemExit>() {
                Some(SystemExit(c)) => *c,
                None => {
                    let msg = p
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_default();
                    log::error!("game thread panicked: {msg}");
                    let _ = exit_tx.send(Msg::Text(format!("\n*** Internal error: {msg}\n")));
                    101
                }
            },
        };
        let _ = exit_tx.send(Msg::Exit(code));
    });

    let mut terminal = ratatui::init();
    let mut app = App {
        lines: Vec::new(),
        partial: String::new(),
        input: String::new(),
        history: Vec::new(),
        hist_pos: None,
        scroll: 0,
        snap: None,
        finished: None,
    };
    let code = event_loop(&mut terminal, &mut app, &rx, &in_tx);
    ratatui::restore();
    drop(in_tx);
    // The game thread may be waiting for input; it ends when the channel closes.
    let _ = game.join();
    code
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App, rx: &Receiver<Msg>, in_tx: &Sender<String>) -> i32 {
    loop {
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Text(s) => app.push_text(&s),
                Msg::Snap(g) => app.snap = Some(g),
                Msg::Exit(c) => app.finished = Some(c),
            }
        }
        let _ = terminal.draw(|f| draw(f, app));
        if !event::poll(Duration::from_millis(20)).unwrap_or(false) {
            continue;
        }
        let Ok(ev) = event::read() else { continue };
        let Event::Key(k) = ev else { continue };
        if k.kind == KeyEventKind::Release {
            continue;
        }
        log::debug!(target: "key", "{:?} {:?}", k.code, k.modifiers);
        if let Some(c) = app.finished {
            return c;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('c') | KeyCode::Char('d') if ctrl => return 0,
            KeyCode::Enter => {
                let line = std::mem::take(&mut app.input);
                app.push_text(&format!("{line}\n"));
                if !line.trim().is_empty() {
                    app.history.push(line.clone());
                }
                app.hist_pos = None;
                app.scroll = 0;
                let _ = in_tx.send(line);
            }
            KeyCode::Backspace => {
                app.input.pop();
            }
            KeyCode::Esc => app.input.clear(),
            KeyCode::Up => {
                if !app.history.is_empty() {
                    let p = match app.hist_pos {
                        None => app.history.len() - 1,
                        Some(p) => p.saturating_sub(1),
                    };
                    app.hist_pos = Some(p);
                    app.input = app.history[p].clone();
                }
            }
            KeyCode::Down => {
                if let Some(p) = app.hist_pos {
                    if p + 1 < app.history.len() {
                        app.hist_pos = Some(p + 1);
                        app.input = app.history[p + 1].clone();
                    } else {
                        app.hist_pos = None;
                        app.input.clear();
                    }
                }
            }
            KeyCode::PageUp => app.scroll = app.scroll.saturating_add(10).min(app.lines.len()),
            KeyCode::PageDown => app.scroll = app.scroll.saturating_sub(10),
            KeyCode::Char(c) if !ctrl => app.input.push(c),
            _ => {}
        }
    }
}

fn panel(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(DIM))
        .title(Span::styled(format!(" {title} "), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)))
}

fn draw(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Block::default().style(Style::default().bg(Color::Black)), area);
    let show_chart = area.height >= 40;
    let mut cons = vec![Constraint::Length(14)];
    if show_chart {
        cons.push(Constraint::Length(12));
    }
    cons.push(Constraint::Min(5));
    let rows = Layout::default().direction(Direction::Vertical).constraints(cons).split(area);
    let top = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(27), Constraint::Min(40), Constraint::Length(21)])
        .split(rows[0]);
    draw_srscan(f, top[0], app.snap.as_deref());
    draw_status(f, top[1], app.snap.as_deref());
    draw_lrscan(f, top[2], app.snap.as_deref());
    if show_chart {
        draw_chart(f, rows[1], app.snap.as_deref());
    }
    draw_log(f, rows[rows.len() - 1], app);
}

fn sym_style(c: char, g: &Gamestate) -> Style {
    let s = Style::default();
    match c {
        'E' | 'F' => s
            .fg(match g.condition.as_str() {
                "green" => GREEN,
                "yellow" => AMBER,
                "red" => RED,
                "docked" => CYAN,
                _ => WHITE,
            })
            .add_modifier(Modifier::BOLD),
        'K' | 'C' | 'S' | 'R' | 'T' => s.fg(RED).add_modifier(Modifier::BOLD),
        '?' => s.fg(MAGENTA).add_modifier(Modifier::BOLD),
        '@' | 'P' => s.fg(GREEN).add_modifier(Modifier::BOLD),
        'B' => s.fg(CYAN).add_modifier(Modifier::BOLD),
        '*' => s.fg(AMBER),
        '#' => s.fg(MAGENTA),
        _ => s.fg(DIM),
    }
}

fn row_label(g: &Gamestate, i: usize) -> String {
    if g.options & OPTION_ALPHAMERIC != 0 { format!("{} ", letterize(i as i64 + 1)) } else { format!("{:>2}", i + 1) }
}

fn draw_srscan(f: &mut Frame, r: Rect, g: Option<&Gamestate>) {
    let mut lines = Vec::new();
    if let Some(g) = g.filter(|g| !g.sector.is_none() && !g.quadrant.is_none()) {
        let good = !g.damaged(DSRSENS) || g.condition == "docked";
        lines.push(Line::styled("    1 2 3 4 5 6 7 8 9 10", Style::default().fg(DIM)));
        for i in 0..QS {
            let mut spans = vec![Span::styled(format!("{}  ", row_label(g, i)), Style::default().fg(DIM))];
            for j in 0..QS {
                let near = (i as i64 - g.sector.i).abs() <= 1 && (j as i64 - g.sector.j).abs() <= 1;
                if good || near {
                    let c = g.quad[i][j];
                    spans.push(Span::styled(format!("{c} "), sym_style(c, g)));
                } else {
                    spans.push(Span::styled("- ", Style::default().fg(DIM)));
                }
            }
            lines.push(Line::from(spans));
        }
        if !good {
            lines.push(Line::styled(" S.R. SENSORS DAMAGED", Style::default().fg(RED)));
        }
    }
    f.render_widget(Paragraph::new(lines).block(panel("Short-range scan")), r);
}

fn draw_status(f: &mut Frame, r: Rect, g: Option<&Gamestate>) {
    let mut lines = Vec::new();
    if let Some(g) = g.filter(|g| !g.quadrant.is_none()) {
        let label = Style::default().fg(DIM);
        let val = Style::default().fg(GREEN);
        let mut row = |k: &str, v: String, st: Style| {
            lines.push(Line::from(vec![Span::styled(format!("{k:<14}"), label), Span::styled(v, st)]));
        };
        let time = if g.sandbox { "Unlimited".to_string() } else { format!("{:.2}", g.state.remtime) };
        row("Stardate", format!("{:.1}", g.state.date), val);
        row("Time Left", time, val);
        let ndam = g.damage.iter().filter(|d| **d > 0.0).count();
        let cond = g.condition.to_uppercase();
        let cst = match g.condition.as_str() {
            "red" | "dead" => Style::default().fg(RED).add_modifier(Modifier::BOLD),
            "yellow" => Style::default().fg(AMBER),
            "docked" => Style::default().fg(CYAN),
            _ => val,
        };
        let cloak = if g.iscloaked { ", CLOAKED" } else { "" };
        row("Condition", format!("{cond}, {ndam} DAMAGES{cloak}"), cst);
        row("Position", format!("{} , {}", g.quadrant, g.sector), val);
        let ls = if g.damaged(DLIFSUP) {
            if g.condition == "docked" { "DAMAGED, Base provides".to_string() } else { format!("DAMAGED, reserves={:4.2}", g.lsupres) }
        } else {
            "ACTIVE".to_string()
        };
        row("Life Support", ls, val);
        row("Warp Factor", format!("{:.1}", g.warpfac), val);
        let est = if g.energy < 1000.0 { Style::default().fg(AMBER) } else { val };
        row("Energy", format!("{:.2}", g.energy), est);
        row("Torpedoes", format!("{}", g.torps), val);
        let sh = if g.damaged(DSHIELD) {
            "DAMAGED,"
        } else if g.shldup {
            "UP,"
        } else {
            "DOWN,"
        };
        let pct = if g.inshld > 0.0 { ((100.0 * g.shield) / g.inshld + 0.5).trunc() as i64 } else { 0 };
        row("Shields", format!("{sh} {pct}% {:.1} units", g.shield), val);
        row("Klingons Left", format!("{}", g.remkl()), Style::default().fg(RED));
        row("Bases Left", format!("{}", g.state.baseq.len()), val);
    }
    f.render_widget(Paragraph::new(lines).block(panel("Status")), r);
}

/// One star-chart cell as `chart()` shows it, from what the ship knows.
fn chart_cell(g: &Gamestate, i: usize, j: usize) -> String {
    let page = &g.state.chart[i][j];
    let q = &g.state.galaxy[i][j];
    if page.supernova {
        "***".into()
    } else if !q.charted && page.starbase {
        ".1.".into()
    } else if q.charted {
        let v = page.klingons.unwrap_or(0) * 100 + page.starbase as i64 * 10 + page.stars.unwrap_or(0);
        let s = format!("{v:>3}");
        if g.options & OPTION_DOTFILL != 0 { s.replace(' ', ".") } else { s }
    } else {
        "...".into()
    }
}

fn cell_style(s: &str) -> Style {
    if s == "***" {
        Style::default().fg(MAGENTA)
    } else if s.starts_with('.') && s != ".1." {
        Style::default().fg(DIM)
    } else if s.trim_start_matches(['.', ' ']).len() == 3 {
        Style::default().fg(RED).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(GREEN)
    }
}

fn draw_lrscan(f: &mut Frame, r: Rect, g: Option<&Gamestate>) {
    let mut lines = Vec::new();
    if let Some(g) = g.filter(|g| !g.quadrant.is_none()) {
        lines.push(Line::raw(""));
        for di in -1..=1i64 {
            let mut spans = vec![Span::raw(" ")];
            for dj in -1..=1i64 {
                let c = Coord::new(g.quadrant.i + di, g.quadrant.j + dj);
                let s = if c.valid_quadrant() { chart_cell(g, c.iu(), c.ju()) } else { " -1".into() };
                let st = if di == 0 && dj == 0 { cell_style(&s).add_modifier(Modifier::REVERSED) } else { cell_style(&s) };
                spans.push(Span::styled(format!(" {s} "), st));
            }
            lines.push(Line::from(spans));
            lines.push(Line::raw(""));
        }
        lines.push(Line::styled(format!("  Quadrant {}", g.quadrant), Style::default().fg(DIM)));
        if g.damaged(DLRSENS) {
            lines.push(Line::styled("  L.R. SENSORS DAMAGED", Style::default().fg(RED)));
        }
    }
    f.render_widget(Paragraph::new(lines).block(panel("Long-range")), r);
}

fn draw_chart(f: &mut Frame, r: Rect, g: Option<&Gamestate>) {
    let mut lines = Vec::new();
    if let Some(g) = g.filter(|g| !g.quadrant.is_none()) {
        lines.push(Line::styled("      1    2    3    4    5    6    7    8", Style::default().fg(DIM)));
        for i in 0..GS {
            let mut spans = vec![Span::styled(format!("{} |", row_label(g, i).trim_end()), Style::default().fg(DIM))];
            for j in 0..GS {
                let here = i as i64 == g.quadrant.i && j as i64 == g.quadrant.j;
                let s = chart_cell(g, i, j);
                let (l, rr) = if here { ("<", ">") } else { (" ", " ") };
                spans.push(Span::styled(l, Style::default().fg(AMBER)));
                spans.push(Span::styled(s.clone(), cell_style(&s)));
                spans.push(Span::styled(rr, Style::default().fg(AMBER)));
            }
            spans.push(Span::styled("  |", Style::default().fg(DIM)));
            lines.push(Line::from(spans));
        }
    }
    f.render_widget(Paragraph::new(lines).block(panel("Star chart")), r);
}

fn draw_log(f: &mut Frame, r: Rect, app: &App) {
    let inner_h = r.height.saturating_sub(2) as usize;
    let inner_w = r.width.saturating_sub(2).max(1) as usize;
    // wrap long lines by width so scrolling counts screen rows
    let mut rows: Vec<Line> = Vec::new();
    let style = Style::default().fg(GREEN);
    for l in &app.lines {
        wrap_into(&mut rows, l, inner_w, style);
    }
    let mut last = vec![Span::styled(app.partial.clone(), style)];
    if app.finished.is_none() {
        last.push(Span::styled(app.input.clone(), Style::default().fg(AMBER).add_modifier(Modifier::BOLD)));
        last.push(Span::styled("_", Style::default().fg(AMBER).add_modifier(Modifier::SLOW_BLINK)));
    }
    let lastline: String = last.iter().map(|s| s.content.as_ref()).collect();
    if lastline.chars().count() <= inner_w {
        rows.push(Line::from(last));
    } else {
        wrap_into(&mut rows, &lastline, inner_w, Style::default().fg(AMBER));
    }
    if app.finished.is_some() {
        rows.push(Line::styled("[press any key to leave]", Style::default().fg(AMBER)));
    }
    let end = rows.len().saturating_sub(app.scroll);
    let start = end.saturating_sub(inner_h);
    let shown: Vec<Line> = rows[start..end].to_vec();
    let title = if app.scroll > 0 { "Bridge (scrolled: PgDn)" } else { "Bridge  (PgUp scrolls, Ctrl-C leaves)" };
    f.render_widget(Paragraph::new(shown).block(panel(title)).wrap(Wrap { trim: false }), r);
}

fn wrap_into(rows: &mut Vec<Line<'static>>, l: &str, w: usize, style: Style) {
    let chars: Vec<char> = l.chars().collect();
    if chars.is_empty() {
        rows.push(Line::raw(""));
        return;
    }
    for chunk in chars.chunks(w) {
        rows.push(Line::styled(chunk.iter().collect::<String>(), style));
    }
}
