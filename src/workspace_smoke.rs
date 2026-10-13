//! Actual eframe windows and GPU screenshots driven through application
//! actions. Physical OS pointer/DPI behaviour is deliberately not claimed.
use super::*;
use crate::dock::Geometry;
use std::collections::HashSet;

pub(super) struct Smoke {
    out: PathBuf,
    phase: usize,
    at: f64,
    pending: Vec<String>,
    captured: HashSet<String>,
    native_seen: HashSet<usize>,
    saved: Option<[Option<Geometry>; 2]>,
    report: Vec<String>,
}

impl App {
    #[allow(dead_code)] // Used by the separate, feature-gated smoke runner.
    pub fn enable_smoke(&mut self, out: PathBuf) {
        self.s.pin_ui = true;
        self.s.dock = Dock::default();
        self.s.tree = true;
        self.s.image = true;
        self.s.panel_w = Some(450.0);
        self.smoke = Some(Smoke {
            out,
            phase: 0,
            at: 0.0,
            pending: Vec::new(),
            captured: HashSet::new(),
            native_seen: HashSet::new(),
            saved: None,
            report: Vec::new(),
        });
    }

    pub(super) fn smoke_native(
        &mut self,
        widget: Widget,
        ctx: &egui::Context,
        class: egui::ViewportClass,
    ) {
        if let Some(smoke) = &mut self.smoke
            && class == egui::ViewportClass::Immediate
            && ctx.input(|i| i.viewport().inner_rect.is_some())
        {
            if smoke.native_seen.insert(widget.index()) {
                smoke.report.push(format!(
                    "{}: {:?}",
                    widget.title(),
                    ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect))
                ));
            }
        }
    }

    pub(super) fn smoke_capture(&mut self, ctx: &egui::Context) {
        let Some(smoke) = &mut self.smoke else { return };
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot {
                user_data, image, ..
            } = event
                && let Some(name) = user_data
                    .data
                    .as_ref()
                    .and_then(|d| d.downcast_ref::<String>())
            {
                let bytes: Vec<u8> = image.pixels.iter().flat_map(|c| c.to_array()).collect();
                image::save_buffer(
                    smoke.out.join(name),
                    &bytes,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                smoke.captured.insert(name.clone());
            }
        }
    }

    pub(super) fn smoke_step(&mut self, ctx: &egui::Context, now: f64) {
        let Some(mut smoke) = self.smoke.take() else {
            return;
        };
        ctx.request_repaint_after(std::time::Duration::from_millis(25));
        if now > 90.0 {
            panic!(
                "workspace smoke timed out at phase {}, waiting for {:?}",
                smoke.phase, smoke.pending
            );
        }
        if now - smoke.at < 0.9
            || smoke.pending.iter().any(|n| !smoke.captured.contains(n))
            || self.files.browser.as_ref().is_some_and(|b| b.loading())
        {
            self.smoke = Some(smoke);
            return;
        }
        let capture = |smoke: &mut Smoke, name: &str, widget: Option<Widget>| {
            if let Some(w) = widget {
                assert!(
                    smoke.native_seen.contains(&w.index()),
                    "native widget viewport created"
                );
                // eframe 0.36's Glow immediate renderer does not drain
                // Screenshot actions. Record geometry evidence separately.
                smoke.report.push(format!(
                    "Native viewport verified: {} (child screenshot unsupported by this backend)",
                    w.title()
                ));
                return;
            }
            let id = widget.map_or(egui::ViewportId::ROOT, |w| {
                egui::ViewportId::from_hash_of(("otf-native", w.index()))
            });
            let name = format!("{name}.png");
            ctx.send_viewport_cmd_to(
                id,
                ViewportCommand::Screenshot(egui::UserData::new(name.clone())),
            );
            smoke.pending.push(name);
        };
        smoke.pending.clear();
        match smoke.phase {
            0 => {
                assert_eq!(self.s.dock.docked_count(), 2);
                assert!(self.current_px().is_some(), "fixture image decoded");
                capture(&mut smoke, "01-joined", None);
            }
            1 => {
                self.act(ctx, Act::Widget(Widget::Favourites), now);
            }
            2 => {
                assert_eq!(self.s.dock.docked_count(), 1);
                capture(&mut smoke, "02-files-fill-dock", None);
                capture(&mut smoke, "02-favourites-native", Some(Widget::Favourites));
            }
            3 => {
                self.act(ctx, Act::Tree, now);
            }
            4 => {
                assert!(self.focus.active());
                capture(&mut smoke, "03-folded-with-favourites-floating", None);
                self.act(ctx, Act::Tree, now);
            }
            5 => {
                self.act(ctx, Act::Widget(Widget::Files), now);
            }
            6 => {
                assert_eq!(self.s.dock.docked_count(), 0);
                assert!(self.focus.active());
                smoke.saved = Some(self.s.dock.widgets.map(|p| p.geometry));
                capture(&mut smoke, "04-both-floating-main", None);
                capture(&mut smoke, "04-files-native", Some(Widget::Files));
                capture(&mut smoke, "04-favourites-native", Some(Widget::Favourites));
            }
            7 => {
                self.act(ctx, Act::RecallTrees, now);
            }
            8 => {
                assert_eq!(self.s.dock.docked_count(), 2);
                assert!(!self.focus.active());
                capture(&mut smoke, "05-recalled", None);
            }
            9 => {
                self.act(ctx, Act::RecallTrees, now);
            }
            10 => {
                assert_eq!(self.s.dock.docked_count(), 0);
                assert_eq!(
                    self.s.dock.widgets.map(|p| p.geometry),
                    smoke.saved.unwrap()
                );
                capture(&mut smoke, "06-restored-main", None);
            }
            11 => {
                let w = Widget::Favourites;
                let initial = Pos2::from(self.s.dock.widgets[0].geometry.unwrap().position);
                let grab = Vec2::new(90.0, 12.0);
                let target = self.snap_zone.unwrap().center() - grab;
                self.float_drags[0] = Some(workspace::FloatDrag {
                    grab,
                    start: initial,
                    last: initial,
                    changed: now,
                });
                ctx.send_viewport_cmd_to(
                    egui::ViewportId::from_hash_of(("otf-native", w.index())),
                    ViewportCommand::OuterPosition(target),
                );
            }
            12 => {
                assert!(
                    !self.s.dock.floating(Widget::Favourites),
                    "native relocation into the dock snapped back"
                );
                assert_eq!(self.s.dock.widgets[0].geometry, smoke.saved.unwrap()[0]);
                capture(&mut smoke, "07-native-snap-back", None);
                self.act(ctx, Act::Widget(Widget::Files), now);
            }
            13 => {
                let missing = self.lib.as_ref().unwrap().tree.root.join("unavailable");
                self.files.start(ctx, missing);
            }
            14 => {
                assert!(matches!(
                    self.files.browser.as_ref().unwrap().entries[0].load,
                    octo::fsbrowser::Load::Error(_)
                ));
                capture(&mut smoke, "08-unavailable-folder", None);
            }
            15 => {
                self.files.browser.as_mut().unwrap().go_back();
            }
            16 => {
                self.s.style = 2;
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(Vec2::new(680.0, 480.0)));
            }
            17 => {
                capture(&mut smoke, "09-narrow-boxed", None);
            }
            18 => {
                smoke.report.push("PASS: actual native windows, one-widget fill, folding/image focus, pair recall/restore with exact geometry, native relocation/snap, unavailable-folder rendering, narrow boxed layout.".into());
                smoke.report.push("Input boundary: triple-click and Shift-click are covered by egui event tests. OS pointer drag and mixed-DPI/multi-monitor input were not exercised; Computer Use native pipe was unavailable. Root screenshots were captured; eframe Glow does not return immediate-child screenshots.".into());
                std::fs::write(smoke.out.join("RESULT.txt"), smoke.report.join("\n")).unwrap();
                ctx.send_viewport_cmd(ViewportCommand::Close);
                return;
            }
            _ => unreachable!(),
        }
        smoke.report.push(format!(
            "Phase {} passed; docked={}; image_focus={}; geometry={:?}",
            smoke.phase,
            self.s.dock.docked_count(),
            self.focus.active(),
            self.s.dock.widgets.map(|p| p.geometry)
        ));
        std::fs::write(smoke.out.join("progress.txt"), smoke.report.join("\n")).unwrap();
        smoke.phase += 1;
        smoke.at = now;
        self.smoke = Some(smoke);
    }
}
