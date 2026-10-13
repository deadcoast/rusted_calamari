//! Integration checks at the real egui input/layout boundary. These do not
//! substitute for physical native-window dragging or multiple-monitor QA.
use super::*;
use eframe::egui::{Modifiers, PointerButton, RawInput};

fn fixture(tag: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-workspace").join(format!("{tag}-{}", std::process::id()));
    std::fs::create_dir_all(root.join("nested")).unwrap();
    image::RgbImage::from_fn(32, 24, |x, y| image::Rgb([x as u8, y as u8, 120])).save(root.join("a.png")).unwrap();
    image::RgbImage::from_pixel(24, 32, image::Rgb([25, 60, 90])).save(root.join("nested/b.png")).unwrap();
    root
}
fn app(tag: &str) -> (egui::Context, App, PathBuf) {
    let root = fixture(tag);
    let ctx = egui::Context::default();
    let app = App::with_settings(ctx.clone(), Settings::default(), Some(root.clone()));
    (ctx, app, root)
}
fn input(time: f64, mut events: Vec<Event>, modifiers: Modifiers) -> RawInput {
    events.insert(0, Event::ModifiersChanged(modifiers));
    RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1200.0, 800.0))), time: Some(time), events, ..Default::default() }
}
fn pointer(pos: Pos2, down: bool, modifiers: Modifiers) -> Vec<Event> {
    vec![Event::PointerMoved(pos), Event::PointerButton { pos, button: PointerButton::Primary, pressed: down, modifiers }]
}

#[test]
fn fold_tab_triple_clicks_recall_then_restore_without_intermediate_fold() {
    let (ctx, mut app, _) = app("triple");
    app.toggle_widget(Widget::Favourites);
    app.toggle_widget(Widget::Files);
    let positions = app.s.dock.widgets.map(|p| p.geometry);
    let tab = Rect::from_min_size(Pos2::new(20.0, 50.0), Vec2::new(28.0, 45.0));
    let _ = ctx.run_ui(input(0.0, vec![], Modifiers::NONE), |ui| { let g = Grid::new(&ctx, 20.0); app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0); });
    let _ = ctx.run_ui(input(0.5, vec![Event::PointerMoved(tab.center())], Modifiers::NONE), |ui| { let g = Grid::new(&ctx, 20.0); app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0); });
    for cycle in 0..2 {
        for click in 0..3 {
            for down in [true, false] {
                let now = 1.0 + cycle as f64 + click as f64 * 0.1 + if down { 0.0 } else { 0.03 };
                let _ = ctx.run_ui(input(now, pointer(tab.center(), down, Modifiers::NONE), Modifiers::NONE), |ui| {
                    let g = Grid::new(&ctx, 20.0);
                    if let Some(a) = app.fold_tab(ui.ctx(), &g, tab, ">", Act::Tree, 1.0) { app.act(&ctx, a, now); }
                });
            }
            if click < 2 { assert_eq!(app.s.dock.docked_count(), if cycle == 0 { 0 } else { 2 }); }
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
    let header = Rect::from_min_size(Pos2::new(10.0, 80.0), Vec2::new(400.0, 300.0));
    let point = Pos2::new(120.0, 92.0);
    let _ = ctx.run_ui(input(0.0, vec![], Modifiers::NONE), |ui| { let g = Grid::new(&ctx, 20.0); app.widget_header(ui, Widget::Favourites, header, &g, false, 0.0); });
    for cycle in 0..2 {
        for down in [true, false] {
            let now = 1.0 + cycle as f64 + if down { 0.0 } else { 0.03 };
            let _ = ctx.run_ui(input(now, pointer(point, down, Modifiers::SHIFT), Modifiers::SHIFT), |ui| {
                let g = Grid::new(&ctx, 20.0);
                app.widget_header(ui, Widget::Favourites, header, &g, false, now);
            });
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
    let path = root.join("nested/b.png");
    app.file_action(crate::browserpane::Action::Preview(path.clone()), 1.0);
    assert_eq!(app.current(), Some(path));
    assert_eq!(app.lib.as_ref().unwrap().tree.root, root);
    assert_eq!(app.lib.as_ref().unwrap().tree.cats.len(), config::defaults().len());
}

#[test]
fn changing_library_protects_unsaved_edits_on_cancel_and_save() {
    let (ctx, mut app, root) = app("unsaved");
    let original = root.join("a.png");
    let bytes = std::fs::read(&original).unwrap();
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
    let saved = image::open(original).unwrap();
    assert_eq!((saved.width(), saved.height()), (24, 32));
}

#[test]
fn docked_widgets_render_at_small_and_large_sizes() {
    let (ctx, mut app, _) = app("layout");
    for size in [Vec2::new(260.0, 220.0), Vec2::new(520.0, 900.0)] {
        let output = ctx.run_ui(RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), time: Some(1.0), ..Default::default() }, |ui| {
            let g = Grid::new(&ctx, 20.0);
            app.docked_trees(ui, ui.max_rect(), &g, 1.0);
        });
        assert!(!output.shapes.is_empty());
    }
}
