//! Motion for the Octobrachial Tree. Every rendered line carries a stable
//! key, so between two renders the view knows which rows moved, which are
//! new and which are gone:
//!
//! * a row that moved glides to its new place;
//! * a new row grows out of the row it belongs to (an arm out of its
//!   parent's label, a beak tier out of its folder) and fades in;
//! * a row that is gone slides back into that row and fades out;
//! * scrolling eases instead of jumping, at the same rate as the rows, so
//!   a row the tide pins under the pointer stays exactly still while the
//!   tree reshapes around it.

use octo::layout::{Frame, Key, Line};
use octo::tree::Tree;
use std::collections::HashMap;

/// Approach rate (1/s). ~90% of the way in 160 ms.
const RATE: f32 = 14.0;
/// Fade-in and fade-out times (s).
const FADE_IN: f32 = 0.18;
const FADE_OUT: f32 = 0.16;

struct Row {
    /// Displayed position, in rows from the top of the whole frame.
    pos: f32,
    a: f32,
    /// Kept while the row is near the view, so it can leave as a ghost.
    line: Option<Line>,
}

pub struct Ghost {
    pub line: Line,
    pub pos: f32,
    to: f32,
    pub a: f32,
}

/// One line to draw: an index into the frame, or a ghost.
pub struct Drawn {
    pub idx: Option<usize>,
    pub ghost: Option<usize>,
    /// Rows from the top of the view (fractional while moving).
    pub y: f32,
    pub a: f32,
}

#[derive(Default)]
pub struct Motion {
    rows: HashMap<Key, Row>,
    pub ghosts: Vec<Ghost>,
    /// Displayed scroll, in rows.
    pub scroll: f32,
    t: f64,
    /// Everything has arrived; nothing needs another frame.
    pub settled: bool,
}

impl Motion {
    /// Forget everything (a new library): the next frame draws in place.
    pub fn reset(&mut self) {
        self.rows.clear();
        self.ghosts.clear();
        self.t = 0.0;
    }

    /// Where a new line starts: the displayed row of the nearest ancestor
    /// that was already on screen.
    fn origin(&self, t: &Tree, k: Key) -> Option<f32> {
        let mut n = k.anchor(t);
        while let Some(x) = n {
            if let Some(r) = self.rows.get(&Key::Label(x)) {
                return Some(r.pos);
            }
            n = t.nodes[x].parent;
        }
        None
    }

    /// Where a departing line goes: the new row of its nearest surviving
    /// ancestor.
    fn dest(t: &Tree, k: Key, index: &HashMap<Key, usize>) -> Option<f32> {
        let mut n = k.anchor(t);
        while let Some(x) = n {
            if let Some(&i) = index.get(&Key::Label(x)) {
                return Some(i as f32);
            }
            n = t.nodes[x].parent;
        }
        None
    }

    /// Advance one frame toward `f` scrolled to `scroll`, with `rows` rows
    /// in view.
    pub fn step(&mut self, t: &Tree, f: &Frame, scroll: usize, rows: usize, now: f64) {
        let fresh = self.rows.is_empty() || self.t == 0.0;
        let dt = if fresh {
            0.0
        } else {
            (now - self.t).clamp(0.0, 0.1) as f32
        };
        self.t = now;
        let k = 1.0 - (-dt * RATE).exp();
        let mut index: HashMap<Key, usize> = HashMap::with_capacity(f.lines.len());
        for (i, l) in f.lines.iter().enumerate() {
            index.insert(l.key, i);
        }

        // Births start where their ancestor is now; read before anything moves.
        let born: Vec<(Key, f32)> = f
            .lines
            .iter()
            .enumerate()
            .filter(|(_, l)| !self.rows.contains_key(&l.key))
            .map(|(i, l)| {
                (
                    l.key,
                    if fresh {
                        i as f32
                    } else {
                        self.origin(t, l.key).unwrap_or(i as f32)
                    },
                )
            })
            .collect();

        // Deaths near the view leave as ghosts.
        let (lo, hi) = (self.scroll - 2.0, self.scroll + rows as f32 + 2.0);
        let dead: Vec<Key> = self
            .rows
            .keys()
            .filter(|k| !index.contains_key(*k))
            .copied()
            .collect();
        for key in dead {
            let r = self.rows.remove(&key).unwrap();
            if let Some(line) = r.line
                && r.pos > lo
                && r.pos < hi
                && !fresh
            {
                let to = Self::dest(t, key, &index).unwrap_or(r.pos);
                self.ghosts.push(Ghost {
                    line,
                    pos: r.pos,
                    to,
                    a: r.a,
                });
            }
        }
        for (key, pos) in born {
            self.rows.insert(
                key,
                Row {
                    pos,
                    a: if fresh { 1.0 } else { 0.0 },
                    line: None,
                },
            );
        }

        let mut settled = true;
        let near = (
            scroll as f32 - rows as f32,
            scroll as f32 + 2.0 * rows as f32,
        );
        for (i, l) in f.lines.iter().enumerate() {
            let r = self.rows.get_mut(&l.key).unwrap();
            let to = i as f32;
            r.pos += (to - r.pos) * k;
            if (to - r.pos).abs() < 0.004 {
                r.pos = to;
            } else {
                settled = false;
            }
            if r.a < 1.0 {
                r.a = (r.a + dt / FADE_IN).min(1.0);
                settled = false;
            }
            r.line = (to > near.0 && to < near.1).then(|| l.clone());
        }
        for g in &mut self.ghosts {
            g.pos += (g.to - g.pos) * k;
            g.a -= dt / FADE_OUT;
        }
        self.ghosts.retain(|g| g.a > 0.0);
        settled &= self.ghosts.is_empty();

        let target = scroll as f32;
        if fresh {
            self.scroll = target;
        }
        self.scroll += (target - self.scroll) * k;
        if (target - self.scroll).abs() < 0.004 {
            self.scroll = target;
        } else {
            settled = false;
        }
        self.settled = settled;
    }

    /// Displayed row of a key, in rows from the top of the frame.
    pub fn pos(&self, k: Key) -> Option<f32> {
        self.rows.get(&k).map(|r| r.pos)
    }

    /// Lines inside the view (with a row of margin), with their displayed
    /// place and opacity; rows sliding past the top or bottom edge fade.
    pub fn visible(&self, f: &Frame, rows: usize) -> Vec<Drawn> {
        let edge =
            |y: f32, a: f32| a * (y + 1.0).clamp(0.0, 1.0) * (rows as f32 - y).clamp(0.0, 1.0);
        let mut out = Vec::new();
        for (i, l) in f.lines.iter().enumerate() {
            let Some(r) = self.rows.get(&l.key) else {
                continue;
            };
            let y = r.pos - self.scroll;
            if y > -1.0 && y < rows as f32 {
                out.push(Drawn {
                    idx: Some(i),
                    ghost: None,
                    y,
                    a: edge(y, r.a),
                });
            }
        }
        for (gi, g) in self.ghosts.iter().enumerate() {
            let y = g.pos - self.scroll;
            if y > -1.0 && y < rows as f32 {
                out.push(Drawn {
                    idx: None,
                    ghost: Some(gi),
                    y,
                    a: edge(y, g.a.max(0.0)),
                });
            }
        }
        out
    }
}
