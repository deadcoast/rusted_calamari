//! Containers for OTF content: one split dock and two native viewports.
//! Docking moves placement only; it never recreates a content controller.

use super::*;
use crate::{browserpane, dock::Geometry};

pub(super) struct DockDrag {
    widget: Widget,
    rect: Rect,
    press: Pos2,
}
pub(super) struct FloatDrag {
    pub(super) grab: Vec2,
    pub(super) start: Pos2,
    pub(super) last: Pos2,
    pub(super) changed: f64,
}

fn primary_down(ctx: &egui::Context) -> bool {
    #[cfg(windows)]
    {
        let _ = ctx;
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(1) < 0 }
    }
    #[cfg(not(windows))]
    {
        ctx.input(|i| i.pointer.primary_down())
    }
}

impl App {
    fn fallback_geometry(&self, w: Widget) -> Geometry {
        let main = self.main_rect.unwrap_or(Rect::from_min_size(
            Pos2::new(100.0, 100.0),
            Vec2::new(1000.0, 800.0),
        ));
        let size = Vec2::new(420.0, (main.height() * 0.6).clamp(300.0, 650.0));
        let pos = main.min
            + Vec2::new(
                (main.width() - size.x - 30.0).max(30.0),
                50.0 + w.index() as f32 * 50.0,
            );
        Geometry::from_rect(Rect::from_min_size(pos, size))
    }

    pub(super) fn toggle_widget(&mut self, w: Widget) {
        self.fold_clicks.reset();
        self.fold_action = None;
        self.dock_drag = None;
        self.float_drags[w.index()] = None;
        if self.s.dock.floating(w) {
            self.s.dock.dock(w);
            self.s.tree = true;
        } else {
            self.s.dock.detach(w, self.fallback_geometry(w));
        }
        self.egui.request_repaint();
    }

    /// The title band reserves its space so appearing controls never move
    /// tree rows. Shift-click anywhere on this hover menu toggles placement.
    pub(super) fn widget_header(
        &mut self,
        ui: &mut egui::Ui,
        w: Widget,
        rect: Rect,
        g: &Grid,
        floating: bool,
        now: f64,
    ) -> Rect {
        let ctx = ui.ctx().clone();
        let header = Rect::from_min_size(rect.min, Vec2::new(rect.width(), g.rh * 1.35));
        let button_rect = Rect::from_min_max(
            Pos2::new(
                (header.right() - g.cw * 4.0).max(header.left()),
                header.top(),
            ),
            header.max,
        );
        let drag_rect =
            Rect::from_min_max(header.min, Pos2::new(button_rect.left(), header.bottom()));
        let id = Id::new("otf-widget-header").with(w.index());
        let resp = ui.interact(drag_rect, id, Sense::CLICK | Sense::DRAG);
        let near = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|p| header.expand(g.rh * 0.5).contains(p))
        });
        let alpha = ctx.animate_bool_with_time(
            id.with("fade"),
            self.s.pin_ui || near || resp.dragged(),
            0.12,
        );
        if alpha > 0.01 {
            let p = ui.painter().with_clip_rect(header);
            p.rect_filled(header, 0.0, IRON.gamma_multiply(alpha));
            g.text(
                &p,
                header.min + Vec2::new(g.cw, g.rh * 0.15),
                w.title(),
                MUTED,
                alpha,
            );
            let glyph = if floating { "[<]" } else { "[>]" };
            let label = if floating {
                "Dock this tree (Shift+click header)"
            } else {
                "Detach this tree (Shift+click header)"
            };
            let toggle =
                browserpane::button(ui, g, button_rect, id.with("toggle"), glyph, label, true);
            if toggle.clicked() {
                self.toggle_widget(w);
            }
        }
        if resp.hovered() {
            ctx.set_cursor_icon(CursorIcon::Grab);
        }
        let shift = ctx.input(|i| i.modifiers.shift);
        if resp.clicked() && shift {
            self.toggle_widget(w);
        }
        if !shift && floating && resp.hovered() && ctx.input(|i| i.pointer.primary_pressed()) {
            if let Some(native) = ctx.input(|i| i.viewport().inner_rect) {
                let grab = resp.interact_pointer_pos().unwrap_or(header.center()) - rect.min;
                self.float_drags[w.index()] = Some(FloatDrag {
                    grab,
                    start: native.min,
                    last: native.min,
                    changed: now,
                });
            }
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        if !floating && !shift {
            if resp.drag_started() {
                let press = ctx
                    .input(|i| i.pointer.press_origin())
                    .unwrap_or(header.center());
                self.dock_drag = Some(DockDrag {
                    widget: w,
                    rect,
                    press,
                });
            }
            if resp.dragged()
                && let Some(drag) = &self.dock_drag
                && drag.widget == w
            {
                ctx.set_cursor_icon(CursorIcon::Grabbing);
                if let Some(q) = ctx.input(|i| i.pointer.interact_pos()) {
                    let ghost = drag.rect.translate(q - drag.press);
                    ui.painter().rect_stroke(
                        ghost,
                        0.0,
                        Stroke::new(1.0, rgb(octo::paint::STEEL)),
                        StrokeKind::Inside,
                    );
                }
            }
            if resp.drag_stopped()
                && let Some(drag) = self.dock_drag.take()
            {
                if let Some(q) = ctx.input(|i| i.pointer.interact_pos()) {
                    let origin = self.main_rect.map_or(Vec2::ZERO, |r| r.min.to_vec2());
                    let point = q + origin;
                    if !self.snap_zone.is_some_and(|zone| zone.contains(point)) {
                        let mut native = drag.rect.translate(q - drag.press + origin);
                        native.max = native.min + native.size().max(Vec2::new(320.0, 240.0));
                        self.s.dock.widgets[w.index()].geometry = Some(Geometry::from_rect(native));
                        self.s.dock.detach(w, Geometry::from_rect(native));
                        self.fold_clicks.reset();
                    }
                }
            }
        }
        Rect::from_min_max(header.left_bottom(), rect.max)
    }

    fn widget_content(&mut self, ui: &mut egui::Ui, w: Widget, rect: Rect, g: &Grid, now: f64) {
        if !rect.is_positive() {
            return;
        }
        match w {
            Widget::Favourites => {
                if ui.input(|i| {
                    i.pointer.primary_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
                }) {
                    self.files.focused = false;
                }
                if self.lib.is_some() {
                    self.tree_pane(ui, rect, rect.left(), g, now);
                } else {
                    g.text(
                        ui.painter(),
                        rect.min + Vec2::new(g.cw, g.rh),
                        "Open a library (Ctrl+O)",
                        MUTED,
                        1.0,
                    );
                }
            }
            Widget::Files => {
                let library = self.lib.as_ref().map(|l| l.tree.root.clone());
                if let Some(action) =
                    self.files
                        .show(ui, rect, g, self.style(), library.as_deref(), now)
                {
                    self.file_action(action, now);
                }
            }
        }
    }

    pub(super) fn docked_trees(&mut self, ui: &mut egui::Ui, panel: Rect, g: &Grid, now: f64) {
        let ctx = ui.ctx().clone();
        ui.painter().rect_filled(panel, 0.0, PANEL);
        let pad = if self.s.pin_ui {
            g.rh * 0.25
        } else {
            g.rh * 1.75
        };
        let bounds = Rect::from_min_max(
            panel.min + Vec2::new(0.0, pad),
            panel.max - Vec2::new(0.0, pad.min(panel.height() * 0.2)),
        );
        if !bounds.is_positive() {
            return;
        }
        let regions = self
            .s
            .dock
            .regions(bounds, (g.rh * 0.45).max(7.0), g.rh * 5.0);
        if let [Some(top), Some(bottom)] = regions {
            let y = (top.bottom() + bottom.top()) * 0.5;
            let split = Rect::from_center_size(
                Pos2::new(bounds.center().x, y),
                Vec2::new(bounds.width(), g.rh * 0.45),
            );
            let response = ui.interact(split, Id::new("tree-widget-split"), Sense::DRAG);
            if response.hovered() || response.dragged() {
                ctx.set_cursor_icon(CursorIcon::ResizeVertical);
            }
            if response.dragged()
                && let Some(p) = response.interact_pointer_pos()
            {
                self.s.dock.split = ((p.y - bounds.top()) / bounds.height()).clamp(0.15, 0.85);
            }
            ui.painter().line_segment(
                [
                    Pos2::new(bounds.left() + g.cw, y),
                    Pos2::new(bounds.right() - g.cw, y),
                ],
                Stroke::new(1.0, EDGE.gamma_multiply(0.45)),
            );
        }
        for w in Widget::ALL {
            if let Some(rect) = regions[w.index()] {
                let content = self.widget_header(ui, w, rect, g, false, now);
                self.widget_content(ui, w, content, g, now);
            }
        }
    }

    pub(super) fn floating_trees(&mut self, ctx: &egui::Context, now: f64) {
        for w in Widget::ALL {
            if !self.s.dock.floating(w) {
                continue;
            }
            let geometry = self.s.dock.widgets[w.index()]
                .geometry
                .unwrap_or_else(|| self.fallback_geometry(w));
            self.s.dock.widgets[w.index()]
                .geometry
                .get_or_insert(geometry);
            let id = egui::ViewportId::from_hash_of(("otf-native", w.index()));
            let builder = egui::ViewportBuilder::default()
                .with_title(format!("{} — Rusted Calamari", w.title()))
                .with_position(Pos2::from(geometry.position))
                .with_inner_size(Vec2::from(geometry.size))
                .with_min_inner_size([320.0, 240.0])
                .with_decorations(false);
            ctx.show_viewport_immediate(id, builder, |ui, class| {
                let child = ui.ctx().clone();
                #[cfg(feature = "workspace-smoke")]
                {
                    self.smoke_capture(&child);
                    self.smoke_native(w, &child, class);
                }
                if child.input(|i| i.viewport().close_requested()) {
                    self.toggle_widget(w);
                    return;
                }
                let native = child.input(|i| i.viewport().inner_rect);
                // ViewportBuilder positions the outer window, while sizes
                // describe its client area. Feeding the client origin back
                // into the builder accumulates native frame offsets.
                let outer = child.input(|i| i.viewport().outer_rect);
                let full = ui.max_rect();
                ui.painter().rect_filled(full, 0.0, PANEL);
                let g = Grid::new(&child, SIZES[self.s.size % SIZES.len()]);
                if w == Widget::Files {
                    self.files.keys(&child);
                }
                self.widget_shortcuts(&child, now);
                let content = self.widget_header(ui, w, full.shrink(5.0), &g, true, now);
                self.widget_content(ui, w, content, &g, now);
                if class != egui::ViewportClass::EmbeddedWindow {
                    window::edges(&child, full);
                    window::hairline(&child, full);
                }
                if let Some(native) = native {
                    let settled = Geometry::from_rect(Rect::from_min_size(
                        outer.unwrap_or(native).min,
                        native.size(),
                    ));
                    let mut snap = false;
                    let mut finish = false;
                    if let Some(drag) = &mut self.float_drags[w.index()] {
                        if drag.last != native.min {
                            drag.last = native.min;
                            drag.changed = now;
                        }
                        if !primary_down(&child) && now - drag.changed > 0.12 {
                            snap = drag.start.distance(native.min) > 6.0
                                && self
                                    .snap_zone
                                    .is_some_and(|zone| zone.contains(native.min + drag.grab));
                            finish = true;
                        }
                        child.request_repaint_after(std::time::Duration::from_millis(16));
                    } else if self.s.dock.floating(w) && settled.valid() {
                        self.s.dock.widgets[w.index()].geometry = Some(settled);
                    }
                    if finish {
                        self.float_drags[w.index()] = None;
                        if snap {
                            self.s.dock.dock(w);
                            self.s.tree = true;
                            self.fold_clicks.reset();
                        } else if settled.valid() {
                            self.s.dock.widgets[w.index()].geometry = Some(settled);
                        }
                        ctx.request_repaint();
                    }
                }
            });
        }
    }

    pub(super) fn dock_preview(&self, ctx: &egui::Context) {
        let Some(zone) = self.snap_zone else { return };
        let Some(main) = self.main_rect else { return };
        let over = self
            .float_drags
            .iter()
            .flatten()
            .any(|d| zone.contains(d.last + d.grab) && d.start.distance(d.last) > 6.0);
        if over {
            let p = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("dock-preview")));
            p.rect_stroke(
                zone.translate(-main.min.to_vec2()).shrink(3.0),
                0.0,
                Stroke::new(2.0, rgb(octo::paint::STEEL)),
                StrokeKind::Inside,
            );
        }
    }

    fn widget_shortcuts(&mut self, ctx: &egui::Context, now: f64) {
        if ctx.egui_wants_keyboard_input() || !matches!(self.dialog, Dialog::None) {
            return;
        }
        for event in ctx.input(|i| i.events.clone()) {
            let Event::Key {
                key,
                pressed: true,
                repeat: false,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if key == Key::Tab {
                let root = self.egui.clone();
                self.act(
                    &root,
                    if modifiers.shift {
                        Act::Image
                    } else {
                        Act::Tree
                    },
                    now,
                );
            } else if !modifiers.command && !modifiers.alt {
                if let Some(name) = Self::key_name(key)
                    && let Some(cat) = self.lib.as_ref().and_then(|l| {
                        l.tree
                            .cats
                            .iter()
                            .position(|c| c.key.as_deref() == Some(&name))
                    })
                {
                    self.classify(cat, now);
                }
            }
        }
    }

    pub(super) fn file_action(&mut self, action: browserpane::Action, now: f64) {
        match action {
            browserpane::Action::OpenFolder(path) => self.request_library(path, None, now),
            browserpane::Action::Preview(path) => {
                if self.current().as_ref() == Some(&path) {
                    return;
                }
                let Some(parent) = path.parent() else { return };
                if let Some(lib) = &mut self.lib
                    && let Some(id) =
                        (0..lib.tree.nodes.len()).find(|&id| lib.tree.dir(id) == parent)
                    && let Some(cursor) = lib.tree.nodes[id].files.iter().position(|f| {
                        Some(f.name.as_str()) == path.file_name().and_then(|n| n.to_str())
                    })
                {
                    lib.active = id;
                    lib.cursor = cursor;
                    self.crop = None;
                    self.quick = None;
                    self.reset_view();
                } else {
                    self.request_library(parent.to_path_buf(), Some(path), now);
                }
            }
        }
    }

    pub(super) fn request_library(&mut self, path: PathBuf, image: Option<PathBuf>, now: f64) {
        self.settle(true);
        if !self.pending.is_empty() {
            self.next_library = Some((path, image));
            self.dialog = Dialog::Unsaved;
        } else {
            self.open_requested_library(path, image, now);
        }
    }

    fn open_requested_library(&mut self, path: PathBuf, image: Option<PathBuf>, now: f64) {
        if self.open_library(&path, now)
            && let Some(image) = image
            && let Some(lib) = &mut self.lib
        {
            lib.cursor = lib.tree.nodes[0]
                .files
                .iter()
                .position(|f| Some(f.name.as_str()) == image.file_name().and_then(|n| n.to_str()))
                .unwrap_or(0);
        }
        self.dialog = Dialog::None;
    }

    pub(super) fn finish_unsaved(&mut self, ctx: &egui::Context, now: f64) {
        if let Some((path, image)) = self.next_library.take() {
            self.open_requested_library(path, image, now);
        } else {
            self.quitting = true;
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }
}
