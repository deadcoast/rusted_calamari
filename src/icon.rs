//! One embedded logo shared by the main window and both floating trees.
use eframe::egui::IconData;
use std::sync::{Arc, OnceLock};

pub fn data() -> Arc<IconData> {
    static ICON: OnceLock<Arc<IconData>> = OnceLock::new();
    Arc::clone(ICON.get_or_init(|| {
        Arc::new(
            eframe::icon_data::from_png_bytes(include_bytes!(
                "../assets/icons/rusted-calamari.png"
            ))
            .expect("The embedded app icon must be a valid PNG"),
        )
    }))
}
