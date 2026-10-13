//! Disk operations. Every one returns what undo needs.

use crate::tree::Tree;
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader};
use std::fs;
use std::io::{self, Cursor};
use std::path::{Path, PathBuf};

/// Decode with EXIF orientation applied.
pub fn decode(p: &Path) -> Result<DynamicImage, String> {
    let mut dec = ImageReader::open(p)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?
        .into_decoder()
        .map_err(|e| e.to_string())?;
    let orient = dec.orientation().ok();
    let mut img = DynamicImage::from_decoder(dec).map_err(|e| e.to_string())?;
    if let Some(o) = orient {
        img.apply_orientation(o);
    }
    Ok(img)
}

/// `name` if free in `dir`, else `stem (2).ext`, `stem (3).ext`, ...
pub fn unique(dir: &Path, name: &str) -> String {
    if !dir.join(name).exists() {
        return name.to_string();
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    (2..)
        .map(|i| format!("{stem} ({i}){ext}"))
        .find(|n| !dir.join(n).exists())
        .unwrap()
}

/// Move `src` into `dst_dir`, creating it. Returns the name it landed as.
pub fn move_into(src: &Path, dst_dir: &Path, name: &str) -> io::Result<String> {
    fs::create_dir_all(dst_dir)?;
    let n = unique(dst_dir, name);
    let dst = dst_dir.join(&n);
    if fs::rename(src, &dst).is_err() {
        // Different volume: copy, then remove.
        fs::copy(src, &dst)?;
        fs::remove_file(src)?;
    }
    Ok(n)
}

pub fn copy_into(src: &Path, dst_dir: &Path) -> io::Result<String> {
    fs::create_dir_all(dst_dir)?;
    let name = src
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let n = unique(dst_dir, &name);
    fs::copy(src, dst_dir.join(&n))?;
    Ok(n)
}

/// A pending edit. Crop rectangles are normalized (0..1) against the image
/// as it stands after the ops before it, so they hold at any resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    RotCw,
    RotCcw,
    FlipH,
    FlipV,
    Crop([f32; 4]),
}

impl Op {
    pub fn label(&self) -> &'static str {
        match self {
            Op::RotCw => "Rotate right",
            Op::RotCcw => "Rotate left",
            Op::FlipH => "Flip horizontal",
            Op::FlipV => "Flip vertical",
            Op::Crop(_) => "Crop",
        }
    }
}

fn crop_px(w: u32, h: u32, r: [f32; 4]) -> (u32, u32, u32, u32) {
    let x0 = (r[0].clamp(0.0, 1.0) * w as f32).round() as u32;
    let y0 = (r[1].clamp(0.0, 1.0) * h as f32).round() as u32;
    let x1 = (r[2].clamp(0.0, 1.0) * w as f32).round() as u32;
    let y1 = (r[3].clamp(0.0, 1.0) * h as f32).round() as u32;
    let x0 = x0.min(w.saturating_sub(1));
    let y0 = y0.min(h.saturating_sub(1));
    (
        x0,
        y0,
        x1.saturating_sub(x0).max(1).min(w - x0),
        y1.saturating_sub(y0).max(1).min(h - y0),
    )
}

pub fn apply(mut img: DynamicImage, ops: &[Op]) -> DynamicImage {
    for op in ops {
        img = match *op {
            Op::RotCw => img.rotate90(),
            Op::RotCcw => img.rotate270(),
            Op::FlipH => img.fliph(),
            Op::FlipV => img.flipv(),
            Op::Crop(r) => {
                let (x, y, w, h) = crop_px(img.width(), img.height(), r);
                img.crop_imm(x, y, w, h)
            }
        };
    }
    img
}

/// Pixel size of an image of size `w`x`h` after `ops`.
pub fn dims(mut w: u32, mut h: u32, ops: &[Op]) -> [u32; 2] {
    for op in ops {
        match *op {
            Op::RotCw | Op::RotCcw => std::mem::swap(&mut w, &mut h),
            Op::Crop(r) => {
                let (_, _, cw, ch) = crop_px(w, h, r);
                (w, h) = (cw, ch);
            }
            _ => {}
        }
    }
    [w, h]
}

fn ext_of(p: &Path) -> String {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Encode for the format named by `path`'s extension. JPEG at quality 95.
pub fn encode(img: &DynamicImage, path: &Path) -> Result<Vec<u8>, String> {
    let ext = ext_of(path);
    let mut buf = Vec::new();
    match ext.as_str() {
        "jpg" | "jpeg" => JpegEncoder::new_with_quality(&mut buf, 95)
            .encode_image(&img.to_rgb8())
            .map_err(|e| e.to_string())?,
        "webp" => DynamicImage::ImageRgba8(img.to_rgba8())
            .write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)
            .map_err(|e| e.to_string())?,
        _ => {
            let fmt =
                ImageFormat::from_path(path).map_err(|_| format!("cannot write .{ext} files"))?;
            img.write_to(&mut Cursor::new(&mut buf), fmt)
                .map_err(|e| e.to_string())?
        }
    }
    Ok(buf)
}

/// Replace a file's bytes through a temp file and a rename, so a failed
/// write never leaves half a file.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension(format!("{}.calamari-tmp", ext_of(path)));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path).inspect_err(|_| {
        let _ = fs::remove_file(&tmp);
    })
}

/// Write `ops` into the file itself. Returns the original bytes for undo.
pub fn save_ops(path: &Path, ops: &[Op]) -> Result<Vec<u8>, String> {
    if ext_of(path) == "gif" {
        return Err("gif files keep their frames; use Export as".into());
    }
    let orig = fs::read(path).map_err(|e| e.to_string())?;
    let img = apply(decode(path)?, ops);
    let buf = encode(&img, path)?;
    write_atomic(path, &buf).map_err(|e| e.to_string())?;
    Ok(orig)
}

/// Write `src` with `ops` applied to a new file; format from `dest`'s extension.
pub fn export_ops(src: &Path, ops: &[Op], dest: &Path) -> Result<(), String> {
    let img = apply(decode(src)?, ops);
    let buf = encode(&img, dest)?;
    fs::write(dest, buf).map_err(|e| e.to_string())
}

pub fn trash_dir(t: &Tree) -> PathBuf {
    t.root.join(".calamari_trash")
}

fn csv(s: &str) -> String {
    if s.contains([',', '"', '\n']) {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

/// Every media file with its category, tier and cid.
pub fn manifest(t: &Tree) -> String {
    let mut out = String::from("path,category,tier,cid\n");
    for (i, n) in t.nodes.iter().enumerate() {
        let dir = t.path(i);
        for f in &n.files {
            let rel = if dir == "." {
                f.name.clone()
            } else {
                format!("{dir}/{}", f.name)
            };
            let (cat, tier, cid) = match f.cat {
                Some(c) => {
                    let c = &t.cats[c];
                    (
                        c.name.clone(),
                        c.tier.to_string(),
                        format!("#{:02X}{:02X}{:02X}", c.cid.0, c.cid.1, c.cid.2),
                    )
                }
                None => (String::new(), String::new(), String::new()),
            };
            out.push_str(&format!("{},{},{},{}\n", csv(&rel), csv(&cat), tier, cid));
        }
    }
    out
}

/// Copy every sorted file into `dest/<category>/`. Returns files copied.
pub fn export_set(t: &Tree, dest: &Path) -> io::Result<usize> {
    let mut n = 0;
    for (i, node) in t.nodes.iter().enumerate() {
        for f in &node.files {
            if let Some(c) = f.cat {
                copy_into(&t.dir(i).join(&f.name), &dest.join(&t.cats[c].name))?;
                n += 1;
            }
        }
    }
    Ok(n)
}
