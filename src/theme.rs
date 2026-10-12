//! The interface kit. Retro delivery: text sits on the character grid in
//! the chosen CP437 face, with the menu bar, lit Alt letters and inverted
//! hover rows of a DOS-era text UI. Modern amplification: frames are vector
//! rules with rounded corners (the shape of `╭─╮`, which no CP437 face
//! carries), soft shadows, fades and a short lift on open.
//!
//! Palette: iron and rust. The image stage stays neutral grey so the
//! interface never tints the photographs it frames.

use crate::glyph::Grid;
use eframe::egui::{self, Area, Color32, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};

/// Stage behind the image: neutral, so colour judgement isn't skewed.
pub const STAGE: Color32 = Color32::from_rgb(0x17, 0x18, 0x1a);
/// Tree panel, a half-step off the stage.
pub const PANEL: Color32 = Color32::from_rgb(0x1c, 0x1d, 0x20);
/// Bars and dialog bodies.
pub const IRON: Color32 = Color32::from_rgb(0x25, 0x27, 0x2b);
/// Row hover inside dialogs.
pub const IRON_HI: Color32 = Color32::from_rgb(0x31, 0x34, 0x39);
/// Rules and borders.
pub const EDGE: Color32 = Color32::from_rgb(0x62, 0x65, 0x6b);
pub const TEXT: Color32 = Color32::from_rgb(0xdd, 0xd8, 0xcc);
pub const MUTED: Color32 = Color32::from_rgb(0x8e, 0x8b, 0x84);
pub const DIM: Color32 = Color32::from_rgb(0x5a, 0x5c, 0x60);
/// Oxidised iron: the one accent.
pub const RUST: Color32 = Color32::from_rgb(0xb2, 0x52, 0x2a);
pub const RUST_HI: Color32 = Color32::from_rgb(0xe3, 0x88, 0x4a);
pub const WARN: Color32 = Color32::from_rgb(0xe0, 0x5a, 0x5a);
pub const SHADOW: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 120);

/// A run of text, optionally carrying an action.
#[derive(Clone)]
pub struct R<A> {
    pub t: String,
    pub c: Color32,
    pub a: Option<A>,
    pub bg: Option<Color32>,
    pub dbl: Option<A>,
}

pub fn r<A>(t: impl Into<String>, c: Color32) -> R<A> {
    R { t: t.into(), c, a: None, bg: None, dbl: None }
}

pub fn ra<A>(t: impl Into<String>, c: Color32, a: A) -> R<A> {
    R { t: t.into(), c, a: Some(a), bg: None, dbl: None }
}

/// A keycap: text on a solid colour.
pub fn chip<A>(t: impl Into<String>, fg: Color32, bg: Color32) -> R<A> {
    R { t: t.into(), c: fg, a: None, bg: Some(bg), dbl: None }
}

/// A keycap that answers a click. On hover it keeps its colour and takes a
/// ring, so the key still reads as its category.
pub fn chipa<A>(t: impl Into<String>, fg: Color32, bg: Color32, a: A) -> R<A> {
    R { t: t.into(), c: fg, a: Some(a), bg: Some(bg), dbl: None }
}

/// A run that only answers a double-click; single clicks pass it by.
pub fn rd<A>(t: impl Into<String>, c: Color32, dbl: A) -> R<A> {
    R { t: t.into(), c, a: None, bg: None, dbl: Some(dbl) }
}

#[derive(Clone)]
pub struct Row<A> {
    pub runs: Vec<R<A>>,
    /// Clicking anywhere on the row does this (unless a run says otherwise).
    pub act: Option<A>,
}

impl<A> Row<A> {
    pub fn new(runs: Vec<R<A>>) -> Self {
        Row { runs, act: None }
    }
    pub fn act(runs: Vec<R<A>>, a: A) -> Self {
        Row { runs, act: Some(a) }
    }
    pub fn blank() -> Self {
        Row { runs: Vec::new(), act: None }
    }
    pub fn cols(&self) -> usize {
        self.runs.iter().map(|x| x.t.chars().count()).sum()
    }
}

pub struct Out<A> {
    pub click: Option<A>,
    /// Where the clicked run sits on screen, when a run (not the row) took
    /// the click; popovers anchor to it.
    pub at: Option<Rect>,
    /// A double-clicked run's action, and where that run sits on screen.
    pub double: Option<(A, Rect)>,
    pub hovered: bool,
    pub rect: Rect,
}

fn paint_runs<A>(g: &Grid, p: &egui::Painter, pos: Pos2, runs: &[R<A>], alpha: f32, hot: Option<usize>) {
    let mut x = pos.x;
    let mut out: Vec<(String, Color32)> = Vec::with_capacity(runs.len());
    for (i, run) in runs.iter().enumerate() {
        let w = run.t.chars().count() as f32 * g.cw;
        let cell = Rect::from_min_size(Pos2::new(x, pos.y + 1.0), Vec2::new(w, g.rh - 2.0));
        if let Some(bg) = run.bg {
            p.rect_filled(cell, 3.0, bg.gamma_multiply(alpha));
        }
        if hot == Some(i) {
            if run.a.is_some() && run.bg.is_some() {
                // A clickable keycap: lift and ring, colour kept.
                p.rect_stroke(cell.expand(1.5), 4.0, Stroke::new(1.5, TEXT.gamma_multiply(alpha)), StrokeKind::Outside);
                out.push((run.t.clone(), run.c));
            } else if run.a.is_some() {
                p.rect_filled(cell, 3.0, RUST.gamma_multiply(alpha));
                out.push((run.t.clone(), STAGE));
            } else {
                // Double-click targets: a lift, not an inversion.
                p.rect_filled(cell, 3.0, IRON_HI.gamma_multiply(alpha));
                p.rect_stroke(cell, 3.0, Stroke::new(1.0, EDGE.gamma_multiply(alpha)), StrokeKind::Inside);
                out.push((run.t.clone(), TEXT));
            }
        } else {
            out.push((run.t.clone(), run.c));
        }
        x += w;
    }
    g.line(p, pos, &out, alpha);
}

fn run_at<A>(g: &Grid, x0: f32, runs: &[R<A>], x: f32) -> Option<usize> {
    crate::glyph::run_at(runs, |r| r.t.chars().count(), g.col(x0, x))
}

/// Corner radius of every framed surface.
pub const ROUND: f32 = 6.0;

/// Width of the frame rule.
pub const RULE: f32 = 2.0;

/// The shared shadow: blurred, offset down, never a second edge.
pub fn shadow(alpha: f32) -> egui::epaint::Shadow {
    egui::epaint::Shadow { offset: [0, 6], blur: 20, spread: 0, color: Color32::from_black_alpha((150.0 * alpha) as u8) }
}

/// Frame of a window or menu. The rule is the window's edge: the body is
/// filled to exactly the rule's outer line and stops there.
pub fn frame(p: &egui::Painter, body: Rect, alpha: f32) {
    p.add(shadow(alpha).as_shape(body, ROUND));
    p.rect_filled(body, ROUND, IRON.gamma_multiply(alpha));
    p.rect_stroke(body, ROUND, Stroke::new(RULE, EDGE.gamma_multiply(alpha)), StrokeKind::Inside);
}

/// A divider inside a frame: one cell short of the rule on each side.
fn divider(p: &egui::Painter, g: &Grid, body: Rect, y: f32, alpha: f32) {
    let y = y.round() + 0.5;
    p.line_segment([Pos2::new(body.left() + g.cw, y), Pos2::new(body.right() - g.cw, y)], Stroke::new(1.0, EDGE.gamma_multiply(alpha)));
}

/// Window layout: half a row of padding, the title row, a divider, the
/// body rows, half a row of padding.
const HEAD: f32 = 2.0;

/// A window: bold rounded rule as its edge, ` ■ Title ` on the first row
/// (the square closes it), a divider, body rows. Rows with actions
/// highlight on hover; runs with actions invert on hover (buttons); runs
/// with double-click actions lift on hover.
#[allow(clippy::too_many_arguments)]
pub fn dialog<A: Clone>(
    ctx: &egui::Context,
    g: &Grid,
    id: &str,
    pos: Pos2,
    title: &str,
    rows: &[Row<A>],
    min_cols: usize,
    alpha: f32,
    close: A,
    mut extra: impl FnMut(&mut Ui, Pos2),
) -> Out<A> {
    let mut out = Out { click: None, at: None, double: None, hovered: false, rect: Rect::NOTHING };
    let lift = (1.0 - alpha) * g.rh * 0.6;
    Area::new(Id::new(id))
        .fixed_pos(pos + Vec2::new(0.0, lift))
        .order(Order::Foreground)
        .interactable(alpha > 0.05)
        .show(ctx, |ui| {
            let size = dialog_size(g, title, rows, min_cols);
            let (body, resp) = ui.allocate_exact_size(size, Sense::click());
            let p = ui.painter();
            frame(p, body, alpha);
            let hover = resp.hover_pos();

            // Title row.
            let head: Vec<R<A>> = vec![ra("■", MUTED, close.clone()), r(format!(" {title}"), TEXT)];
            let hp = Pos2::new(body.left() + g.cw * 1.5, body.top() + g.rh * 0.5);
            let hot_head = hover
                .filter(|h| h.y >= hp.y && h.y < hp.y + g.rh)
                .and_then(|h| run_at(g, hp.x, &head, h.x))
                .filter(|i| head[*i].a.is_some());
            paint_runs(g, p, hp, &head, alpha, hot_head);
            divider(p, g, body, body.top() + g.rh * 1.75, alpha);

            // Body.
            let o = Pos2::new(body.left() + g.cw * 2.0, body.top() + g.rh * HEAD);
            let hr = hover.map(|h| ((h.y - o.y) / g.rh).floor() as isize);
            let inner_l = body.left() + RULE + 3.0;
            let inner_r = body.right() - RULE - 3.0;
            for (i, row) in rows.iter().enumerate() {
                let y = o.y + i as f32 * g.rh;
                let hot_row = hr == Some(i as isize);
                let hot_run = if hot_row {
                    hover
                        .and_then(|h| run_at(g, o.x, &row.runs, h.x))
                        .filter(|j| row.runs[*j].a.is_some() || row.runs[*j].dbl.is_some())
                } else {
                    None
                };
                if hot_row && row.act.is_some() && hot_run.is_none() {
                    p.rect_filled(Rect::from_min_max(Pos2::new(inner_l, y), Pos2::new(inner_r, y + g.rh)), 3.0, IRON_HI.gamma_multiply(alpha));
                }
                paint_runs(g, p, Pos2::new(o.x, y), &row.runs, alpha, hot_run);
            }
            extra(ui, o);
            let at = resp.interact_pointer_pos();
            if let Some(c) = at
                && (resp.clicked() || resp.double_clicked())
            {
                if c.y < o.y {
                    if resp.clicked() {
                        out.click = run_at(g, hp.x, &head, c.x).and_then(|i| head[i].a.clone());
                    }
                } else {
                    let i = ((c.y - o.y) / g.rh).floor();
                    if i >= 0.0
                        && let Some(row) = rows.get(i as usize)
                    {
                        let j = run_at(g, o.x, &row.runs, c.x);
                        let run = j.map(|j| &row.runs[j]);
                        let cell = |j: usize| {
                            let x0: usize = row.runs[..j].iter().map(|x| x.t.chars().count()).sum();
                            let w = row.runs[j].t.chars().count();
                            Rect::from_min_size(Pos2::new(o.x + x0 as f32 * g.cw, o.y + i * g.rh), g.size(w, 1))
                        };
                        if resp.double_clicked() {
                            if let (Some(j), Some(a)) = (j, run.and_then(|x| x.dbl.clone())) {
                                out.double = Some((a, cell(j)));
                            }
                        } else if run.is_some_and(|x| x.dbl.is_some() && x.a.is_none()) {
                            // A double-click target: the first click does nothing.
                        } else if let (Some(j), Some(a)) = (j, run.and_then(|x| x.a.clone())) {
                            out.click = Some(a);
                            out.at = Some(cell(j));
                        } else {
                            out.click = row.act.clone();
                        }
                    }
                }
            }
            out.hovered = resp.contains_pointer();
            out.rect = body;
        });
    out
}

pub fn dialog_size<A>(g: &Grid, title: &str, rows: &[Row<A>], min: usize) -> Vec2 {
    let inner = rows.iter().map(Row::cols).max().unwrap_or(0).max(min).max(title.chars().count() + 6);
    Vec2::new((inner + 4) as f32 * g.cw, (rows.len() as f32 + HEAD + 0.5) * g.rh)
}

pub enum Item<A> {
    Act { label: String, key: String, act: A, on: bool },
    Rule,
}

/// A menu dropdown: the same rounded frame, shortcut right-aligned, hovered
/// row inverted, disabled rows dim. Dividers stop one cell short of the
/// frame on both sides so they read as dividers, not as more frame.
pub fn dropdown<A: Clone>(ctx: &egui::Context, g: &Grid, id: &str, pos: Pos2, items: &[Item<A>], alpha: f32) -> Out<A> {
    let mut out = Out { click: None, at: None, double: None, hovered: false, rect: Rect::NOTHING };
    let lw = items.iter().map(|i| if let Item::Act { label, .. } = i { label.chars().count() } else { 0 }).max().unwrap_or(0);
    let kw = items.iter().map(|i| if let Item::Act { key, .. } = i { key.chars().count() } else { 0 }).max().unwrap_or(0);
    let w = lw + kw + 8;
    let lift = (1.0 - alpha) * g.rh * 0.6;
    Area::new(Id::new(id))
        .fixed_pos(pos - Vec2::new(0.0, lift))
        .order(Order::Foreground)
        .interactable(alpha > 0.05)
        .show(ctx, |ui| {
            let (body, resp) = ui.allocate_exact_size(Vec2::new(w as f32 * g.cw, (items.len() as f32 + 1.0) * g.rh), Sense::click());
            let p = ui.painter();
            frame(p, body, alpha);
            let top = body.top() + g.rh * 0.5;
            let rule = body.shrink(RULE + 3.0);
            let hr = resp.hover_pos().map(|h| ((h.y - top) / g.rh).floor() as isize);
            for (i, it) in items.iter().enumerate() {
                let y = top + i as f32 * g.rh;
                match it {
                    Item::Rule => divider(p, g, body, y + g.rh * 0.5, alpha),
                    Item::Act { label, key, on, .. } => {
                        let hot = *on && hr == Some(i as isize);
                        if hot {
                            p.rect_filled(Rect::from_min_max(Pos2::new(rule.left(), y), Pos2::new(rule.right(), y + g.rh)), 3.0, RUST.gamma_multiply(alpha));
                        }
                        let (tc, kc) = match (hot, on) {
                            (true, _) => (STAGE, STAGE),
                            (false, true) => (TEXT, MUTED),
                            (false, false) => (DIM, DIM),
                        };
                        g.line(
                            p,
                            Pos2::new(body.left() + g.cw * 2.0, y),
                            &[(format!("{label:<lw$}"), tc), ("   ".into(), tc), (format!("{key:>kw$}"), kc)],
                            alpha,
                        );
                    }
                }
            }
            if resp.clicked()
                && let Some(i) = hr
                && i >= 0
                && let Some(Item::Act { act, on: true, .. }) = items.get(i as usize)
            {
                out.click = Some(act.clone());
            }
            out.hovered = resp.contains_pointer();
            out.rect = body;
        });
    out
}

/// A framed note on the stage (empty folder, undecodable file).
pub fn card(p: &egui::Painter, g: &Grid, center: Pos2, lines: &[(&str, Color32)]) -> Rect {
    let cols = lines.iter().map(|(s, _)| s.chars().count()).max().unwrap_or(0) + 4;
    let body = Rect::from_center_size(center, Vec2::new(cols as f32 * g.cw, (lines.len() as f32 + 1.0) * g.rh));
    frame(p, body, 1.0);
    for (i, (s, c)) in lines.iter().enumerate() {
        g.text(p, body.min + Vec2::new(g.cw * 2.0, (i as f32 + 0.5) * g.rh), s, *c, 1.0);
    }
    body
}

/// A text field cell: a recessed strip the caller puts a TextEdit on.
pub fn field_bg(p: &egui::Painter, r: Rect) {
    p.rect_filled(r.expand2(Vec2::new(3.0, 1.0)), 0.0, STAGE);
}
