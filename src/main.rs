#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]
//! Rusted Calamari. `cargo run --release` opens the window; the last
//! library reopens on its own. A folder path argument opens that instead.
//! The window has no system frame: the interface's own top bar moves it,
//! and its edges resize it (see `window`).

mod app;
mod theme;
mod focus;
mod fonts;
mod glyph;
mod loader;
mod saver;
mod toolbar;
mod treeview;
mod window;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let arg = std::env::args().nth(1).map(PathBuf::from);
    let opts = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Rusted Calamari")
            .with_inner_size([1440.0, 900.0])
            // Small enough for image focus to wrap a portrait picture.
            .with_min_inner_size([320.0, 200.0])
            .with_decorations(false)
            .with_drag_and_drop(true),
        ..Default::default()
    };
    eframe::run_native("Rusted Calamari", opts, Box::new(move |cc| Ok(Box::new(app::App::new(cc, arg)))))
}
