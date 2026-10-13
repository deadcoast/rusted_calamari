//! Background writes for DANGER crop. Releasing the mouse must not wait on
//! a decode and an encode, so the write goes to one worker thread and the
//! stage shows the cut at once. One worker keeps writes to the same file in
//! the order they were made; anything else that touches files waits for the
//! queue to drain first (`App::settle`), so the disk and the undo stack
//! always agree on what happened in which order.

use eframe::egui;
use octo::ops::{self, Op};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

pub struct Done {
    pub path: PathBuf,
    /// What was written, so a failed write can give its edits back.
    pub ops: Vec<Op>,
    /// The file's bytes before the write, or why it failed.
    pub res: Result<Vec<u8>, String>,
}

pub struct Saver {
    tx: Sender<(PathBuf, Vec<Op>)>,
    rx: Receiver<Done>,
    busy: HashMap<PathBuf, usize>,
}

impl Saver {
    pub fn new(ctx: egui::Context) -> Saver {
        let (tx, jobs) = channel::<(PathBuf, Vec<Op>)>();
        let (done, rx) = channel::<Done>();
        thread::spawn(move || {
            let mut held: VecDeque<(PathBuf, Vec<Op>)> = VecDeque::new();
            loop {
                let Some((path, ops)) = held.pop_front().or_else(|| jobs.recv().ok()) else {
                    return;
                };
                // A decoder or encoder that panics fails this write, not the queue.
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    ops::save_ops(&path, &ops)
                }))
                .unwrap_or_else(|_| Err("the write failed unexpectedly".into()));
                let failed = res.is_err();
                if done
                    .send(Done {
                        path: path.clone(),
                        ops,
                        res,
                    })
                    .is_err()
                {
                    return;
                }
                if failed {
                    // Later boxes on this file were drawn on the cut that
                    // didn't land; writing them would cut the wrong place.
                    held.extend(jobs.try_iter());
                    let (same, rest): (Vec<_>, Vec<_>) =
                        held.drain(..).partition(|(p, _)| *p == path);
                    held.extend(rest);
                    for (p, ops) in same {
                        let res = Err("an earlier crop of this file didn't go through".to_string());
                        if done.send(Done { path: p, ops, res }).is_err() {
                            return;
                        }
                    }
                }
                ctx.request_repaint();
            }
        });
        Saver {
            tx,
            rx,
            busy: HashMap::new(),
        }
    }

    pub fn submit(&mut self, path: PathBuf, ops: Vec<Op>) {
        *self.busy.entry(path.clone()).or_default() += 1;
        let _ = self.tx.send((path, ops));
    }

    pub fn busy(&self, p: &Path) -> bool {
        self.busy.contains_key(p)
    }

    pub fn idle(&self) -> bool {
        self.busy.is_empty()
    }

    /// Finished writes, oldest first. With `wait`, blocks until every
    /// submitted write is finished.
    pub fn poll(&mut self, wait: bool) -> Vec<Done> {
        let mut out = Vec::new();
        loop {
            let next = if wait && !self.busy.is_empty() {
                self.rx.recv().ok()
            } else {
                self.rx.try_recv().ok()
            };
            let Some(d) = next else { break };
            if let Some(n) = self.busy.get_mut(&d.path) {
                *n -= 1;
                if *n == 0 {
                    self.busy.remove(&d.path);
                }
            }
            out.push(d);
        }
        out
    }
}
