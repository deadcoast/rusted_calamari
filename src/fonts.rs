//! The twelve preinstalled faces from the Ultimate Oldschool PC Font Pack
//! (VileR, int10h.org, CC BY-SA 4.0), pixel-outline (Px437) cuts. All of
//! them carry the CP437 box and block glyphs the tree is drawn with.
//!
//! Icons come from Phosphor (phosphoricons.com, MIT) through the
//! egui-phosphor crate, in two cuts: bold for idle tools, fill for tools
//! that are engaged. Each cut is its own family, so icon codepoints never
//! fall back through the text faces.

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const ICON_BOLD: &str = "ph-bold";
pub const ICON_FILL: &str = "ph-fill";

/// An icon font at `size` points; `fill` picks the solid cut.
pub fn icon(size: f32, fill: bool) -> FontId {
    FontId::new(size, FontFamily::Name((if fill { ICON_FILL } else { ICON_BOLD }).into()))
}

pub struct Face {
    pub name: &'static str,
    pub bytes: &'static [u8],
}

macro_rules! face {
    ($name:literal, $file:literal) => {
        Face { name: $name, bytes: include_bytes!(concat!("../assets/fonts/", $file)) }
    };
}

pub const FACES: [Face; 12] = [
    face!("IBM Model3x Alt1", "Px437_IBM_Model3x_Alt1.ttf"),
    face!("Kaypro2K G-2y", "Px437_Kaypro2K_G-2y.ttf"),
    face!("MBytePC230 Mono", "Px437_MBytePC230_Mono.ttf"),
    face!("MBytePC230 CGA-2y", "Px437_MBytePC230_CGA-2y.ttf"),
    face!("Phoenix BIOS-2y", "Px437_Phoenix_BIOS-2y.ttf"),
    face!("ToshibaTxL2 8x16", "Px437_ToshibaTxL2_8x16.ttf"),
    face!("SperryPC CGA-2y", "Px437_SperryPC_CGA-2y.ttf"),
    face!("SperryPC 8x16", "Px437_SperryPC_8x16.ttf"),
    face!("Tandy1K-I 200L-2y", "Px437_Tandy1K-I_200L-2y.ttf"),
    face!("SanyoMBC16-2y", "Px437_SanyoMBC16-2y.ttf"),
    face!("OlivettiThin 8x16", "Px437_OlivettiThin_8x16.ttf"),
    face!("Nix8810 M35-2y", "Px437_Nix8810_M35-2y.ttf"),
];

/// Pixel faces stay crisp at multiples of their 16px cell.
pub const SIZES: [f32; 6] = [12.0, 16.0, 20.0, 24.0, 28.0, 32.0];

/// Make `sel` the face for everything: tree, menus, text fields.
pub fn install(ctx: &egui::Context, sel: usize) {
    let mut defs = FontDefinitions::default();
    let face = &FACES[sel % FACES.len()];
    defs.font_data.insert(face.name.to_owned(), Arc::new(FontData::from_static(face.bytes)));
    for fam in [FontFamily::Monospace, FontFamily::Proportional] {
        defs.families.entry(fam).or_default().insert(0, face.name.to_owned());
    }
    for (name, bytes) in [(ICON_BOLD, egui_phosphor::bytes::bold::FONT), (ICON_FILL, egui_phosphor::bytes::fill::FONT)] {
        defs.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
        defs.families.insert(FontFamily::Name(name.into()), vec![name.to_owned()]);
    }
    ctx.set_fonts(defs);
}
