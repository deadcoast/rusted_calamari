//! Native rendering/placement smoke runner. Uses isolated data and fixtures;
//! does not inject operating-system input or touch the user's library.
mod app;
mod browserpane;
mod dock;
mod focus;
mod fonts;
mod glyph;
mod icon;
mod loader;
mod saver;
mod theme;
mod toolbar;
mod treeview;
mod window;

use eframe::egui;
use std::path::PathBuf;

fn main() -> eframe::Result {
    let out = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .expect("Pass a fresh evidence directory");
    assert!(
        !out.exists(),
        "Use a fresh directory to preserve earlier evidence"
    );
    std::fs::create_dir_all(&out).unwrap();
    let root = out.join("fixture");
    for dir in ["", "_fav_", "1", "portraits", "empty", "many"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    for (name, w, h) in [
        ("sample.png", 800, 600),
        ("portraits/portrait.png", 500, 750),
        ("_fav_/favourite.png", 500, 500),
    ] {
        let img = image::RgbImage::from_fn(w, h, |x, y| {
            image::Rgb([
                (x * 180 / w + 30) as u8,
                (y * 140 / h + 35) as u8,
                if (x / 50 + y / 50) % 2 == 0 { 100 } else { 125 },
            ])
        });
        img.save(root.join(name)).unwrap();
    }
    for n in 0..22 {
        std::fs::write(
            root.join(format!("many/note-{n:02}.txt")),
            b"Read-only browser fixture.\n",
        )
        .unwrap();
    }
    std::fs::write(
        root.join("notes with a long filename.txt"),
        b"Files are never executed.\n",
    )
    .unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Calamari workspace QA")
            .with_icon(icon::data())
            .with_inner_size([1200.0, 850.0])
            .with_position([60.0, 60.0])
            .with_min_inner_size([320.0, 200.0])
            .with_decorations(false),
        persistence_path: Some(out.join("profile")),
        ..Default::default()
    };
    eframe::run_native(
        "Calamari workspace QA",
        options,
        Box::new(move |cc| {
            let mut app = app::App::new(cc, Some(root));
            app.enable_smoke(out);
            Ok(Box::new(app))
        }),
    )
}
