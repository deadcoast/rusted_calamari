//! Background decode. Three workers pull paths, decode with EXIF
//! orientation applied, downscale to screen-class size and hand back pixels;
//! the UI thread turns them into textures. A request that falls out of the
//! wanted window before a worker reaches it is skipped, so holding an arrow
//! key through a thousand images never queues a thousand decodes.

use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::thread;

const MAX_SIDE: u32 = 2560;
const KEEP: usize = 48;

pub enum Slot {
    Ready {
        tex: TextureHandle,
        size: [u32; 2],
        bytes: u64,
    },
    Failed(String),
}

type Done = (PathBuf, Option<Result<(ColorImage, [u32; 2], u64), String>>);

pub struct Loader {
    tx: Sender<PathBuf>,
    rx: Receiver<Done>,
    wanted: Arc<Mutex<HashSet<PathBuf>>>,
    pending: HashSet<PathBuf>,
    /// In flight when the file changed: the decode coming back is old.
    stale: HashSet<PathBuf>,
    cache: HashMap<PathBuf, Slot>,
    lru: VecDeque<PathBuf>,
}

fn work(p: &Path) -> Result<(ColorImage, [u32; 2], u64), String> {
    let bytes = std::fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    let img = octo::ops::decode(p)?;
    let size = [img.width(), img.height()];
    let img = if size[0].max(size[1]) > MAX_SIDE {
        img.thumbnail(MAX_SIDE, MAX_SIDE)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let c = ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    );
    Ok((c, size, bytes))
}

impl Loader {
    pub fn new(ctx: egui::Context) -> Loader {
        let (tx, jobs) = channel::<PathBuf>();
        let (done, rx) = channel::<Done>();
        let jobs = Arc::new(Mutex::new(jobs));
        let wanted = Arc::new(Mutex::new(HashSet::new()));
        for _ in 0..3 {
            let (jobs, done, wanted, ctx) =
                (jobs.clone(), done.clone(), wanted.clone(), ctx.clone());
            thread::spawn(move || {
                loop {
                    let Ok(p) = jobs.lock().unwrap().recv() else {
                        return;
                    };
                    let live = wanted.lock().unwrap().contains(&p);
                    let r = live.then(|| work(&p));
                    if done.send((p, r)).is_err() {
                        return;
                    }
                    ctx.request_repaint();
                }
            });
        }
        Loader {
            tx,
            rx,
            wanted,
            pending: HashSet::new(),
            stale: HashSet::new(),
            cache: HashMap::new(),
            lru: VecDeque::new(),
        }
    }

    /// Declare the window of paths worth having, most urgent first.
    pub fn want(&mut self, paths: &[PathBuf]) {
        *self.wanted.lock().unwrap() = paths.iter().cloned().collect();
        for p in paths {
            if !self.cache.contains_key(p)
                && !self.pending.contains(p)
                && octo::tree::is_decodable(&p.to_string_lossy())
            {
                self.pending.insert(p.clone());
                let _ = self.tx.send(p.clone());
            }
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok((p, r)) = self.rx.try_recv() {
            self.pending.remove(&p);
            if self.stale.remove(&p) {
                // Asked for again by the next `want`.
                continue;
            }
            match r {
                None => {}
                Some(Ok((img, size, bytes))) => {
                    let tex = ctx.load_texture(p.to_string_lossy(), img, TextureOptions::LINEAR);
                    self.cache
                        .insert(p.clone(), Slot::Ready { tex, size, bytes });
                    self.lru.push_back(p);
                }
                Some(Err(e)) => {
                    self.cache.insert(p, Slot::Failed(e));
                }
            }
        }
        let wanted = self.wanted.lock().unwrap().clone();
        let mut guard = self.lru.len();
        while self.lru.len() > KEEP && guard > 0 {
            guard -= 1;
            let p = self.lru.pop_front().unwrap();
            if wanted.contains(&p) {
                self.lru.push_back(p);
            } else {
                self.cache.remove(&p);
            }
        }
    }

    pub fn get(&self, p: &Path) -> Option<&Slot> {
        self.cache.get(p)
    }

    pub fn texture(&self, p: &Path) -> Option<TextureHandle> {
        match self.cache.get(p) {
            Some(Slot::Ready { tex, .. }) => Some(tex.clone()),
            _ => None,
        }
    }

    /// Drop a cached decode (the file changed or moved).
    pub fn forget(&mut self, p: &Path) {
        self.cache.remove(p);
        self.lru.retain(|x| x != p);
        if self.pending.contains(p) {
            self.stale.insert(p.to_path_buf());
        }
    }
}
