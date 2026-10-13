//! Character-cell drawing. Everything on screen except the image itself is
//! laid on one monospace grid, so clicks resolve to (row, column).

use eframe::egui::text::{LayoutJob, TextFormat};
use eframe::egui::{self, Color32, FontId, Painter, Pos2, Rect, Vec2};

#[derive(Clone)]
pub struct Grid {
    pub font: FontId,
    pub cw: f32,
    pub rh: f32,
}

pub type Run = (String, Color32);

impl Grid {
    pub fn new(ctx: &egui::Context, size: f32) -> Grid {
        let font = FontId::monospace(size);
        let (cw, rh) = ctx.fonts_mut(|f| (f.glyph_width(&font, 'M'), f.row_height(&font)));
        Grid { font, cw, rh }
    }

    pub fn cols(runs: &[Run]) -> usize {
        runs.iter().map(|(t, _)| t.chars().count()).sum()
    }

    pub fn size(&self, cols: usize, rows: usize) -> Vec2 {
        Vec2::new(cols as f32 * self.cw, rows as f32 * self.rh)
    }

    /// Paint runs with their top-left at `pos`; returns the covered rect.
    pub fn line(&self, p: &Painter, pos: Pos2, runs: &[Run], alpha: f32) -> Rect {
        let mut job = LayoutJob::default();
        for (t, c) in runs {
            job.append(
                t,
                0.0,
                TextFormat {
                    font_id: self.font.clone(),
                    color: c.gamma_multiply(alpha),
                    ..Default::default()
                },
            );
        }
        let g = p.layout_job(job);
        let r = Rect::from_min_size(pos, g.size());
        p.galley(pos, g, Color32::WHITE);
        r
    }

    /// Paint one string in one colour.
    pub fn text(&self, p: &Painter, pos: Pos2, s: &str, c: Color32, alpha: f32) -> Rect {
        self.line(p, pos, &[(s.to_string(), c)], alpha)
    }

    /// Column under `x` for a line starting at `x0`.
    pub fn col(&self, x0: f32, x: f32) -> usize {
        ((x - x0) / self.cw).max(0.0) as usize
    }
}

/// Which run covers a column.
pub fn run_at<T>(runs: &[T], len: impl Fn(&T) -> usize, col: usize) -> Option<usize> {
    let mut c = 0;
    for (i, r) in runs.iter().enumerate() {
        let l = len(r);
        if col >= c && col < c + l {
            return Some(i);
        }
        c += l;
    }
    None
}
