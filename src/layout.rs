//! Layout: tree -> tagged glyph lines.
//!
//! Every subtree renders to a `Block` (lines + the row its own label sits
//! on, the anchor). A directory has two kinds of children:
//!
//! * the **beak** — its category directories, one row per hier tier
//!   (`[_float_:6]`, `[1:9, 2:4, 3:7]`), fanned off the label row.
//!   The beak is the sorting mouth: the targets the active image can go to.
//! * **arms** — every other subdirectory, hung on a spine above (mirrored)
//!   or below the label.
//!
//! A node shows at most eight: the beak counts as one, the rest are arms;
//! overflow pages through an ink row.
//!
//! The tree reflows to the width it is given (`fit`): sucker bars shorten,
//! then the beak stops fanning right and hangs under its label on the
//! spine (bold `├─`), then the bars go, then long names are cut and slot lists
//! tighten (`[1:4,2,3:2]`). Every line carries a stable `Key` so a viewer
//! can animate rows between renders.

use crate::tree::{FileEntry, Fold, Side, Tree};

pub const ARMS: usize = 8;
const SUCKER_CELLS: usize = 10;

/// Reflow levels, widest first: (sucker cells, stacked beak, name limit,
/// tight slot lists).
pub const REFLOW: [(usize, bool, usize, bool); 8] = [
    (SUCKER_CELLS, false, 0, false),
    (6, false, 0, false),
    (3, false, 0, false),
    (3, true, 0, false),
    (0, true, 0, false),
    (0, true, 16, true),
    (0, true, 10, true),
    (0, true, 6, true),
];

/// What a line is, independent of where it lands: the same key on two
/// renders is the same row, moved.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Key {
    /// The row a node's label sits on (the outermost label, when a fan
    /// pins a parent onto a child's row).
    Label(usize),
    /// One hier tier of a node's beak.
    Beak(usize, char),
    /// Top or bottom edge of a node's Boxtobrachial beak.
    Border(usize, bool),
    Ink(usize),
}

impl Key {
    /// The node whose row a new line grows out of.
    pub fn anchor(self, t: &Tree) -> Option<usize> {
        match self {
            Key::Label(n) => t.nodes[n].parent,
            Key::Beak(n, _) | Key::Border(n, _) | Key::Ink(n) => Some(n),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    /// Beak fanned off the label, arms on a spine.
    Arm,
    /// Beak and arms share one fan; no spine anywhere.
    Beak,
    /// Beak enclosed in a box, arms on a spine.
    Boxed,
}

impl Style {
    pub fn parse(s: &str) -> Option<Style> {
        match s.to_ascii_lowercase().as_str() {
            "arm" => Some(Style::Arm),
            "beak" => Some(Style::Beak),
            "box" | "boxed" | "boxtobrachial" => Some(Style::Boxed),
            _ => None,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Style::Arm => "OCTOBRACHIAL ARM",
            Style::Beak => "OCTOBRACHIAL BEAK",
            Style::Boxed => "BOXTOBRACHIAL",
        }
    }
}

/// Token classes; the painter maps them to colours like lexer tokens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cls {
    /// Spine/connector glyphs; depth drives a rainbow-bracket cycle.
    Trunk(usize),
    /// Stacked category connector: a bold single rule in the spine colour.
    BeakTrunk(usize),
    Border(usize),
    Tier,
    Brack,
    Name,
    CatName(usize),
    Sucker(Option<usize>),
    Count,
    Fold,
    Ink,
    Mantle,
    /// The folder the viewer is sorting from.
    Active,
    /// Position of the active image inside the active folder.
    Cursor,
    Plain,
}

#[derive(Clone, Debug)]
pub struct Span {
    pub cls: Cls,
    pub text: String,
    /// Node this glyph run belongs to (label, slot or ink), for clicks.
    pub node: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Label,
    Beak,
    Border,
    Ink,
}

#[derive(Clone, Debug)]
pub struct Line {
    pub spans: Vec<Span>,
    /// Nodes with glyphs on this row (subtree extents are built from these).
    pub tags: Vec<usize>,
    /// Nodes whose label sits on this row.
    pub heads: Vec<usize>,
    pub kind: Kind,
    pub key: Key,
}

impl Line {
    pub fn width(&self) -> usize {
        self.spans.iter().map(|s| s.text.chars().count()).sum()
    }
    pub fn text(&self) -> String {
        self.spans.iter().map(|s| s.text.as_str()).collect()
    }
    fn front(mut self, mut pre: Vec<Span>) -> Line {
        pre.append(&mut self.spans);
        self.spans = pre;
        self
    }
    /// Run a spine through this row's left padding at column 0.
    fn spine(&mut self, c: Cls) {
        if let Some(f) = self.spans.first_mut()
            && f.cls == Cls::Plain
            && f.text.starts_with(' ')
        {
            let rest: String = f.text.chars().skip(1).collect();
            f.text = rest;
            self.spans.insert(0, sp(c, "│"));
        }
    }
}

fn sp(cls: Cls, text: impl Into<String>) -> Span {
    Span {
        cls,
        text: text.into(),
        node: None,
    }
}

fn own(mut s: Span, node: Option<usize>) -> Span {
    s.node = node;
    s
}

#[derive(Clone, Debug)]
pub struct Block {
    pub lines: Vec<Line>,
    pub anchor: usize,
}

impl Block {
    fn one(l: Line) -> Block {
        Block {
            lines: vec![l],
            anchor: 0,
        }
    }
}

struct Arm {
    b: Block,
    side: Side,
    /// A beak tier hung on the spine (stacked reflow): drawn with bold `├─`.
    beak: bool,
}

fn arm(b: Block, side: Side) -> Arm {
    Arm {
        b,
        side,
        beak: false,
    }
}

/// Category children grouped into one beak row per tier. The active
/// folder shows every category as a slot, including ones not yet created.
fn beak_rows(t: &Tree, id: usize, active: bool, ctx: &Ctx) -> Vec<Line> {
    let mut slots: Vec<(usize, Option<usize>)> = if active {
        (0..t.cats.len())
            .map(|c| {
                (
                    c,
                    t.nodes[id]
                        .kids
                        .iter()
                        .copied()
                        .find(|&k| t.nodes[k].own_cat == Some(c)),
                )
            })
            .collect()
    } else {
        t.nodes[id]
            .kids
            .iter()
            .filter_map(|&k| t.nodes[k].own_cat.map(|c| (c, Some(k))))
            .collect()
    };
    slots.sort_by_key(|&(c, _)| (t.cats[c].tier, c));
    let mut rows: Vec<Line> = Vec::new();
    let mut cur: Option<char> = None;
    for (c, k) in slots {
        let tier = t.cats[c].tier;
        let n = k.map_or(0, |k| t.total(k));
        if Some(tier) != cur {
            if let Some(r) = rows.last_mut() {
                r.spans.push(sp(Cls::Brack, "]"));
            }
            rows.push(Line {
                spans: vec![sp(Cls::Brack, "[")],
                tags: vec![],
                heads: vec![],
                kind: Kind::Beak,
                key: Key::Beak(id, tier),
            });
            cur = Some(tier);
        } else {
            rows.last_mut()
                .unwrap()
                .spans
                .push(sp(Cls::Brack, if ctx.tight { "," } else { ", " }));
        }
        let r = rows.last_mut().unwrap();
        r.spans
            .push(own(sp(Cls::CatName(c), t.cats[c].name.clone()), k));
        // Tight rows drop the `:0` of empty slots; the name still takes clicks.
        if n > 0 || !ctx.tight {
            r.spans.push(own(
                sp(
                    if n == 0 {
                        Cls::Sucker(None)
                    } else {
                        Cls::Count
                    },
                    format!(":{n}"),
                ),
                k,
            ));
        }
        if let Some(kk) = k
            && k == ctx.active
        {
            // Browsing a slot: it is the active folder, marked in place.
            let len = t.nodes[kk].files.len();
            r.spans.push(own(
                sp(
                    Cls::Cursor,
                    format!(
                        " ◄{}/{}",
                        ctx.cursor.min(len.saturating_sub(1)) + usize::from(len > 0),
                        len
                    ),
                ),
                k,
            ));
        }
        if let Some(k) = k {
            r.tags.push(k);
        }
    }
    if let Some(r) = rows.last_mut() {
        r.spans.push(sp(Cls::Brack, "]"));
    }
    rows
}

/// Visible arms (non-category subdirs) and ink paging.
/// Returns (visible, hidden, page, pages).
pub fn arms(t: &Tree, id: usize, active: bool) -> (Vec<usize>, usize, usize, usize) {
    let plain: Vec<usize> = t.nodes[id]
        .kids
        .iter()
        .copied()
        .filter(|&k| t.nodes[k].own_cat.is_none())
        .collect();
    let has_beak = active || plain.len() < t.nodes[id].kids.len();
    let cap = ARMS - usize::from(has_beak);
    if plain.len() <= cap {
        return (plain, 0, 0, 1);
    }
    let per = cap - 1;
    let pages = plain.len().div_ceil(per);
    let p = t.nodes[id].page % pages;
    let s = p * per;
    let v: Vec<usize> = plain[s..(s + per).min(plain.len())].to_vec();
    let hidden = plain.len() - v.len();
    (v, hidden, p, pages)
}

fn suckers(files: &[FileEntry], cells_max: usize) -> Vec<Span> {
    let files: Vec<Option<usize>> = files.iter().map(|f| f.cat).collect();
    let key = |c: &Option<usize>| c.unwrap_or(usize::MAX);
    let cells: Vec<Option<usize>> = if files.len() <= cells_max {
        let mut f = files.clone();
        f.sort_by_key(key);
        f
    } else {
        // Proportional cells by largest remainder; sorted first, loose last.
        let mut counts: Vec<(Option<usize>, usize)> = Vec::new();
        for &f in &files {
            match counts.iter_mut().find(|(c, _)| *c == f) {
                Some((_, n)) => *n += 1,
                None => counts.push((f, 1)),
            }
        }
        counts.sort_by_key(|(c, _)| key(c));
        let total = files.len();
        let mut alloc: Vec<(Option<usize>, usize, usize)> = counts
            .iter()
            .map(|&(c, n)| (c, n * cells_max / total, n * cells_max % total))
            .collect();
        let mut left = cells_max - alloc.iter().map(|a| a.1).sum::<usize>();
        let mut order: Vec<usize> = (0..alloc.len()).collect();
        order.sort_by(|&a, &b| alloc[b].2.cmp(&alloc[a].2));
        for i in order {
            if left == 0 {
                break;
            }
            alloc[i].1 += 1;
            left -= 1;
        }
        alloc
            .iter()
            .flat_map(|&(c, n, _)| std::iter::repeat_n(c, n))
            .collect()
    };
    let mut out: Vec<Span> = Vec::new();
    for c in cells {
        let g = if c.is_some() { '■' } else { '·' };
        match out.last_mut() {
            Some(s) if s.cls == Cls::Sucker(c) => s.text.push(g),
            _ => out.push(sp(Cls::Sucker(c), g.to_string())),
        }
    }
    out
}

fn label(t: &Tree, id: usize, ctx: &Ctx) -> Line {
    let n = &t.nodes[id];
    let mut s = Vec::new();
    if n.parent.is_none() {
        s.push(sp(Cls::Mantle, "○ "));
    }
    let on = ctx.active == Some(id);
    s.push(sp(Cls::Brack, "["));
    s.push(sp(
        if on { Cls::Active } else { Cls::Name },
        cut(&n.name, ctx.names),
    ));
    s.push(sp(Cls::Brack, "]"));
    if on && !n.files.is_empty() {
        s.push(sp(
            Cls::Cursor,
            format!(" {}/{}", ctx.cursor + 1, n.files.len()),
        ));
    }
    if !n.files.is_empty() {
        if ctx.suckers > 0 {
            s.push(sp(Cls::Plain, " "));
            s.extend(suckers(&n.files, ctx.suckers));
        }
        s.push(sp(Cls::Count, format!(" {}", n.files.len())));
    }
    if n.parent.is_none() {
        s.push(sp(Cls::Mantle, format!(" Σ{}", t.total(id))));
    }
    if !n.open && !n.kids.is_empty() {
        s.push(sp(Cls::Fold, format!(" ►{}", n.kids.len())));
    }
    for x in &mut s {
        x.node = Some(id);
    }
    Line {
        spans: s,
        tags: vec![id],
        heads: vec![id],
        kind: Kind::Label,
        key: Key::Label(id),
    }
}

/// A folder name cut to `max` columns with `..`; 0 never cuts.
fn cut(name: &str, max: usize) -> String {
    if max == 0 || name.chars().count() <= max {
        return name.to_string();
    }
    let mut s: String = name.chars().take(max.saturating_sub(2).max(1)).collect();
    s.push_str("..");
    s
}

fn ink(id: usize, hidden: usize, p: usize, pages: usize) -> Line {
    Line {
        spans: vec![own(
            sp(Cls::Ink, format!("~[ink +{hidden}] {}/{pages} ►", p + 1)),
            Some(id),
        )],
        tags: vec![id],
        heads: vec![],
        kind: Kind::Ink,
        key: Key::Ink(id),
    }
}

/// What the viewer contributes to the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ctx {
    pub active: Option<usize>,
    pub cursor: usize,
    /// Cells in a sucker bar; 0 drops the bar and keeps the count.
    pub suckers: usize,
    /// The beak hangs under its label on the spine instead of fanning right.
    pub stack: bool,
    /// Longest folder name before it is cut with `..`; 0 never cuts.
    pub names: usize,
    /// Slots in a beak row are separated by `,` alone, and empty slots
    /// show their name without `:0`.
    pub tight: bool,
    /// Width to reflow into, in columns: `render` picks the widest level
    /// that fits. 0 draws at this context's own level.
    pub cols: usize,
}

impl Default for Ctx {
    fn default() -> Self {
        Ctx {
            active: None,
            cursor: 0,
            suckers: SUCKER_CELLS,
            stack: false,
            names: 0,
            tight: false,
            cols: 0,
        }
    }
}

impl Ctx {
    pub fn viewer(active: usize, cursor: usize) -> Ctx {
        Ctx {
            active: Some(active),
            cursor,
            ..Ctx::default()
        }
    }
    /// This context at reflow level `l` (see `REFLOW`).
    pub fn level(self, l: usize) -> Ctx {
        let (suckers, stack, names, tight) = REFLOW[l.min(REFLOW.len() - 1)];
        Ctx {
            suckers,
            stack,
            names,
            tight,
            ..self
        }
    }
}

/// Beak tiers as arms hung on the spine, for the stacked reflow.
fn beak_arms(rows: Vec<Line>) -> Vec<Arm> {
    rows.into_iter()
        .map(|l| Arm {
            b: Block::one(l),
            side: Side::Down,
            beak: true,
        })
        .collect()
}

fn block(t: &Tree, id: usize, depth: usize, style: Style, ctx: &Ctx) -> Block {
    block_with_labels(t, id, depth, style, ctx, &|id, cx| label(t, id, cx))
}

fn block_with_labels(
    t: &Tree,
    id: usize,
    depth: usize,
    style: Style,
    ctx: &Ctx,
    labels: &dyn Fn(usize, &Ctx) -> Line,
) -> Block {
    let lab = labels(id, ctx);
    let n = &t.nodes[id];
    let is_active = ctx.active == Some(id);
    if !is_active && (!n.open || n.kids.is_empty()) {
        return Block::one(lab);
    }
    if !n.open && is_active {
        // A folded active folder still shows its slots.
        let rows = beak_rows(t, id, true, ctx);
        if rows.is_empty() {
            return Block::one(lab);
        }
        if ctx.stack {
            return trunk(Block::one(lab), beak_arms(rows), depth);
        }
        let beak: Vec<Block> = rows.into_iter().map(Block::one).collect();
        return match style {
            Style::Boxed => boxed(id, lab, beak, depth, n.beak),
            _ => fan(lab, beak, depth, n.beak),
        };
    }
    let rows = beak_rows(t, id, ctx.active == Some(id), ctx);
    let (vis, hidden, p, pages) = arms(t, id, ctx.active == Some(id));
    let mut kids: Vec<Arm> = vis
        .iter()
        .map(|&k| {
            arm(
                block_with_labels(t, k, depth + 1, style, ctx, labels),
                t.nodes[k].side,
            )
        })
        .collect();
    if hidden > 0 {
        let side = kids.last().map_or(Side::Down, |k| k.side);
        kids.push(arm(Block::one(ink(id, hidden, p, pages)), side));
    }
    if ctx.stack {
        // Narrow: every style converges on one spine, beak tiers first.
        let mut all = beak_arms(rows);
        all.extend(kids);
        return trunk(Block::one(lab), all, depth);
    }
    let fold = n.beak;
    let beak: Vec<Block> = rows.into_iter().map(Block::one).collect();
    match style {
        Style::Beak => {
            let mut all = beak;
            all.extend(kids.into_iter().map(|k| k.b));
            fan(lab, all, depth, fold)
        }
        Style::Arm => {
            let head = if beak.is_empty() {
                Block::one(lab)
            } else {
                fan(lab, beak, depth, fold)
            };
            trunk(head, kids, depth)
        }
        Style::Boxed => {
            let head = if beak.is_empty() {
                Block::one(lab)
            } else {
                boxed(id, lab, beak, depth, fold)
            };
            trunk(head, kids, depth)
        }
    }
}

/// Spine composition. Up arms are mirrored: first arm nearest the label,
/// farthest on top, corner glyphs flipped.
fn trunk(head: Block, kids: Vec<Arm>, depth: usize) -> Block {
    let c = Cls::Trunk(depth);
    let (ups, downs): (Vec<Arm>, Vec<Arm>) = kids.into_iter().partition(|k| k.side == Side::Up);
    let mut out = Vec::new();
    let nu = ups.len();
    for (i, k) in ups.into_iter().enumerate().rev() {
        let far = i == nu - 1;
        let a = k.b.anchor;
        for (r, l) in k.b.lines.into_iter().enumerate() {
            let pre = if r == a {
                if far { "┌─ " } else { "├─ " }
            } else if r < a && far {
                "   "
            } else {
                "│  "
            };
            out.push(l.front(vec![sp(c, pre)]));
        }
    }
    let (has_up, has_down) = (nu > 0, !downs.is_empty());
    let anchor = out.len() + head.anchor;
    for (r, mut l) in head.lines.into_iter().enumerate() {
        if (r < head.anchor && has_up) || (r > head.anchor && has_down) {
            l.spine(c);
        }
        out.push(l);
    }
    let nd = downs.len();
    for (i, k) in downs.into_iter().enumerate() {
        let last = i == nd - 1;
        let a = k.b.anchor;
        for (r, l) in k.b.lines.into_iter().enumerate() {
            let pre = if r == a {
                if last { "└─ " } else { "├─ " }
            } else if r > a && last {
                "   "
            } else {
                "│  "
            };
            let cls = if r == a && k.beak {
                Cls::BeakTrunk(depth)
            } else {
                c
            };
            out.push(l.front(vec![sp(cls, pre)]));
        }
    }
    Block { lines: out, anchor }
}

/// The beak: children stacked to the right, the label pinned to one of
/// them by `fold`, so the fan spends no row of its own.
fn fan(lab: Line, kids: Vec<Block>, depth: usize, fold: Fold) -> Block {
    let c = Cls::Trunk(depth);
    let mut rows: Vec<Line> = Vec::new();
    let mut anchors = Vec::new();
    for k in kids {
        anchors.push(rows.len() + k.anchor);
        rows.extend(k.lines);
    }
    let (a0, al) = (anchors[0], *anchors.last().unwrap());
    let last = anchors.len() - 1;
    let p = match fold {
        Fold::Mid => anchors[last / 2],
        Fold::Up => al,
        Fold::Down => a0,
    };
    let lw = lab.width() + 2;
    let mut lab = Some(lab);
    let mut out = Vec::with_capacity(rows.len());
    for (r, mut l) in rows.into_iter().enumerate() {
        let ai = anchors.iter().position(|&a| a == r);
        let g = if last == 0 {
            if r == p { '─' } else { ' ' }
        } else {
            match (ai, r == p) {
                (Some(0), true) => '┬',
                (Some(i), true) if i == last => '┴',
                (Some(_), true) => '┼',
                (None, true) => '┤',
                (Some(0), false) => '┌',
                (Some(i), false) if i == last => '└',
                (Some(_), false) => '├',
                (None, false) => {
                    if r > a0 && r < al {
                        '│'
                    } else {
                        ' '
                    }
                }
            }
        };
        let tail = if ai.is_some() { "─ " } else { "  " };
        let mut pre = Vec::new();
        if r == p {
            let pl = lab.take().unwrap();
            l.tags.extend(pl.tags.iter().copied());
            l.heads.extend(pl.heads.iter().copied());
            // The row now belongs to the outermost label on it.
            l.key = pl.key;
            pre.extend(pl.spans);
            pre.push(sp(c, " ─"));
        } else {
            pre.push(sp(Cls::Plain, " ".repeat(lw)));
        }
        pre.push(sp(c, format!("{g}{tail}")));
        out.push(l.front(pre));
    }
    Block {
        lines: out,
        anchor: p,
    }
}

/// Boxtobrachial beak: tier rows in a box hooked to the label with ─┤.
fn boxed(id: usize, lab: Line, rows: Vec<Block>, depth: usize, fold: Fold) -> Block {
    let c = Cls::Border(depth);
    let rows: Vec<Line> = rows.into_iter().flat_map(|b| b.lines).collect();
    let n = rows.len();
    let at = match fold {
        Fold::Mid => (n - 1) / 2,
        Fold::Up => n - 1,
        Fold::Down => 0,
    };
    let w = rows.iter().map(Line::width).max().unwrap_or(0);
    let lw = lab.width() + 3;
    let tags: Vec<usize> = rows.iter().flat_map(|l| l.tags.clone()).collect();
    let edge = |l: &str, r: &str, top: bool| Line {
        spans: vec![
            sp(Cls::Plain, " ".repeat(lw)),
            sp(c, format!("{l}{}{r}", "─".repeat(w + 2))),
        ],
        tags: tags.clone(),
        heads: vec![],
        kind: Kind::Border,
        key: Key::Border(id, top),
    };
    let mut out = vec![edge("┌", "┐", true)];
    let mut lab = Some(lab);
    for (i, mut l) in rows.into_iter().enumerate() {
        let pad = w - l.width();
        l.spans.push(sp(c, format!("{} │", " ".repeat(pad))));
        let pre = if i == at {
            let pl = lab.take().unwrap();
            l.tags.extend(pl.tags.iter().copied());
            l.heads.extend(pl.heads.iter().copied());
            l.key = pl.key;
            let mut v = pl.spans;
            v.push(sp(c, " ──┤ "));
            v
        } else {
            vec![sp(Cls::Plain, " ".repeat(lw)), sp(c, "│ ")]
        };
        out.push(l.front(pre));
    }
    out.push(edge("└", "┘", false));
    Block {
        lines: out,
        anchor: at + 1,
    }
}

/// Rendered tree plus row lookups for the tide engine.
pub struct Frame {
    pub lines: Vec<Line>,
}

impl Frame {
    pub fn row(&self, id: usize) -> Option<usize> {
        self.lines.iter().position(|l| l.heads.contains(&id))
    }
    /// Row a node shows on: its label, or for a category folder, the beak
    /// row that holds its slot.
    pub fn row_of(&self, id: usize) -> Option<usize> {
        self.row(id).or_else(|| {
            self.lines
                .iter()
                .position(|l| l.kind == Kind::Beak && l.tags.contains(&id))
        })
    }
    /// Widest line, in columns.
    pub fn width(&self) -> usize {
        self.lines.iter().map(Line::width).max().unwrap_or(0)
    }
    pub fn ink_row(&self, id: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|l| l.kind == Kind::Ink && l.tags[0] == id)
    }
    /// First and last rows a subtree occupies.
    pub fn span(&self, t: &Tree, id: usize) -> (usize, usize) {
        let mut lo = usize::MAX;
        let mut hi = 0;
        for (r, l) in self.lines.iter().enumerate() {
            if l.tags.iter().any(|&x| t.is_under(x, id)) {
                lo = lo.min(r);
                hi = hi.max(r);
            }
        }
        (lo, hi)
    }
}

/// The tree as glyph lines. With `ctx.cols` set, reflowed to that width;
/// the tide renders through here too, so its probes see the same shape the
/// viewer paints.
pub fn render(t: &Tree, style: Style, ctx: &Ctx) -> Frame {
    if ctx.cols > 0 {
        return fit(t, style, ctx, ctx.cols).0;
    }
    Frame {
        lines: block(t, 0, 0, style, ctx).lines,
    }
}

/// Render at the widest reflow level that fits `cols`. Returns the frame
/// and the level used; if nothing fits, the narrowest.
pub fn fit(t: &Tree, style: Style, ctx: &Ctx, cols: usize) -> (Frame, usize) {
    let mut last = None;
    for l in 0..REFLOW.len() {
        let f = render(
            t,
            style,
            &Ctx {
                cols: 0,
                ..ctx.level(l)
            },
        );
        if f.width() <= cols {
            return (f, l);
        }
        last = Some((f, l));
    }
    last.unwrap()
}

pub fn height(t: &Tree, id: usize, style: Style, ctx: &Ctx) -> usize {
    block(t, id, 1, style, ctx).lines.len()
}

/// Reuse OTF composition, paging, mirroring and reflow with labels supplied
/// by a read-only filesystem projection. Category rendering is unchanged.
pub fn outline(
    t: &Tree,
    style: Style,
    cols: usize,
    labels: &dyn Fn(usize, &Ctx) -> Line,
) -> (Frame, Ctx) {
    let mut last = None;
    for level in 0..REFLOW.len() {
        let cx = Ctx::default().level(level);
        let frame = Frame {
            lines: block_with_labels(t, 0, 0, style, &cx, labels).lines,
        };
        if cols == 0 || frame.width() <= cols {
            return (frame, cx);
        }
        last = Some((frame, cx));
    }
    last.unwrap()
}
