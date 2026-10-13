//! Image focus: the view the app takes when the tree is folded. The window
//! stops being a panel the picture sits in and becomes the picture's frame.
//! It collapses round the image edge to edge, and takes each image's shape
//! as you browse, keeping its centre where it was.
//!
//! Three sizes, cycled by the window's maximize button, F11, or a
//! double-click on the top bar:
//!
//! * **Fit**: the image fills a size budget (the window as it was when the
//!   tree folded, scaled whenever you resize the window by hand) and the
//!   window wraps it;
//! * **Large**: the same, as large as the screen's work area allows;
//! * **Full**: true full screen, the image centred, with bars only where
//!   its shape differs from the screen's.
//!
//! When a dialog or menu needs more room than the picture gives it, the
//! window grows for as long as it is open. Unfolding the tree gives the
//! window back the size it had before.

use eframe::egui::{Context, Pos2, Rect, Vec2, ViewportCommand};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Fit,
    Large,
    Full,
}

/// What the view needs from the window this frame.
pub struct Want {
    /// Width over height of the picture on the stage; None leaves the
    /// window as it is (a card, a decode on its way).
    pub aspect: Option<f32>,
    /// Room the interface itself takes around the picture (pinned bars).
    pub chrome: Vec2,
    /// Room an open dialog, menu or popover needs.
    pub need: Vec2,
    /// The smallest window that still carries the interface.
    pub min: Vec2,
}

pub struct Focus {
    pub stage: Stage,
    /// The size Fit images fill.
    pub budget: Vec2,
    /// The window to give back when the tree unfolds: size, and whether it
    /// was maximized.
    pub home: Option<(Vec2, bool)>,
    /// Focus was on last frame.
    on: bool,
    /// The first activation may come from a saved session: keep the saved
    /// budget and home instead of reading them off the window.
    restored: bool,
    /// A size we asked for and when; nothing else is sent until it lands.
    sent: Option<(Vec2, f64)>,
    /// A size the platform would not give us; not asked for again.
    refused: Option<Vec2>,
    /// The window size as last agreed: a change from it that we didn't ask
    /// for is a hand resize.
    settled: Option<Vec2>,
    /// When the window last changed size by hand.
    changed_at: f64,
    /// No requests before this (the platform is entering or leaving full
    /// screen or a maximize).
    hold_until: f64,
}

/// The largest size of shape `aspect` (width / height) inside `bound`.
pub fn fit(aspect: f32, bound: Vec2) -> Vec2 {
    let w = bound.x.min(bound.y * aspect).max(1.0);
    Vec2::new(w, w / aspect)
}

/// A budget after a hand resize from `was` to `now`: scaled by the side
/// that changed most, so the budget keeps its shape.
pub fn rescale(budget: Vec2, was: Vec2, now: Vec2) -> Vec2 {
    let rx = now.x / was.x.max(1.0);
    let ry = now.y / was.y.max(1.0);
    budget
        * if (rx - 1.0).abs() >= (ry - 1.0).abs() {
            rx
        } else {
            ry
        }
}

/// `size` centred on `center`, moved (never shrunk below what fits) to sit
/// inside `work`.
pub fn place(size: Vec2, center: Pos2, work: Option<Rect>) -> Rect {
    let Some(w) = work else {
        return Rect::from_center_size(center, size);
    };
    let r = Rect::from_center_size(center, size.min(w.size()));
    let dx = if r.left() < w.left() {
        w.left() - r.left()
    } else if r.right() > w.right() {
        w.right() - r.right()
    } else {
        0.0
    };
    let dy = if r.top() < w.top() {
        w.top() - r.top()
    } else if r.bottom() > w.bottom() {
        w.bottom() - r.bottom()
    } else {
        0.0
    };
    r.translate(Vec2::new(dx, dy))
}

/// The window size for a picture of shape `aspect` in `bound`, with the
/// interface's room around it, never smaller than `need` or `min`, never
/// larger than `work`.
pub fn size_for(aspect: f32, bound: Vec2, want: &Want, work: Option<Rect>) -> Vec2 {
    let pic = fit(aspect, (bound - want.chrome).max(Vec2::splat(1.0)));
    let size = (pic + want.chrome).max(want.need).max(want.min);
    work.map_or(size, |w| size.min(w.size())).round()
}

impl Focus {
    pub fn new(budget: Option<Vec2>, home: Option<(Vec2, bool)>) -> Focus {
        Focus {
            stage: Stage::Fit,
            budget: budget.unwrap_or(Vec2::new(1440.0, 900.0)),
            home,
            on: false,
            restored: budget.is_some(),
            sent: None,
            refused: None,
            settled: None,
            changed_at: 0.0,
            hold_until: 0.0,
        }
    }

    pub fn active(&self) -> bool {
        self.on
    }

    /// One frame. `active` is whether the tree is folded with a picture to
    /// show; `work` is the screen's work area in points, if known.
    pub fn step(&mut self, ctx: &Context, active: bool, want: &Want, work: Option<Rect>, now: f64) {
        let (cur, maxed, full) = ctx.input(|i| {
            let v = i.viewport();
            (
                v.inner_rect,
                v.maximized.unwrap_or(false),
                v.fullscreen.unwrap_or(false),
            )
        });
        let size = cur.map_or_else(|| ctx.content_rect().size(), |r| r.size());
        if active && !self.on {
            self.enter(ctx, size, maxed, full, now);
        }
        if !active && self.on {
            self.leave(ctx, cur, work, full);
        }
        self.restored = false;
        self.on = active;
        if !active {
            return;
        }
        // The platform has the last word on full screen.
        if self.stage == Stage::Full && !full && now > self.hold_until {
            self.stage = Stage::Fit;
        }
        if full || maxed || now < self.hold_until {
            self.sent = None;
            self.settled = None;
            if now < self.hold_until {
                ctx.request_repaint();
            }
            return;
        }
        // Wait for what we asked for.
        if let Some((s, t)) = self.sent {
            let landed = (size - s).abs().max_elem() <= 2.0;
            if !landed && now - t < 0.6 {
                ctx.request_repaint();
                return;
            }
            if !landed {
                self.refused = Some(s);
            }
            self.sent = None;
            self.settled = Some(size);
        }
        // A hand resize scales the budget; the window settles back round
        // the picture once the resizing stops.
        match self.settled {
            Some(was) if (size - was).abs().max_elem() > 2.0 => {
                let base = match (self.stage, work) {
                    (Stage::Large, Some(w)) => w.size(),
                    _ => self.budget,
                };
                self.budget = rescale(base, was, size);
                if let Some(w) = work {
                    self.budget = self.budget.min(w.size());
                }
                self.stage = Stage::Fit;
                self.settled = Some(size);
                self.changed_at = now;
                self.refused = None;
            }
            None => self.settled = Some(size),
            _ => {}
        }
        if now - self.changed_at < 0.35 {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
            return;
        }
        let Some(aspect) = want.aspect.filter(|a| a.is_finite() && *a > 0.0) else {
            return;
        };
        let bound = match (self.stage, work) {
            (Stage::Large, Some(w)) => w.size(),
            _ => self.budget,
        };
        let target = size_for(aspect, bound, want, work);
        if (target - size).abs().max_elem() <= 1.5
            || self
                .refused
                .is_some_and(|r| (r - target).abs().max_elem() <= 1.5)
        {
            return;
        }
        if let Some(c) = cur {
            let center = match (self.stage, work) {
                (Stage::Large, Some(w)) => w.center(),
                _ => c.center(),
            };
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(
                place(target, center, work).min,
            ));
        }
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(target));
        self.sent = Some((target, now));
        self.refused = None;
        ctx.request_repaint();
    }

    fn enter(&mut self, ctx: &Context, size: Vec2, maxed: bool, full: bool, now: f64) {
        if !self.restored || self.home.is_none() {
            self.home = Some((size, maxed));
        }
        self.stage = if full {
            Stage::Full
        } else if maxed {
            // A maximized window can't take a picture's shape: let go of the
            // maximize and fill the work area instead.
            ctx.send_viewport_cmd(ViewportCommand::Maximized(false));
            self.hold_until = now + 0.3;
            Stage::Large
        } else {
            if !self.restored {
                self.budget = size;
            }
            Stage::Fit
        };
        self.sent = None;
        self.refused = None;
        self.settled = None;
        self.changed_at = 0.0;
    }

    fn leave(&mut self, ctx: &Context, cur: Option<Rect>, work: Option<Rect>, full: bool) {
        self.stage = Stage::Fit;
        self.sent = None;
        self.settled = None;
        if full {
            // The tree comes back inside full screen.
            return;
        }
        match self.home {
            Some((_, true)) => ctx.send_viewport_cmd(ViewportCommand::Maximized(true)),
            Some((size, false)) => {
                if let Some(c) = cur {
                    ctx.send_viewport_cmd(ViewportCommand::OuterPosition(
                        place(size, c.center(), work).min,
                    ));
                }
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(
                    work.map_or(size, |w| size.min(w.size())),
                ));
            }
            None => {}
        }
    }

    /// The maximize button, F11 and a double-click on the top bar while
    /// focused: Fit, Large, Full, and back to Fit.
    pub fn cycle(&mut self, ctx: &Context, now: f64) {
        let (maxed, full) = ctx.input(|i| {
            (
                i.viewport().maximized.unwrap_or(false),
                i.viewport().fullscreen.unwrap_or(false),
            )
        });
        self.stage = match (self.stage, full, maxed) {
            (_, true, _) | (Stage::Full, _, _) => {
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
                self.hold_until = now + 0.5;
                Stage::Fit
            }
            (Stage::Large, _, _) | (_, _, true) => {
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(true));
                self.hold_until = now + 0.5;
                Stage::Full
            }
            (Stage::Fit, _, _) => Stage::Large,
        };
        self.sent = None;
        self.refused = None;
        self.settled = None;
        self.changed_at = 0.0;
    }

    /// Esc in full screen: back to the window.
    pub fn leave_full(&mut self, ctx: &Context, now: f64) {
        ctx.send_viewport_cmd(ViewportCommand::Fullscreen(false));
        self.stage = Stage::Fit;
        self.hold_until = now + 0.5;
        self.sent = None;
        self.settled = None;
    }
}

/// The work area (the screen minus taskbars and docks) of the monitor the
/// window is on, in points. Exact on Windows; elsewhere estimated from the
/// monitor's size.
pub fn work_area(frame: &eframe::Frame, ctx: &Context) -> Option<Rect> {
    #[cfg(windows)]
    {
        if let Some(r) = windows_work_area(frame, ctx.pixels_per_point()) {
            return Some(r);
        }
    }
    let _ = frame;
    let (mon, cur) = ctx.input(|i| (i.viewport().monitor_size, i.viewport().inner_rect));
    let mon = mon?;
    // Assume monitors side by side from the origin, and leave room for a
    // menu bar on top and a dock or taskbar below.
    let c = cur.map_or(Pos2::ZERO, |r| r.center());
    let ox = (c.x / mon.x).floor().max(0.0) * mon.x;
    let oy = (c.y / mon.y).floor().max(0.0) * mon.y;
    Some(Rect::from_min_max(
        Pos2::new(ox, oy + 32.0),
        Pos2::new(ox + mon.x, oy + mon.y - 56.0),
    ))
}

#[cfg(windows)]
fn windows_work_area(frame: &eframe::Frame, ppp: f32) -> Option<Rect> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
    };
    let RawWindowHandle::Win32(h) = frame.window_handle().ok()?.as_raw() else {
        return None;
    };
    // SAFETY: plain Win32 queries on our own window handle; MONITORINFO is
    // a C struct whose size we pass in.
    unsafe {
        let mon = MonitorFromWindow(h.hwnd.get() as _, MONITOR_DEFAULTTONEAREST);
        if mon.is_null() {
            return None;
        }
        let mut mi: MONITORINFO = std::mem::zeroed();
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(mon, &mut mi) == 0 {
            return None;
        }
        let r = mi.rcWork;
        Some(Rect::from_min_max(
            Pos2::new(r.left as f32 / ppp, r.top as f32 / ppp),
            Pos2::new(r.right as f32 / ppp, r.bottom as f32 / ppp),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_fills_the_tighter_side() {
        let b = Vec2::new(1440.0, 900.0);
        assert_eq!(
            fit(2.0 / 3.0, b),
            Vec2::new(600.0, 900.0),
            "portrait: full height"
        );
        let l = fit(1.5, b);
        assert_eq!(l, Vec2::new(1350.0, 900.0), "3:2: full height");
        let p = fit(3.0, b);
        assert_eq!(p, Vec2::new(1440.0, 480.0), "panorama: full width");
    }

    #[test]
    fn window_wraps_the_picture_and_grows_for_dialogs() {
        let want = Want {
            aspect: None,
            chrome: Vec2::ZERO,
            need: Vec2::ZERO,
            min: Vec2::new(380.0, 200.0),
        };
        assert_eq!(
            size_for(2.0 / 3.0, Vec2::new(1440.0, 900.0), &want, None),
            Vec2::new(600.0, 900.0)
        );
        let want = Want {
            need: Vec2::new(760.0, 500.0),
            ..want
        };
        assert_eq!(
            size_for(2.0 / 3.0, Vec2::new(1440.0, 900.0), &want, None),
            Vec2::new(760.0, 900.0)
        );
        // Pinned bars: the picture keeps its shape, the bars add height.
        let want = Want {
            chrome: Vec2::new(0.0, 60.0),
            need: Vec2::ZERO,
            ..want
        };
        assert_eq!(
            size_for(1.0, Vec2::new(1000.0, 1000.0), &want, None),
            Vec2::new(940.0, 1000.0)
        );
        // Never larger than the work area.
        let work = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let want = Want {
            chrome: Vec2::ZERO,
            ..want
        };
        let s = size_for(2.0 / 3.0, Vec2::new(1440.0, 2000.0), &want, Some(work));
        assert!(s.x <= 800.0 && s.y <= 600.0, "{s:?}");
    }

    #[test]
    fn hand_resize_scales_the_budget_by_the_side_that_moved() {
        let b = Vec2::new(1440.0, 900.0);
        // A portrait window 600x900 dragged wider to 800: everything grows by 4/3.
        let nb = rescale(b, Vec2::new(600.0, 900.0), Vec2::new(800.0, 900.0));
        assert!(
            (nb.x - 1920.0).abs() < 0.01 && (nb.y - 1200.0).abs() < 0.01,
            "{nb:?}"
        );
        let nb = rescale(b, Vec2::new(600.0, 900.0), Vec2::new(600.0, 450.0));
        assert!(
            (nb.y - 450.0).abs() < 0.01 && (nb.x - 720.0).abs() < 0.01,
            "{nb:?}"
        );
    }

    #[test]
    fn place_keeps_the_centre_and_stays_on_screen() {
        let work = Rect::from_min_max(Pos2::ZERO, Pos2::new(1920.0, 1040.0));
        let r = place(Vec2::new(600.0, 900.0), Pos2::new(960.0, 520.0), Some(work));
        assert_eq!(r.center(), Pos2::new(960.0, 520.0));
        let r = place(Vec2::new(600.0, 900.0), Pos2::new(100.0, 520.0), Some(work));
        assert_eq!(r.left(), 0.0, "pushed back on screen");
        let r = place(
            Vec2::new(600.0, 1200.0),
            Pos2::new(960.0, 520.0),
            Some(work),
        );
        assert!(r.height() <= 1040.0 && r.top() >= 0.0);
    }
}
