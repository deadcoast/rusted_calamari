//! Convert the supplied transparent logo into runtime PNG and multi-size ICO assets.
//! Run from any directory with `cargo run --release --locked --example make_icons`.
use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageReader};
use std::fs::{self, File};
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = ImageReader::open(root.join("assets/logos/rc-logo-transparant-bg.png"))?
        .decode()?
        .into_rgba8();
    assert_eq!(source.width(), source.height(), "The source must be square");
    assert!(
        source.pixels().any(|p| p[3] == 0),
        "Keep the transparent background"
    );
    let dest = root.join("assets/icons");
    fs::create_dir_all(&dest)?;
    let mut frames = Vec::new();
    for size in [16, 20, 24, 32, 40, 48, 64, 128, 256] {
        let icon = image::imageops::resize(&source, size, size, FilterType::Lanczos3);
        if size == 256 {
            icon.save(dest.join("rusted-calamari.png"))?;
        }
        frames.push(IcoFrame::as_png(
            &icon,
            size,
            size,
            ExtendedColorType::Rgba8,
        )?);
    }
    IcoEncoder::new(File::create(dest.join("rusted-calamari.ico"))?).encode_images(&frames)?;
    println!(
        "Generated transparent 256px PNG and nine-size ICO from {}x{} logo",
        source.width(),
        source.height()
    );
    Ok(())
}
