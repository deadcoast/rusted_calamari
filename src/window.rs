//! The window without the system's frame. A title bar's jobs move into the
//! interface: the menu bar drags the window and double-clicks to maximize,
//! three buttons at its right end minimize, maximize or restore, and close,
//! and a thin band along every edge and corner resizes. While the window
//! floats, a hairline marks its edge against the desktop. Maximized or full
//! screen, the bands and the hairline step aside.

use crate::fonts::icon;
use crate::theme::{EDGE, IRON_HI, TEXT, WARN};
use eframe::egui::{self, Align2, Area, Color32, CursorIcon, Id, LayerId, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2, ViewportCommand};
use eframe::egui::viewport::ResizeDirection;
use egui_phosphor::bold as ph;

/// Thickness of the resize bands and size of the corner squares.
const BAND: f32 = 4.0;
const CORNER: f32 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ctl {
    Minimize,
    Maximize,
    Close,
}

/// (maximized, full screen) as the platform reports them.
pub fn state(ctx: &egui::Context) -> (bool, bool) {
    ctx.input(|i| (i.viewport().maximized.unwrap_or(false), i.viewport().fullscreen.unwrap_or(false)))
}

/// Resize bands on every edge and corner, above everything else on screen.
pub fn edges(ctx: &egui::Context, full: Rect) {
    let (maxed, fs) = state(ctx);
    if maxed || fs {
        return;
    }
    let (l, t, r, b) = (full.left(), full.top(), full.right(), full.bottom());
    let zones = [
        (Rect::from_min_max(Pos2::new(l + CORNER, t), Pos2::new(r - CORNER, t + BAND)), ResizeDirection::North, CursorIcon::ResizeNorth),
        (Rect::from_min_max(Pos2::new(l + CORNER, b - BAND), Pos2::new(r - CORNER, b)), ResizeDirection::South, CursorIcon::ResizeSouth),
        (Rect::from_min_max(Pos2::new(l, t + CORNER), Pos2::new(l + BAND, b - CORNER)), ResizeDirection::West, CursorIcon::ResizeWest),
        (Rect::from_min_max(Pos2::new(r - BAND, t + CORNER), Pos2::new(r, b - CORNER)), ResizeDirection::East, CursorIcon::ResizeEast),
        (Rect::from_min_size(Pos2::new(l, t), Vec2::splat(CORNER)), ResizeDirection::NorthWest, CursorIcon::ResizeNorthWest),
        (Rect::from_min_size(Pos2::new(r - CORNER, t), Vec2::splat(CORNER)), ResizeDirection::NorthEast, CursorIcon::ResizeNorthEast),
        (Rect::from_min_size(Pos2::new(l, b - CORNER), Vec2::splat(CORNER)), ResizeDirection::SouthWest, CursorIcon::ResizeSouthWest),
        (Rect::from_min_size(Pos2::new(r - CORNER, b - CORNER), Vec2::splat(CORNER)), ResizeDirection::SouthEast, CursorIcon::ResizeSouthEast),
    ];
    for (i, (zone, dir, cursor)) in zones.into_iter().enumerate() {
        Area::new(Id::new("resize").with(i)).fixed_pos(zone.min).order(Order::Tooltip).show(ctx, |ui| {
            let (_, resp) = ui.allocate_exact_size(zone.size(), Sense::drag());
            if resp.hovered() || resp.dragged() {
                ctx.set_cursor_icon(cursor);
            }
            if resp.hovered() && ctx.input(|i| i.pointer.primary_pressed()) {
                ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
            }
        });
    }
}

/// A 1px rule on the window's edge while it floats.
pub fn hairline(ctx: &egui::Context, full: Rect) {
    let (maxed, fs) = state(ctx);
    if maxed || fs {
        return;
    }
    let p = ctx.layer_painter(LayerId::new(Order::Tooltip, Id::new("hairline")));
    p.rect_stroke(full, 0.0, Stroke::new(1.0, EDGE.gamma_multiply(0.55)), StrokeKind::Inside);
}

/// The three window buttons, right-aligned in `bar`, each `w` wide.
/// Returns the one clicked and the span they cover.
pub fn controls(ui: &egui::Ui, bar: Rect, w: f32, alpha: f32) -> (Option<Ctl>, Rect) {
    let ctx = ui.ctx().clone();
    let (maxed, fs) = state(&ctx);
    let items = [
        (Ctl::Minimize, ph::MINUS),
        (Ctl::Maximize, if maxed || fs { ph::COPY } else { ph::SQUARE }),
        (Ctl::Close, ph::X),
    ];
    let span = Rect::from_min_max(Pos2::new(bar.right() - w * items.len() as f32, bar.top()), bar.max);
    let mut hit = None;
    for (i, (ctl, glyph)) in items.into_iter().enumerate() {
        let r = Rect::from_min_size(Pos2::new(span.left() + w * i as f32, bar.top()), Vec2::new(w, bar.height()));
        // Click and drag: a press that wanders must not start a window drag.
        let resp = ui.interact(r, Id::new("winctl").with(i), Sense::click_and_drag());
        let h = ctx.animate_bool_with_time(resp.id.with("h"), resp.hovered(), 0.1);
        let d = ctx.animate_bool_with_time(resp.id.with("d"), resp.is_pointer_button_down_on(), 0.05);
        let p = ui.painter();
        let (fill, lit) = if ctl == Ctl::Close { (WARN, Color32::WHITE) } else { (IRON_HI, TEXT) };
        if h > 0.0 {
            p.rect_filled(r, 0.0, fill.gamma_multiply(alpha * h * (1.0 - 0.25 * d)));
        }
        let col = if h > 0.5 { lit } else { TEXT.gamma_multiply(0.8) };
        let size = bar.height() * 0.42 * (1.0 - 0.1 * d);
        p.text(r.center() + Vec2::new(0.0, d), Align2::CENTER_CENTER, glyph, icon(size, false), col.gamma_multiply(alpha));
        if resp.clicked() {
            hit = Some(ctl);
        }
    }
    (hit, span)
}

/// Act on a window button or its keyboard twin.
pub fn apply(ctx: &egui::Context, c: Ctl) {
    let (maxed, fs) = state(ctx);
    match c {
        Ctl::Minimize => ctx.send_viewport_cmd(ViewportCommand::Minimized(true)),
        Ctl::Maximize if fs => ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false)),
        Ctl::Maximize => ctx.send_viewport_cmd(ViewportCommand::Maximized(!maxed)),
        Ctl::Close => ctx.send_viewport_cmd(ViewportCommand::Close),
    }
}

pub fn toggle_fullscreen(ctx: &egui::Context) {
    let (_, fs) = state(ctx);
    ctx.send_viewport_cmd(ViewportCommand::Fullscreen(!fs));
}
