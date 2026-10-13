//! The tool widget: a small horizontal strip of icon buttons that sits at
//! the top right of the stage, under the menu bar's band, so the two never
//! share a pixel.
//!
//! Tools are data. The app hands in a list of `Tool`s each frame (icon,
//! name, shortcut, group, whether it is engaged or available); the strip
//! sizes itself from that list, puts a rule between groups, right-aligns to
//! its anchor, and wraps onto another row before it would run past the room
//! it is given. Adding a tool is one more entry; nothing else moves for it.
//!
//! Every button animates the same way: hover lights the icon and lifts it a
//! point and a half; press sinks and shrinks it; a click sends a ring out
//! from the button; an engaged tool fills with its own colour and switches
//! to the solid icon cut. A name and shortcut appear under a button after a
//! short hover.

use crate::fonts::icon;
use crate::glyph::{Grid, Run};
use crate::theme::{self, DIM, EDGE, IRON, IRON_HI, MUTED, RUST_HI, STAGE, TEXT};
use eframe::egui::{self, Align2, Area, Color32, CursorIcon, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};

pub struct Tool<A> {
    /// Stable name; animation state is keyed on it.
    pub id: &'static str,
    /// Phosphor codepoint, drawn in the bold cut (fill when engaged).
    pub icon: &'static str,
    /// A small solid glyph on the icon's lower right.
    pub badge: Option<&'static str>,
    pub name: &'static str,
    pub keys: &'static str,
    /// A second tooltip line: what the tool does.
    pub note: &'static str,
    /// Tools with the same group sit together; a rule separates groups.
    pub group: u8,
    pub act: A,
    /// The tool's mode is on.
    pub on: bool,
    /// The tool can be used now (an engaged tool can always be turned off).
    pub enabled: bool,
    /// Colour of the engaged state and of the click ring.
    pub tone: Color32,
}

pub struct Out<A> {
    pub click: Option<A>,
    pub hovered: bool,
    /// The strip as drawn; `Rect::NOTHING` when hidden.
    pub rect: Rect,
}

/// The strip's measurements, from the text grid so it scales with the
/// interface size.
pub struct Metrics {
    pub btn: f32,
    pub gap: f32,
    pub sep: f32,
    pub pad: f32,
}

impl Metrics {
    pub fn new(g: &Grid) -> Metrics {
        Metrics { btn: (g.rh * 1.7).round(), gap: 2.0, sep: 9.0, pad: 4.0 }
    }
}

/// Lay buttons out left to right with a rule between groups, wrapping onto
/// a new row before `max_w`. Returns button rects and rules (x, row), both
/// relative to the strip's top-left, and the strip's size.
pub fn layout(groups: &[u8], m: &Metrics, max_w: f32) -> (Vec<Rect>, Vec<(f32, usize)>, Vec2) {
    let mut cells = Vec::with_capacity(groups.len());
    let mut rules = Vec::new();
    let (mut x, mut row, mut w) = (m.pad, 0usize, 0.0f32);
    for (i, &gr) in groups.iter().enumerate() {
        if i > 0 {
            let rule = gr != groups[i - 1];
            let lead = if rule { m.sep } else { m.gap };
            if x + lead + m.btn + m.pad > max_w {
                // A new row starts flush; a group break there needs no rule.
                row += 1;
                x = m.pad;
            } else {
                if rule {
                    rules.push((x + lead * 0.5, row));
                }
                x += lead;
            }
        }
        cells.push(Rect::from_min_size(Pos2::new(x, m.pad + row as f32 * (m.btn + m.gap)), Vec2::splat(m.btn)));
        x += m.btn;
        w = w.max(x + m.pad);
    }
    let rows = row + 1;
    let h = m.pad * 2.0 + rows as f32 * m.btn + (rows - 1) as f32 * m.gap;
    (cells, rules, Vec2::new(w, h))
}

/// Where the strip will sit, without drawing it: its hover zone exists
/// before it has ever been shown.
pub fn bounds<A>(g: &Grid, top_right: Pos2, max_w: f32, tools: &[Tool<A>]) -> Rect {
    let groups: Vec<u8> = tools.iter().map(|t| t.group).collect();
    let (_, _, size) = layout(&groups, &Metrics::new(g), max_w);
    Rect::from_min_size(Pos2::new(top_right.x - size.x, top_right.y), size)
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

/// Draw the strip with its right edge at `top_right`. `alpha` is the
/// shared interface fade; the strip slides in a few points as it fades in.
/// A strip that isn't `live` takes no input.
#[allow(clippy::too_many_arguments)]
pub fn show<A: Clone>(ctx: &egui::Context, g: &Grid, top_right: Pos2, max_w: f32, tools: &[Tool<A>], alpha: f32, live: bool, now: f64) -> Out<A> {
    let mut out = Out { click: None, hovered: false, rect: Rect::NOTHING };
    if tools.is_empty() || alpha < 0.01 {
        ctx.data_mut(|d| d.remove::<(Id, f64)>(Id::new("tool-tip-since")));
        return out;
    }
    let m = Metrics::new(g);
    let groups: Vec<u8> = tools.iter().map(|t| t.group).collect();
    let (cells, rules, size) = layout(&groups, &m, max_w);
    let slide = (1.0 - alpha) * 10.0;
    let pos = Pos2::new(top_right.x - size.x + slide, top_right.y);
    let mut tip: Option<(usize, Rect)> = None;
    Area::new(Id::new("toolbar"))
        .fixed_pos(pos)
        .order(Order::Foreground)
        .interactable(live && alpha > 0.05)
        .show(ctx, |ui| {
            // The body takes drags too, so a drag that starts on the strip
            // never reaches the image underneath.
            let (body, bresp) = ui.allocate_exact_size(size, Sense::CLICK | Sense::DRAG);
            let p = ui.painter();
            theme::frame(p, body, alpha);
            for (x, row) in &rules {
                let y0 = body.top() + m.pad + *row as f32 * (m.btn + m.gap);
                let x = (body.left() + x).round() + 0.5;
                p.line_segment([Pos2::new(x, y0 + 7.0), Pos2::new(x, y0 + m.btn - 7.0)], Stroke::new(1.0, EDGE.gamma_multiply(alpha * 0.8)));
            }
            for (i, (t, c)) in tools.iter().zip(&cells).enumerate() {
                let r = c.translate(body.min.to_vec2());
                let resp = ui.interact(r, Id::new("tool").with(t.id), Sense::CLICK);
                if button(ui, t, r, &resp, alpha, now) {
                    out.click = Some(t.act.clone());
                }
                if resp.hovered() {
                    tip = Some((i, r));
                }
            }
            out.hovered = bresp.contains_pointer();
            out.rect = body;
        });
    tooltip(ctx, g, tools, tip, out.rect.bottom(), alpha, now);
    out
}

/// One button. Returns true on a click that should act.
fn button<A>(ui: &egui::Ui, t: &Tool<A>, r: Rect, resp: &egui::Response, alpha: f32, now: f64) -> bool {
    let ctx = ui.ctx();
    let id = resp.id;
    let live = t.enabled || t.on;
    let hot = live && resp.hovered();
    let down = live && resp.is_pointer_button_down_on();
    let h = ctx.animate_bool_with_time(id.with("hover"), hot, 0.12);
    let d = ctx.animate_bool_with_time(id.with("press"), down, 0.05);
    let on = ctx.animate_bool_with_time(id.with("on"), t.on, 0.16);
    let clicked = live && resp.clicked();
    if clicked {
        ctx.data_mut(|m| m.insert_temp(id.with("ring"), now));
    }
    let p = ui.painter();

    // Fill: frame body, lit on hover, the tool's colour when engaged.
    let bg = mix(mix(IRON, IRON_HI, h), t.tone, on);
    let face = r.shrink(d);
    if h > 0.0 || on > 0.0 {
        p.rect_filled(face, 5.0, bg.gamma_multiply(alpha));
    }
    if h > 0.0 && on < 1.0 {
        p.rect_stroke(face, 5.0, Stroke::new(1.0, EDGE.gamma_multiply(alpha * h * (1.0 - on))), StrokeKind::Inside);
    }

    // Icon: lifts on hover, sinks and shrinks on press.
    let col = if live { mix(mix(TEXT, RUST_HI, h), STAGE, on) } else { DIM };
    let size = r.height() * 0.6 * (1.0 - 0.12 * d);
    let c = r.center() + Vec2::new(0.0, -1.5 * h * (1.0 - d) + d);
    p.text(c, Align2::CENTER_CENTER, t.icon, icon(size, false), col.gamma_multiply(alpha));
    if let Some(b) = t.badge {
        let bpos = c + Vec2::new(size * 0.4, size * 0.36);
        // Knock the icon out behind the badge so it reads at any size.
        p.circle_filled(bpos, size * 0.3, bg.gamma_multiply(alpha));
        let bc = if !live { DIM } else if on > 0.5 { STAGE } else { t.tone };
        p.text(bpos, Align2::CENTER_CENTER, b, icon(size * 0.48, true), bc.gamma_multiply(alpha));
    }

    // Click ring: out from the button and gone in under half a second.
    if let Some(t0) = ctx.data(|m| m.get_temp::<f64>(id.with("ring"))) {
        let age = ((now - t0) / 0.38) as f32;
        if age < 1.0 {
            let e = 1.0 - (1.0 - age).powi(3);
            let ring = mix(t.tone, Color32::WHITE, 0.25);
            p.rect_stroke(r.expand(1.0 + 7.0 * e), 5.0 + 4.0 * e, Stroke::new(2.0 * (1.0 - age), ring.gamma_multiply(alpha * (1.0 - age))), StrokeKind::Outside);
            ctx.request_repaint();
        }
    }
    if hot {
        ctx.set_cursor_icon(CursorIcon::PointingHand);
    }
    clicked
}

/// Name, shortcut and note under the hovered button, after a short hover.
fn tooltip<A>(ctx: &egui::Context, g: &Grid, tools: &[Tool<A>], tip: Option<(usize, Rect)>, below: f32, alpha: f32, now: f64) {
    let key = Id::new("tool-tip-since");
    let Some((i, r)) = tip else {
        ctx.data_mut(|d| d.remove::<(Id, f64)>(key));
        ctx.animate_bool_with_time(Id::new("tool-tip"), false, 0.08);
        return;
    };
    let t = &tools[i];
    let tid = Id::new("tool").with(t.id);
    let since = ctx.data_mut(|d| {
        let e = d.get_temp_mut_or_insert_with(key, || (tid, now));
        if e.0 != tid {
            *e = (tid, now);
        }
        e.1
    });
    let ready = now - since > 0.45;
    if !ready {
        ctx.request_repaint_after(std::time::Duration::from_millis(60));
    }
    let a = ctx.animate_bool_with_time(Id::new("tool-tip"), ready, 0.1) * alpha;
    if a < 0.01 {
        return;
    }
    let mut lines: Vec<Vec<Run>> = vec![vec![(t.name.to_string(), TEXT), ("  ".into(), DIM), (t.keys.to_string(), MUTED)]];
    if !t.note.is_empty() {
        lines.push(vec![(t.note.to_string(), MUTED)]);
    }
    if !t.enabled && !t.on {
        lines.push(vec![("Open an image to use it".to_string(), DIM)]);
    }
    let cols = lines.iter().map(|l| Grid::cols(l)).max().unwrap_or(0) + 4;
    let size = Vec2::new(cols as f32 * g.cw, (lines.len() as f32 + 1.0) * g.rh);
    let screen = ctx.content_rect();
    // Right-aligned to the button, kept on screen; a tip wider than the
    // screen starts at its left edge.
    let x = (r.right() - size.x).min(screen.right() - size.x - 4.0).max(screen.left() + 4.0);
    let pos = Pos2::new(x, below + 6.0 + (1.0 - a) * 4.0);
    Area::new(Id::new("tool-tip-area")).fixed_pos(pos).order(Order::Tooltip).interactable(false).show(ctx, |ui| {
        let (body, _) = ui.allocate_exact_size(size, Sense::hover());
        let p = ui.painter();
        theme::frame(p, body, a);
        for (k, l) in lines.iter().enumerate() {
            g.line(p, body.min + Vec2::new(g.cw * 2.0, (k as f32 + 0.5) * g.rh), l, a);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m() -> Metrics {
        Metrics { btn: 32.0, gap: 2.0, sep: 9.0, pad: 4.0 }
    }

    #[test]
    fn one_row_with_a_rule_between_groups() {
        let (cells, rules, size) = layout(&[0, 0, 1], &m(), 1000.0);
        assert_eq!(cells.len(), 3);
        assert_eq!(rules.len(), 1, "one group break");
        assert!(rules[0].0 > cells[1].right() && rules[0].0 < cells[2].left());
        assert_eq!(size.y, 4.0 * 2.0 + 32.0);
        assert_eq!(size.x, cells[2].right() + 4.0);
    }

    #[test]
    fn wraps_before_the_room_runs_out_and_never_overlaps() {
        let groups = [0, 0, 1, 1, 1, 2, 2];
        let room = 4.0 + 3.0 * 32.0 + 2.0 * 9.0 + 4.0;
        let (cells, rules, size) = layout(&groups, &m(), room);
        assert!(size.x <= room, "{size:?}");
        assert!(cells.iter().map(|c| c.top()).fold(0.0f32, f32::max) > 4.0, "wrapped onto a second row");
        for (i, a) in cells.iter().enumerate() {
            assert!(a.right() <= size.x - 4.0 + 0.01 && a.bottom() <= size.y - 4.0 + 0.01);
            for b in &cells[i + 1..] {
                assert!(!a.intersects(b.shrink(0.5)), "{a:?} {b:?}");
            }
        }
        // No rule sits at the start of a row.
        for (x, row) in rules {
            assert!(cells.iter().any(|c| (c.top() - (4.0 + row as f32 * 34.0)).abs() < 0.1 && c.right() < x));
        }
    }
}
