//! Tide: the on-click fit engine.
//!
//! A click never moves the clicked row. Opening a node probes its arms on
//! the preferred side; if the subtree overruns the viewport that trips a
//! snag and the beak folds, then the arms mirror to the other side. If no
//! shape fits at the pinned row the row slides the fewest rows that make
//! it fit; if the subtree is taller than the viewport, its own open
//! descendants retract (deepest first) until it fits. Each probe is one O(visible rows) render, so a click costs a
//! handful of renders at most.

use crate::layout::{height, render, Ctx, Style};
use crate::tree::{Fold, Side, Tree};

pub struct View {
    pub rows: usize,
    pub scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    pub ok: bool,
    /// Rows the opened subtree needs above / below the clicked row.
    pub up: usize,
    pub down: usize,
    /// Rows the viewport has above / below the clicked row.
    pub room_up: usize,
    pub room_down: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Trip {
    Leaf,
    Folded,
    Paged { page: usize },
    Fit { side: Side, fold: Fold, p: Probe },
    Mirrored { side: Side, fold: Fold, snag: Probe, p: Probe },
    /// No shape fits at the pinned row; the clicked row slides `by` rows.
    Slid { side: Side, fold: Fold, by: isize, p: Probe },
    /// Own open descendants folded so the subtree fits the viewport.
    Retracted { side: Side, fold: Fold, arms: Vec<usize>, by: isize, p: Probe },
    Clipped { side: Side, p: Probe },
    Balanced { up: usize, down: usize },
}

/// An arm already hanging above the mantle keeps growing upward;
/// everything else opens downward.
fn preferred(t: &Tree, id: usize) -> Side {
    if t.nodes[id].side == Side::Up && t.nodes[id].parent.is_some() { Side::Up } else { Side::Down }
}

/// Probe order. The beak flexes before the arms mirror; hier(a) sits at the
/// top of every beak, so `Fold::Up` (tiers folded above the label) is tried
/// before `Fold::Down`.
fn shapes(style: Style, pref: Side) -> Vec<(Side, Fold)> {
    let sides: &[Side] = if style == Style::Beak { &[Side::Down] } else { &[pref, pref.flip()] };
    sides.iter().flat_map(|&s| [Fold::Mid, Fold::Up, Fold::Down].map(|f| (s, f))).collect()
}

fn shape(t: &mut Tree, id: usize, (side, fold): (Side, Fold)) {
    set_side(t, id, side);
    t.nodes[id].beak = fold;
}

fn set_side(t: &mut Tree, id: usize, side: Side) {
    for k in t.nodes[id].kids.clone() {
        t.nodes[k].side = side;
    }
}

/// Render, pin the clicked row at screen row `rs`, measure the subtree.
fn probe(t: &Tree, v: &mut View, style: Style, ctx: &Ctx, id: usize, rs: usize) -> Probe {
    let f = render(t, style, ctx);
    let row = f.row(id).unwrap_or(0);
    let (lo, hi) = f.span(t, id);
    let s = row.saturating_sub(rs);
    v.scroll = s;
    let rs = row - s;
    let room_up = rs;
    let room_down = v.rows.saturating_sub(rs + 1);
    let up = row - lo;
    let down = hi - row;
    Probe { ok: up <= room_up && down <= room_down, up, down, room_up, room_down }
}

/// After something shrinks, pull hidden rows back down into blank space.
fn fill(t: &Tree, v: &mut View, style: Style, ctx: &Ctx) {
    let total = render(t, style, ctx).lines.len();
    let blank = (v.scroll + v.rows).saturating_sub(total);
    v.scroll -= blank.min(v.scroll);
}

/// Open descendants of the clicked node, deepest first: re-opening a node
/// restores the arms it had open, and those are what retract to make room.
fn retract_order(t: &Tree, id: usize) -> Vec<usize> {
    fn walk(t: &Tree, x: usize, d: usize, out: &mut Vec<(usize, usize)>) {
        for &k in &t.nodes[x].kids {
            if t.nodes[k].open && !t.nodes[k].kids.is_empty() {
                walk(t, k, d + 1, out);
                out.push((d, k));
            }
        }
    }
    let mut v = Vec::new();
    walk(t, id, 1, &mut v);
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().map(|(_, k)| k).collect()
}

/// Shift (in rows) the clicked row must slide for the subtree to fit, or
/// None if the subtree is taller than the viewport. Positive = down.
fn slide(p: &Probe, rows: usize) -> Option<isize> {
    if p.up + p.down + 1 > rows {
        return None;
    }
    Some(if p.up > p.room_up {
        (p.up - p.room_up) as isize
    } else if p.down > p.room_down {
        -((p.down - p.room_down) as isize)
    } else {
        0
    })
}

/// Try every shape at the pinned row, then the smallest slide.
fn place(t: &mut Tree, v: &mut View, style: Style, ctx: &Ctx, id: usize, rs: usize, order: &[(Side, Fold)]) -> Option<((Side, Fold), Probe, isize)> {
    let mut best: Option<((Side, Fold), Probe, isize)> = None;
    for &sh in order {
        shape(t, id, sh);
        let p = probe(t, v, style, ctx, id, rs);
        if let Some(d) = slide(&p, v.rows)
            && best.as_ref().is_none_or(|b| d.abs() < b.2.abs())
        {
            best = Some((sh, p, d));
            if d == 0 {
                break;
            }
        }
    }
    let (sh, _, d) = best?;
    shape(t, id, sh);
    let rs2 = (rs as isize + d) as usize;
    let p = probe(t, v, style, ctx, id, rs2);
    Some((sh, p, d))
}

/// Split the mantle's arms above and below it: hier(a) above, the rest
/// below, then move arms up while that evens the body.
pub fn balance(t: &mut Tree, style: Style, ctx: &Ctx) -> (usize, usize) {
    let kids = t.nodes[0].kids.clone();
    for &k in &kids {
        t.nodes[k].side = if t.tier(k) == Some('a') { Side::Up } else { Side::Down };
    }
    let h: Vec<usize> = kids.iter().map(|&k| height(t, k, style, ctx)).collect();
    let sum = |t: &Tree, s: Side| -> usize {
        kids.iter().zip(&h).filter(|(k, _)| t.nodes[**k].side == s).map(|(_, h)| h).sum()
    };
    loop {
        let (up, down) = (sum(t, Side::Up), sum(t, Side::Down));
        let Some(i) = kids.iter().position(|&k| t.nodes[k].side == Side::Down) else { break };
        let (nu, nd) = (up + h[i], down - h[i]);
        if nu.abs_diff(nd) < up.abs_diff(down) {
            t.nodes[kids[i]].side = Side::Up;
        } else {
            break;
        }
    }
    (sum(t, Side::Up), sum(t, Side::Down))
}

pub fn click(t: &mut Tree, v: &mut View, style: Style, ctx: &Ctx, id: usize, on_ink: bool) -> Trip {
    let f = render(t, style, ctx);
    let row = if on_ink { f.ink_row(id) } else { f.row(id) }.unwrap_or(0);
    if row < v.scroll || row >= v.scroll + v.rows {
        v.scroll = row.saturating_sub(v.rows / 2);
    }
    let rs = row - v.scroll;

    if on_ink {
        t.nodes[id].page += 1;
        let f = render(t, style, ctx);
        let r = f.ink_row(id).unwrap_or(row);
        v.scroll = r.saturating_sub(rs);
        fill(t, v, style, ctx);
        return Trip::Paged { page: t.nodes[id].page };
    }
    if t.nodes[id].kids.is_empty() {
        return Trip::Leaf;
    }
    if t.nodes[id].open {
        t.nodes[id].open = false;
        probe(t, v, style, ctx, id, rs);
        fill(t, v, style, ctx);
        return Trip::Folded;
    }
    t.nodes[id].open = true;
    if t.nodes[id].parent.is_none() {
        let (up, down) = balance(t, style, ctx);
        probe(t, v, style, ctx, id, rs);
        return Trip::Balanced { up, down };
    }

    let pref = preferred(t, id);
    let order = shapes(style, pref);

    let mut snag = None;
    for &sh in &order {
        shape(t, id, sh);
        let p = probe(t, v, style, ctx, id, rs);
        if p.ok {
            let (side, fold) = sh;
            return match snag {
                None => Trip::Fit { side, fold, p },
                Some(s) => Trip::Mirrored { side, fold, snag: s, p },
            };
        }
        snag.get_or_insert(p);
    }

    if let Some(((side, fold), p, by)) = place(t, v, style, ctx, id, rs, &order) {
        return Trip::Slid { side, fold, by, p };
    }
    let mut gone = Vec::new();
    for victim in retract_order(t, id) {
        t.nodes[victim].open = false;
        gone.push(victim);
        if let Some(((side, fold), p, by)) = place(t, v, style, ctx, id, rs, &order) {
            return Trip::Retracted { side, fold, arms: gone, by, p };
        }
    }

    // Nothing left to retract: open toward the larger room and let the
    // clicked row slide to that edge.
    let p = probe(t, v, style, ctx, id, rs);
    let side = if p.room_up > p.room_down && style != Style::Beak { Side::Up } else { pref };
    shape(t, id, (side, if side == Side::Up { Fold::Up } else { Fold::Down }));
    let p = probe(t, v, style, ctx, id, rs);
    let f = render(t, style, ctx);
    let row = f.row(id).unwrap_or(0);
    v.scroll = match side {
        Side::Down => row.saturating_sub(p.up),
        Side::Up => (row + 1 + p.down).saturating_sub(v.rows),
    };
    Trip::Clipped { side, p }
}
