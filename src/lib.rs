//! Octobrachial Tree Format (OTF) for Rusted Calamari.
//!
//! The tree is the library's map: it shows where you are browsing and the
//! categories the image in view can be filed into. `tree` holds the
//! directory arena and the category table, `layout` turns it into tagged,
//! keyed glyph lines and reflows them to a width, `tide` is the on-click
//! fit/mirror/retract engine, `paint` colours lines like a syntax
//! highlighter.

pub mod config;
pub mod fsbrowser;
pub mod layout;
pub mod ops;
pub mod paint;
pub mod tide;
pub mod tree;

pub use layout::{render, Frame, Style};
pub use tide::{click, Trip, View};
pub use tree::{Side, Tree};
