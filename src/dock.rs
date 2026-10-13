//! Placement and gestures for the two OTF widgets. Content state lives in
//! the widgets, never in their docked/floating containers.

use eframe::egui::{Pos2, Rect};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Widget {
    Favourites,
    Files,
}

impl Widget {
    pub const ALL: [Self; 2] = [Self::Favourites, Self::Files];
    pub fn index(self) -> usize { match self { Self::Favourites => 0, Self::Files => 1 } }
    pub fn title(self) -> &'static str { match self { Self::Favourites => "Favourites", Self::Files => "Files" } }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub position: [f32; 2],
    pub size: [f32; 2],
}

impl Geometry {
    pub fn from_rect(r: Rect) -> Self { Self { position: r.min.into(), size: r.size().into() } }
    pub fn valid(self) -> bool {
        self.position.iter().chain(&self.size).all(|n| n.is_finite()) && self.size[0] >= 240.0 && self.size[1] >= 160.0
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Placement {
    pub floating: bool,
    /// Last settled detached position, retained when the widget docks.
    pub geometry: Option<Geometry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Dock {
    pub widgets: [Placement; 2],
    pub split: f32,
    /// Set only by recalling two floating widgets. Enables the reverse
    /// triple-click, without making ordinary docked triple-clicks detach.
    pub pair_recalled: bool,
}

impl Default for Dock {
    fn default() -> Self { Self { widgets: [Placement::default(); 2], split: 0.5, pair_recalled: false } }
}

impl Dock {
    pub fn sanitize(&mut self) {
        self.split = if self.split.is_finite() { self.split.clamp(0.15, 0.85) } else { 0.5 };
        for p in &mut self.widgets {
            if p.geometry.is_some_and(|g| !g.valid()) { p.geometry = None; }
        }
        if self.docked_count() != 2 { self.pair_recalled = false; }
    }
    pub fn floating(&self, w: Widget) -> bool { self.widgets[w.index()].floating }
    pub fn docked_count(&self) -> usize { self.widgets.iter().filter(|p| !p.floating).count() }
    pub fn pair_allowed(&self) -> bool { self.docked_count() == 0 || (self.docked_count() == 2 && self.pair_recalled) }
    pub fn detach(&mut self, w: Widget, fallback: Geometry) {
        let p = &mut self.widgets[w.index()];
        p.geometry.get_or_insert(fallback);
        p.floating = true;
        self.pair_recalled = false;
    }
    pub fn dock(&mut self, w: Widget) {
        self.widgets[w.index()].floating = false;
        self.pair_recalled = false;
    }
    pub fn toggle_pair(&mut self) -> bool {
        if !self.pair_allowed() { return false; }
        let recall = self.docked_count() == 0;
        for p in &mut self.widgets { p.floating = !recall; }
        self.pair_recalled = recall;
        true
    }
    /// A lone widget takes the whole dock. The saved split is unchanged.
    pub fn regions(&self, bounds: Rect, gap: f32, minimum: f32) -> [Option<Rect>; 2] {
        match self.docked_count() {
            0 => [None, None],
            1 => std::array::from_fn(|i| (!self.widgets[i].floating).then_some(bounds)),
            _ => {
                let gap = gap.min(bounds.height().max(0.0));
                let available = (bounds.height() - gap).max(0.0);
                let minimum = minimum.min(available / 2.0);
                let h = (available * self.split).clamp(minimum, available - minimum);
                let top = Rect::from_min_max(bounds.min, Pos2::new(bounds.right(), bounds.top() + h));
                let bottom = Rect::from_min_max(Pos2::new(bounds.left(), top.bottom() + gap), bounds.max);
                [Some(top), Some(bottom)]
            }
        }
    }
}

#[derive(Default)]
pub struct FoldClicks { count: u8, last: f64 }

#[derive(Debug, PartialEq, Eq)]
pub enum FoldGesture { None, Single, Pair }

impl FoldClicks {
    pub const INTERVAL: f64 = 0.4;
    pub fn click(&mut self, now: f64, pair_allowed: bool) -> FoldGesture {
        if !pair_allowed { self.count = 0; return FoldGesture::Single; }
        self.count = if now - self.last <= Self::INTERVAL { self.count + 1 } else { 1 };
        self.last = now;
        if self.count == 3 { self.count = 0; FoldGesture::Pair } else { FoldGesture::None }
    }
    pub fn poll(&mut self, now: f64) -> FoldGesture {
        if self.count > 0 && now - self.last > Self::INTERVAL {
            self.count = 0;
            FoldGesture::Single
        } else { FoldGesture::None }
    }
    pub fn pending(&self) -> bool { self.count > 0 }
    pub fn reset(&mut self) { self.count = 0; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::Vec2;
    fn geometry(x: f32) -> Geometry { Geometry { position: [x, 60.0], size: [380.0, 500.0] } }
    #[test]
    fn remaining_widget_fills_dock_and_split_returns() {
        let mut dock = Dock { split: 0.37, ..Dock::default() };
        let bounds = Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 800.0));
        let before = dock.regions(bounds, 1.0, 100.0);
        for w in Widget::ALL {
            dock.detach(w, geometry(800.0));
            assert_eq!(dock.regions(bounds, 1.0, 100.0)[1 - w.index()], Some(bounds));
            dock.dock(w);
            assert_eq!(dock.regions(bounds, 1.0, 100.0), before);
        }
    }
    #[test]
    fn recall_and_restore_keep_both_geometries() {
        let mut dock = Dock::default();
        assert!(!dock.toggle_pair());
        dock.detach(Widget::Favourites, geometry(800.0));
        assert!(!dock.toggle_pair());
        dock.detach(Widget::Files, geometry(1250.0));
        let before = dock.widgets.map(|p| p.geometry);
        assert!(dock.toggle_pair());
        assert_eq!(dock.docked_count(), 2);
        assert!(dock.toggle_pair());
        assert_eq!(dock.docked_count(), 0);
        assert_eq!(dock.widgets.map(|p| p.geometry), before);
    }
    #[test]
    fn individual_docking_disarms_pair_restore() {
        let mut dock = Dock::default();
        for w in Widget::ALL { dock.detach(w, geometry(800.0)); }
        dock.toggle_pair();
        dock.detach(Widget::Files, geometry(0.0));
        dock.dock(Widget::Files);
        assert!(!dock.pair_allowed());
    }
    #[test]
    fn triple_click_does_not_fire_intermediate_folds() {
        let mut clicks = FoldClicks::default();
        assert_eq!(clicks.click(1.0, true), FoldGesture::None);
        assert_eq!(clicks.click(1.1, true), FoldGesture::None);
        assert_eq!(clicks.click(1.2, true), FoldGesture::Pair);
        assert_eq!(clicks.poll(2.0), FoldGesture::None);
        assert_eq!(clicks.click(3.0, true), FoldGesture::None);
        assert_eq!(clicks.poll(3.5), FoldGesture::Single);
        assert_eq!(clicks.click(4.0, false), FoldGesture::Single);
    }
    #[test]
    fn tiny_and_invalid_saved_layouts_stay_finite() {
        let mut dock = Dock { split: f32::NAN, ..Dock::default() };
        dock.widgets[0].geometry = Some(geometry(f32::INFINITY));
        dock.sanitize();
        assert_eq!(dock.split, 0.5);
        assert!(dock.widgets[0].geometry.is_none());
        for r in dock.regions(Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0)), 8.0, 80.0).into_iter().flatten() {
            assert!(r.height() >= 0.0);
        }
    }
}
