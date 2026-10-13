//! Token colouring. Spines cycle colour by depth like rainbow brackets;
//! category names and suckers take their cid.

use crate::config::Rgb;
use crate::layout::{Cls, Line};
use crate::tree::Tree;

const DEPTH: [Rgb; 6] = [
    (0x6f, 0x8f, 0xbf),
    (0xb0, 0x7f, 0xc4),
    (0x5f, 0xa8, 0x9a),
    (0xc4, 0x9a, 0x5f),
    (0xc4, 0x6f, 0x7f),
    (0x7f, 0xa8, 0x5f),
];
pub const DIM: Rgb = (0x4b, 0x51, 0x60);
pub const TEXT: Rgb = (0xd7, 0xda, 0xe0);
pub const MUTED: Rgb = (0x7a, 0x82, 0x90);
pub const GOLD: Rgb = (0xe6, 0xd4, 0x63);
pub const RUST_HI: Rgb = (0xe3, 0x88, 0x4a);
/// Cool steel distinguishes folder navigation from orange category slots.
pub const STEEL: Rgb = (0xa9, 0xbc, 0xd0);

pub fn fg(c: Rgb, bold: bool) -> String {
    format!(
        "\x1b[{}38;2;{};{};{}m",
        if bold { "1;" } else { "" },
        c.0,
        c.1,
        c.2
    )
}
pub const RESET: &str = "\x1b[0m";

pub fn colour(t: &Tree, cls: Cls) -> (Rgb, bool) {
    match cls {
        Cls::Trunk(d) | Cls::Border(d) => (DEPTH[d % DEPTH.len()], false),
        Cls::BeakTrunk(d) => (DEPTH[d % DEPTH.len()], true),
        Cls::Tier => (MUTED, false),
        Cls::Brack => ((0x5c, 0x63, 0x70), false),
        Cls::Name => (TEXT, false),
        // A line drawn from an older category table (a row fading out) may
        // name a category that is gone.
        Cls::CatName(c) => (t.cats.get(c).map_or(MUTED, |x| x.cid), true),
        Cls::Sucker(Some(c)) => (t.cats.get(c).map_or(MUTED, |x| x.cid), false),
        Cls::Sucker(None) => (DIM, false),
        Cls::Count => (MUTED, false),
        Cls::Fold => ((0xc4, 0x9a, 0x5f), false),
        Cls::Ink => ((0x8a, 0x7f, 0xd0), false),
        Cls::Mantle | Cls::Active => (STEEL, true),
        Cls::Cursor => (MUTED, false),
        Cls::Plain => (TEXT, false),
    }
}

pub fn line(t: &Tree, l: &Line, ansi: bool) -> String {
    if !ansi {
        return l.text();
    }
    let mut s = String::new();
    for sp in &l.spans {
        let (c, b) = colour(t, sp.cls);
        s.push_str(&fg(c, b));
        s.push_str(&sp.text);
        s.push_str(RESET);
    }
    s
}
