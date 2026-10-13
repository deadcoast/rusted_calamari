//! calamari — terminal preview of the Octobrachial Tree Format.
//!
//!   calamari <dir> [--style arm|beak|box] [--rows N] [--open PATH]...
//!                  [--click PATH]... [--full] [--plain]
//!
//! `--open` pre-opens a node silently; `--click` runs it through the tide
//! engine and prints the viewport after each click. `PATH/~` clicks ink.

use octo::paint::{self, fg, DIM, GOLD, MUTED, RESET, TEXT};
use octo::tide::{balance, click, Trip, View};
use octo::tree::Fold;
use octo::layout::Ctx;
use octo::{config, render, Side, Style, Tree};
use std::path::PathBuf;
use std::process::ExitCode;

struct Args {
    dir: PathBuf,
    style: Style,
    rows: usize,
    open: Vec<String>,
    clicks: Vec<String>,
    full: bool,
    ansi: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        dir: PathBuf::new(),
        style: Style::Arm,
        rows: 24,
        open: vec![],
        clicks: vec![],
        full: false,
        ansi: true,
    };
    let mut it = std::env::args().skip(1);
    let mut dir = None;
    while let Some(x) = it.next() {
        let mut val = |n: &str| it.next().ok_or(format!("{n} needs a value"));
        match x.as_str() {
            "--style" => a.style = Style::parse(&val("--style")?).ok_or("style: arm | beak | box")?,
            "--rows" => a.rows = val("--rows")?.parse().map_err(|_| "rows: number")?,
            "--open" => a.open.push(val("--open")?),
            "--click" => a.clicks.push(val("--click")?),
            "--full" => a.full = true,
            "--plain" => a.ansi = false,
            _ if x.starts_with("--") => return Err(format!("unknown flag {x}")),
            _ => dir = Some(PathBuf::from(x)),
        }
    }
    a.dir = dir.ok_or("usage: calamari <dir> [--style arm|beak|box] [--rows N] [--open P] [--click P] [--full] [--plain]")?;
    Ok(a)
}

struct Out {
    ansi: bool,
}

impl Out {
    fn c(&self, rgb: (u8, u8, u8), bold: bool, s: &str) -> String {
        if self.ansi { format!("{}{s}{RESET}", fg(rgb, bold)) } else { s.to_string() }
    }
}

fn arrow(s: Side) -> &'static str {
    match s {
        Side::Up => "▲",
        Side::Down => "▼",
    }
}

fn beak(f: Fold) -> &'static str {
    match f {
        Fold::Mid => "beak mid",
        Fold::Up => "beak up ",
        Fold::Down => "beak dn ",
    }
}

fn trip(o: &Out, t: &Tree, tr: &Trip) -> String {
    let need = |p: &octo::tide::Probe| format!("needs {}▲ {}▼ · room {}▲ {}▼", p.up, p.down, p.room_up, p.room_down);
    match tr {
        Trip::Leaf => o.c(MUTED, false, "LEAF      no arms to open"),
        Trip::Folded => o.c(MUTED, false, "FOLDED    arm drawn in"),
        Trip::Paged { page } => o.c((0x8a, 0x7f, 0xd0), true, &format!("INK       page {}", page + 1)),
        Trip::Balanced { up, down } => o.c(GOLD, true, &format!("MANTLE    arms balanced {up}▲ {down}▼")),
        Trip::Fit { side, fold, p } => format!(
            "{}  {}",
            o.c((0x92, 0xcc, 0x7c), true, &format!("FIT     {} {}", arrow(*side), beak(*fold))),
            o.c(MUTED, false, &need(p))
        ),
        Trip::Mirrored { side, fold, snag, p } => format!(
            "{}  {}  {}",
            o.c((0x7f, 0x96, 0xff), true, &format!("MIRROR  {} {}", arrow(*side), beak(*fold))),
            o.c(DIM, false, &format!("snag {}▲ {}▼", snag.up, snag.down)),
            o.c(MUTED, false, &need(p))
        ),
        Trip::Slid { side, fold, by, p } => format!(
            "{}  {}  {}",
            o.c((0xa6, 0xcf, 0xd5), true, &format!("SLIDE   {} {}", arrow(*side), beak(*fold))),
            o.c(DIM, false, &format!("row {by:+}")),
            o.c(MUTED, false, &need(p))
        ),
        Trip::Retracted { side, fold, arms, by, p } => format!(
            "{}  {}  {}",
            o.c((0xe5, 0x63, 0x99), true, &format!("RETRACT {} {}", arrow(*side), beak(*fold))),
            o.c(DIM, false, &format!("{} · row {by:+}", arms.iter().map(|&a| t.path(a)).collect::<Vec<_>>().join(", "))),
            o.c(MUTED, false, &need(p))
        ),
        Trip::Clipped { side, p } => format!("{}  {}", o.c((0xc4, 0x6f, 0x7f), true, &format!("CLIPPED {}       ", arrow(*side))), o.c(MUTED, false, &need(p))),
    }
}

fn frame(o: &Out, t: &Tree, style: Style, v: &View, mark: Option<usize>) {
    let f = render(t, style, &Ctx::default());
    let total = f.lines.len();
    let w = f.lines.iter().map(|l| l.width()).max().unwrap_or(0).max(40);
    let above = v.scroll;
    let below = total.saturating_sub(v.scroll + v.rows);
    let edge = |l: &str, note: String| {
        let note = if note.is_empty() { String::new() } else { format!(" {note} ") };
        let fill = (w + 2).saturating_sub(note.chars().count());
        println!("     {}", o.c(DIM, false, &format!("{l}─{note}{}", "─".repeat(fill))));
    };
    edge("┌", if above > 0 { format!("▲ {above}") } else { String::new() });
    let mark_row = mark.and_then(|id| f.row(id));
    for r in v.scroll..v.scroll + v.rows {
        let g = if Some(r) == mark_row { o.c(GOLD, true, "►") } else { " ".into() };
        let n = o.c(DIM, false, &format!("{r:>3}"));
        match f.lines.get(r) {
            Some(l) => println!(" {n}{g}{} {}", o.c(DIM, false, "│"), paint::line(t, l, o.ansi)),
            None => println!(" {n}{g}{}", o.c(DIM, false, "│")),
        }
    }
    edge("└", if below > 0 { format!("▼ {below}") } else { String::new() });
}

fn run() -> Result<(), String> {
    let a = args()?;
    let o = Out { ansi: a.ansi };
    let cats = config::load(&a.dir);
    let mut t = Tree::scan(&a.dir, cats).map_err(|e| format!("{}: {e}", a.dir.display()))?;
    balance(&mut t, a.style, &Ctx::default());
    for p in &a.open {
        let (id, _) = t.find(p).ok_or(format!("no such node: {p}"))?;
        let mut x = Some(id);
        while let Some(i) = x {
            t.nodes[i].open = true;
            x = t.nodes[i].parent;
        }
    }
    balance(&mut t, a.style, &Ctx::default());

    let sorted: usize = t.nodes.iter().flat_map(|n| &n.files).filter(|f| f.cat.is_some()).count();
    let total = t.total(0);
    println!();
    println!(
        " {}  {}  {}",
        o.c(GOLD, true, "RUSTED CALAMARI"),
        o.c(TEXT, false, a.style.name()),
        o.c(MUTED, false, &format!("· {} dirs · {total} media · {sorted} sorted · viewport {} rows", t.nodes.len(), a.rows))
    );
    let legend: Vec<String> = t
        .cats
        .iter()
        .map(|c| {
            format!(
                "{}{}",
                o.c(c.cid, true, &c.name),
                o.c(MUTED, false, &c.key.as_ref().map(|k| format!("«{k}»")).unwrap_or_default())
            )
        })
        .collect();
    println!(" {}  {}", legend.join("  "), o.c(DIM, false, "· loose"));
    println!();

    let mut v = View { rows: a.rows, scroll: 0 };
    if a.full {
        v.rows = render(&t, a.style, &Ctx::default()).lines.len();
    } else {
        let f = render(&t, a.style, &Ctx::default());
        v.scroll = f.row(0).unwrap_or(0).saturating_sub(a.rows / 2);
    }
    frame(&o, &t, a.style, &v, None);

    for p in &a.clicks {
        let (id, ink) = t.find(p).ok_or(format!("no such node: {p}"))?;
        let tr = click(&mut t, &mut v, a.style, &Ctx::default(), id, ink);
        println!();
        println!(" {} {}  {}", o.c(GOLD, true, "►"), o.c(TEXT, true, &format!("{p:<28}")), trip(&o, &t, &tr));
        frame(&o, &t, a.style, &v, Some(id));
    }
    println!();
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!(" calamari: {e}");
            ExitCode::FAILURE
        }
    }
}
