//! Integration checks at the real egui input/layout boundary. These do not
//! substitute for physical native-window dragging or multiple-monitor QA.
use super::*;
use eframe::egui::{Modifiers, PointerButton, RawInput};

fn fixture(tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-workspace")
        .join(format!("{tag}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    image::RgbImage::from_fn(32, 24, |x, y| image::Rgb([x as u8, y as u8, 120]))
        .save(root.join("a.png"))
        .unwrap();
    image::RgbImage::from_pixel(24, 32, image::Rgb([25, 60, 90]))
        .save(root.join("nested/b.png"))
        .unwrap();
    root
}
fn app(tag: &str) -> (egui::Context, App, PathBuf) {
    let root: PathBuf = fixture(tag);
    let ctx: egui::Context = egui::Context::default();
    let app: App = App::with_settings(ctx.clone(), Settings::default(), Some(root.clone()));
    (ctx, app, root)
}
fn input(time: f64, mut events: Vec<Event>, modifiers: Modifiers) -> RawInput {
    events.insert(0, Event::ModifiersChanged(modifiers));
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))),
        time: Some(time),
        events,
        ..Default::default()
    }
}
fn pointer(pos: Pos2, down: bool, modifiers: Modifiers) -> Vec<Event> {
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: down,
            modifiers,
        },
    ]
}

#[test]
fn fold_tab_triple_clicks_recall_then_restore_without_intermediate_fold() {
    let (ctx, mut app, _) = app("triple");
    app.toggle_widget(Widget::Favourites);
    app.toggle_widget(Widget::Files);
    let positions: [Option<crate::dock::Geometry>; 2] = app.s.dock.widgets.map(|p| p.geometry);
    let tab: Rect = Rect::from_min_size(Pos2::new(20.0, 50.0), Vec2::new(28.0, 45.0));
    let _ = ctx.run_ui(input(0.0, vec![], Modifiers::NONE), |ui| {
        let g = Grid::new(&ctx, 20.0);
        app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0);
    });
    let _ = ctx.run_ui(
        input(
            0.5,
            vec![Event::PointerMoved(tab.center())],
            Modifiers::NONE,
        ),
        |ui| {
            let g = Grid::new(&ctx, 20.0);
            app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0);
        },
    );
    for cycle in 0..2 {
        for click in 0..3 {
            for down in [true, false] {
                let now = 1.0 + cycle as f64 + click as f64 * 0.1 + if down { 0.0 } else { 0.03 };
                let _ = ctx.run_ui(
                    input(
                        now,
                        pointer(tab.center(), down, Modifiers::NONE),
                        Modifiers::NONE,
                    ),
                    |ui| {
                        let g = Grid::new(&ctx, 20.0);
                        if let Some(a) = app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0) {
                            app.act(&ctx, a, now);
                        }
                    },
                );
            }
            if click < 2 {
                assert_eq!(app.s.dock.docked_count(), if cycle == 0 { 0 } else { 2 });
            }
        }
        assert_eq!(app.s.dock.docked_count(), if cycle == 0 { 2 } else { 0 });
    }
    assert_eq!(app.s.dock.widgets.map(|p| p.geometry), positions);
}

#[test]
fn shift_header_click_toggles_without_recreating_tree_state() {
    let (ctx, mut app, root) = app("shift");
    app.view.scroll = 2;
    app.s.dock.split = 0.42;
    let header: Rect = Rect::from_min_size(Pos2::new(10.0, 80.0), Vec2::new(400.0, 300.0));
    let point: Pos2 = Pos2::new(120.0, 92.0);
    let _ = ctx.run_ui(input(0.0, vec![], Modifiers::NONE), |ui| {
        let g = Grid::new(&ctx, 20.0);
        app.widget_header(ui, Widget::Favourites, header, &g, false, 0.0);
    });
    for cycle in 0..2 {
        for down in [true, false] {
            let now: f64 = 1.0 + cycle as f64 + if down { 0.0 } else { 0.03 };
            let _ = ctx.run_ui(
                input(
                    now,
                    pointer(point, down, Modifiers::SHIFT),
                    Modifiers::SHIFT,
                ),
                |ui| {
                    let g: Grid = Grid::new(&ctx, 20.0);
                    app.widget_header(ui, Widget::Favourites, header, &g, false, now);
                },
            );
        }
        assert_eq!(app.s.dock.floating(Widget::Favourites), cycle == 0);
        assert_eq!(app.view.scroll, 2);
        assert_eq!(app.s.dock.split, 0.42);
        assert_eq!(app.files.browser.as_ref().unwrap().tree.root, root);
    }
}

#[test]
fn browser_preview_inside_library_preserves_favourites_root() {
    let (_, mut app, root) = app("preview");
    let path: PathBuf = root.join("nested/b.png");
    app.file_action(crate::browserpane::Action::Preview(path.clone()), 1.0);
    assert_eq!(app.current(), Some(path));
    assert_eq!(app.lib.as_ref().unwrap().tree.root, root);
    assert_eq!(
        app.lib.as_ref().unwrap().tree.cats.len(),
        config::defaults().len()
    );
}

#[test]
fn changing_library_protects_unsaved_edits_on_cancel_and_save() {
    let (ctx, mut app, root) = app("unsaved");
    let original: PathBuf = root.join("a.png");
    let bytes: Vec<u8> = std::fs::read(&original).unwrap();
    app.add_op_now(&ctx, &original, Op::RotCw).unwrap();
    app.request_library(root.join("nested"), None, 1.0);
    assert!(matches!(app.dialog, Dialog::Unsaved));
    assert_eq!(app.lib.as_ref().unwrap().tree.root, root);
    app.act(&ctx, Act::Close, 1.1);
    assert!(app.next_library.is_none());
    assert!(app.pending.contains_key(&original));
    assert_eq!(std::fs::read(&original).unwrap(), bytes);
    app.request_library(root.join("nested"), None, 2.0);
    app.act(&ctx, Act::SaveAllQuit, 2.1);
    assert!(!app.quitting);
    assert_eq!(app.lib.as_ref().unwrap().tree.root, root.join("nested"));
    assert!(app.pending.is_empty());
    let saved: DynamicImage = image::open(original).unwrap();
    assert_eq!((saved.width(), saved.height()), (24, 32));
}

#[test]
fn docked_widgets_render_at_small_and_large_sizes() {
    let (ctx, mut app, _) = app("layout");
    for size in [Vec2::new(260.0, 220.0), Vec2::new(520.0, 900.0)] {
        let output: egui::FullOutput = ctx.run_ui(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                time: Some(1.0),
                ..Default::default()
            },
            |ui| {
                let g: Grid = Grid::new(&ctx, 20.0);
                app.docked_trees(ui, ui.max_rect(), &g, 1.0);
            },
        );
        assert!(!output.shapes.is_empty());
    }
}

#[test]
fn dragging_a_docked_header_out_detaches_and_preserves_split() {
    let (ctx, mut app, _) = app("drag");
    app.main_rect = Some(Rect::from_min_size(
        Pos2::new(100.0, 100.0),
        Vec2::new(1200.0, 800.0),
    ));
    app.snap_zone = Some(Rect::from_min_size(
        Pos2::new(100.0, 100.0),
        Vec2::new(400.0, 800.0),
    ));
    app.s.dock.split = 0.42;
    let bounds = Rect::from_min_size(Pos2::new(10.0, 80.0), Vec2::new(400.0, 300.0));
    let start = Pos2::new(120.0, 92.0);
    let end = Pos2::new(700.0, 160.0);
    let events = [
        vec![],
        vec![Event::PointerMoved(start)],
        pointer(start, true, Modifiers::NONE),
        vec![Event::PointerMoved(end)],
        pointer(end, false, Modifiers::NONE),
    ];
    for (step, events) in events.into_iter().enumerate() {
        let now = step as f64 * 0.1;
        let _ = ctx.run_ui(input(now, events, Modifiers::NONE), |ui| {
            let g = Grid::new(&ctx, 20.0);
            app.widget_header(ui, Widget::Favourites, bounds, &g, false, now);
        });
    }
    assert!(app.s.dock.floating(Widget::Favourites));
    assert_eq!(app.s.dock.docked_count(), 1);
    assert_eq!(app.s.dock.split, 0.42);
    assert_eq!(
        app.s.dock.widgets[0].geometry.unwrap().position,
        [690.0, 248.0]
    );
}

#[test]
fn settings_load_legacy_and_round_trip_widget_geometry() {
    #[derive(Default)]
    struct Store(HashMap<String, String>);
    impl eframe::Storage for Store {
        fn get_string(&self, k: &str) -> Option<String> {
            self.0.get(k).cloned()
        }
        fn set_string(&mut self, k: &str, v: String) {
            self.0.insert(k.into(), v);
        }
        fn remove_string(&mut self, k: &str) {
            self.0.remove(k);
        }
        fn flush(&mut self) {}
    }
    let mut store = Store::default();
    store.0.insert(
        "settings".into(),
        "(font: 3, tree: false, image: true)".into(),
    );
    let mut settings: Settings = eframe::get_value(&store, "settings").unwrap();
    assert_eq!(settings.font, 3);
    assert!(!settings.tree);
    assert_eq!(settings.dock.docked_count(), 2);
    assert!(settings.files_root.is_none());
    let geometry = crate::dock::Geometry {
        position: [-500.0, 100.0],
        size: [480.0, 620.0],
    };
    settings.dock.detach(Widget::Files, geometry);
    settings.dock.split = 0.35;
    settings.files_root = Some(PathBuf::from("C:/"));
    eframe::set_value(&mut store, "settings", &settings);
    let restored: Settings = eframe::get_value(&store, "settings").unwrap();
    assert!(restored.dock.floating(Widget::Files));
    assert_eq!(restored.dock.widgets[1].geometry, Some(geometry));
    assert_eq!(restored.dock.split, 0.35);
    assert_eq!(restored.files_root, settings.files_root);
}

#[test]
fn floating_files_consume_navigation_and_forward_app_keys_to_root() {
    let (ctx, mut app, _) = app("widget-keys");
    app.files.focused = true;
    let events = [Key::ArrowDown, Key::Tab]
        .into_iter()
        .map(|key| Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        })
        .collect();
    let _ = ctx.run_ui(input(1.0, events, Modifiers::NONE), |_| {
        app.files.keys(&ctx);
        app.widget_shortcuts(&ctx);
    });
    assert_eq!(app.widget_keys.len(), 1);
    assert!(matches!(
        app.widget_keys[0],
        Event::Key { key: Key::Tab, .. }
    ));
    assert!(app.s.tree, "root action must wait for the root frame");
    let _ = ctx.run_ui(input(1.1, vec![], Modifiers::NONE), |_| app.keys(&ctx, 1.1));
    assert!(!app.s.tree);
    assert!(app.widget_keys.is_empty());
}

#[test]
fn folded_files_do_not_consume_image_navigation() {
    let (ctx, mut app, root) = app("folded-keys");
    std::fs::copy(root.join("a.png"), root.join("c.png")).unwrap();
    assert!(app.open_library(&root, 0.0));
    app.files.focused = true;
    app.s.tree = false;
    for folded in [true, false] {
        app.s.tree = !folded;
        let event = Event::Key {
            key: Key::ArrowRight,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        let _ = ctx.run_ui(input(1.0, vec![event], Modifiers::NONE), |_| {
            app.keys(&ctx, 1.0)
        });
        assert_eq!(
            app.lib.as_ref().unwrap().cursor,
            1,
            "folded Files must yield image keys; visible focused Files must consume them"
        );
    }
}
