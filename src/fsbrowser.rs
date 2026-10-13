//! Read-only, lazy filesystem navigation projected onto the OTF arena.
//! Only requested directories are enumerated, on one background worker.
//! Category-looking names are ordinary directories in this projection.

use crate::layout::{self, Cls, Ctx, Frame, Key, Kind, Line, Span, Style};
use crate::tree::{Fold, Node, Side, Tree};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}, mpsc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind { Directory, File, Link }

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Load { Unloaded, Loading, Ready, Error(String) }

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub bytes: u64,
    pub load: Load,
}

struct Scan { generation: u64, ticket: u64, node: usize, path: PathBuf }
struct Listed { path: PathBuf, kind: EntryKind, bytes: u64 }
struct Reply { generation: u64, ticket: u64, node: usize, result: Result<Vec<Listed>, String> }

fn list(path: &Path) -> Result<Vec<Listed>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let kind = if ty.is_symlink() { EntryKind::Link } else if ty.is_dir() { EntryKind::Directory } else { EntryKind::File };
        let bytes = if kind == EntryKind::File { entry.metadata().map_or(0, |m| m.len()) } else { 0 };
        out.push(Listed { path: entry.path(), kind, bytes });
    }
    out.sort_by(|a, b| {
        let rank = |e: &Listed| (e.kind != EntryKind::Directory, e.path.file_name().unwrap_or_default().to_string_lossy().to_lowercase());
        rank(a).cmp(&rank(b)).then_with(|| a.path.cmp(&b.path))
    });
    Ok(out)
}

fn node(path: &Path, parent: Option<usize>) -> Node {
    Node {
        name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string()),
        parent, kids: Vec::new(), own_cat: None, files: Vec::new(), open: false,
        side: Side::Down, page: 0, beak: Fold::Mid,
    }
}

pub struct Browser {
    pub tree: Tree,
    pub entries: Vec<Entry>,
    pub selected: usize,
    pub generation: u64,
    pub back: Vec<PathBuf>,
    pub forward: Vec<PathBuf>,
    paths: HashMap<PathBuf, usize>,
    tx: mpsc::Sender<Scan>,
    rx: mpsc::Receiver<Reply>,
    epoch: Arc<AtomicU64>,
    ticket: u64,
    pending: HashMap<usize, u64>,
}

impl Browser {
    pub fn new(root: PathBuf, wake: impl Fn() + Send + 'static) -> Self {
        let (tx, requests) = mpsc::channel::<Scan>();
        let (replies, rx) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let current = epoch.clone();
        std::thread::Builder::new().name("octo-files".into()).spawn(move || {
            while let Ok(req) = requests.recv() {
                if current.load(Ordering::Relaxed) != req.generation { continue; }
                let result = list(&req.path);
                if current.load(Ordering::Relaxed) != req.generation { continue; }
                if replies.send(Reply { generation: req.generation, ticket: req.ticket, node: req.node, result }).is_err() { break; }
                wake();
            }
        }).expect("filesystem worker");
        let mut this = Self {
            tree: Tree { root: root.clone(), nodes: Vec::new(), cats: Vec::new() },
            entries: Vec::new(), selected: 0, generation: 0, back: Vec::new(), forward: Vec::new(),
            paths: HashMap::new(), tx, rx, epoch, ticket: 0, pending: HashMap::new(),
        };
        this.reset(root);
        this
    }

    fn reset(&mut self, path: PathBuf) {
        self.generation += 1;
        self.epoch.store(self.generation, Ordering::Relaxed);
        self.tree = Tree { root: path.clone(), nodes: vec![node(&path, None)], cats: Vec::new() };
        self.tree.nodes[0].open = true;
        self.entries = vec![Entry { path: path.clone(), kind: EntryKind::Directory, bytes: 0, load: Load::Unloaded }];
        self.paths.clear();
        self.paths.insert(path, 0);
        self.pending.clear();
        self.selected = 0;
        self.load(0);
    }

    pub fn navigate(&mut self, path: PathBuf) {
        if path == self.tree.root { return; }
        self.back.push(self.tree.root.clone());
        self.forward.clear();
        self.reset(path);
    }
    pub fn go_back(&mut self) {
        if let Some(path) = self.back.pop() {
            self.forward.push(self.tree.root.clone());
            self.reset(path);
        }
    }
    pub fn go_forward(&mut self) {
        if let Some(path) = self.forward.pop() {
            self.back.push(self.tree.root.clone());
            self.reset(path);
        }
    }
    pub fn go_up(&mut self) {
        if let Some(p) = self.tree.root.parent().filter(|p| !p.as_os_str().is_empty()) { self.navigate(p.to_path_buf()); }
    }
    pub fn loading(&self) -> bool { !self.pending.is_empty() }

    /// Refresh expanded directories after the viewer changes files. Only
    /// live descendants are visited; removed paths in the arena stay inert.
    pub fn refresh_open(&mut self) {
        let mut work = vec![0];
        let mut refresh = Vec::new();
        while let Some(id) = work.pop() {
            if self.entries[id].kind == EntryKind::Directory && self.tree.nodes[id].open {
                work.extend(self.tree.nodes[id].kids.iter().copied());
                if self.entries[id].load == Load::Ready { refresh.push(id); }
            }
        }
        for id in refresh { self.load(id); }
    }

    pub fn load(&mut self, id: usize) {
        if self.entries.get(id).is_none_or(|e| e.kind != EntryKind::Directory) { return; }
        self.ticket += 1;
        self.entries[id].load = Load::Loading;
        self.pending.insert(id, self.ticket);
        let request = Scan { generation: self.generation, ticket: self.ticket, node: id, path: self.entries[id].path.clone() };
        if self.tx.send(request).is_err() {
            self.entries[id].load = Load::Error("Folder reader is unavailable".into());
            self.pending.remove(&id);
        }
    }

    /// Apply only the newest request for the current location. Reuse node
    /// IDs by full path, preserving expanded children and animation keys.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(reply) = self.rx.try_recv() {
            if reply.generation != self.generation || self.pending.get(&reply.node) != Some(&reply.ticket) { continue; }
            self.pending.remove(&reply.node);
            changed = true;
            match reply.result {
                Err(e) => self.entries[reply.node].load = Load::Error(e),
                Ok(listing) => {
                    let mut kids = Vec::with_capacity(listing.len());
                    for item in listing {
                        let id = if let Some(&id) = self.paths.get(&item.path) {
                            // A file replaced by a directory (or vice versa)
                            // cannot keep children or its old loaded state.
                            if self.entries[id].kind != item.kind {
                                self.tree.nodes[id] = node(&item.path, Some(reply.node));
                                self.entries[id].load = Load::Unloaded;
                                self.pending.remove(&id);
                            }
                            self.entries[id].kind = item.kind;
                            self.entries[id].bytes = item.bytes;
                            id
                        } else {
                            let id = self.tree.nodes.len();
                            self.tree.nodes.push(node(&item.path, Some(reply.node)));
                            self.paths.insert(item.path.clone(), id);
                            self.entries.push(Entry { path: item.path, kind: item.kind, bytes: item.bytes, load: Load::Unloaded });
                            id
                        };
                        kids.push(id);
                    }
                    if self.selected != reply.node && self.tree.is_under(self.selected, reply.node) {
                        let mut child = self.selected;
                        while self.tree.nodes[child].parent != Some(reply.node) {
                            let Some(parent) = self.tree.nodes[child].parent else { break };
                            child = parent;
                        }
                        if !kids.contains(&child) { self.selected = reply.node; }
                    }
                    self.tree.nodes[reply.node].kids = kids;
                    self.entries[reply.node].load = Load::Ready;
                    let (_, _, page, _) = layout::arms(&self.tree, reply.node, false);
                    self.tree.nodes[reply.node].page = page;
                }
            }
        }
        changed
    }

    pub fn frame(&self, style: Style, cols: usize) -> (Frame, Ctx) {
        layout::outline(&self.tree, style, cols, &|id, cx| self.label(id, cx.names))
    }

    fn label(&self, id: usize, limit: usize) -> Line {
        let n = &self.tree.nodes[id];
        let e = &self.entries[id];
        let mut spans = Vec::new();
        let mut push = |cls, text: String| spans.push(Span { cls, text, node: Some(id) });
        if id == 0 { push(Cls::Mantle, "○ ".into()); }
        let title = if limit > 0 && n.name.chars().count() > limit {
            format!("{}..", n.name.chars().take(limit.saturating_sub(2)).collect::<String>())
        } else { n.name.clone() };
        let selected = if self.selected == id { Cls::Active } else { Cls::Name };
        match e.kind {
            EntryKind::Directory => {
                push(Cls::Brack, "[".into());
                push(selected, title);
                push(Cls::Brack, "]".into());
                match &e.load {
                    Load::Loading => push(Cls::Count, " … loading".into()),
                    Load::Error(_) => push(Cls::Ink, " ! unavailable".into()),
                    Load::Unloaded => push(Cls::Fold, " ►".into()),
                    Load::Ready if n.kids.is_empty() => push(Cls::Count, " (empty)".into()),
                    Load::Ready => push(if n.open { Cls::Count } else { Cls::Fold }, format!(" {}{}", if n.open { "" } else { "►" }, n.kids.len())),
                }
            }
            EntryKind::File => {
                push(Cls::Brack, "· ".into());
                push(selected, title);
                if limit == 0 { push(Cls::Count, format!("  {}", size_label(e.bytes))); }
            }
            EntryKind::Link => {
                push(Cls::Brack, "~ ".into());
                push(selected, title);
                push(Cls::Count, " (link)".into());
            }
        }
        Line { spans, tags: vec![id], heads: vec![id], kind: Kind::Label, key: Key::Label(id) }
    }
}

pub fn size_label(bytes: u64) -> String {
    if bytes < 1024 { format!("{bytes} B") }
    else if bytes < 1024 * 1024 { format!("{:.1} KB", bytes as f64 / 1024.0) }
    else if bytes < 1024 * 1024 * 1024 { format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)) }
    else { format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0)) }
}

/// Enumerating drive letters does not probe disks or network shares.
pub fn roots() -> Vec<PathBuf> {
    #[cfg(windows)]
    {
        let mask = unsafe { windows_sys::Win32::Storage::FileSystem::GetLogicalDrives() };
        (0..26).filter(|bit| mask & (1 << bit) != 0).map(|bit| PathBuf::from(format!("{}:\\", (b'A' + bit) as char))).collect()
    }
    #[cfg(not(windows))]
    { vec![PathBuf::from("/")] }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};
    fn ready(b: &mut Browser) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while b.loading() && Instant::now() < deadline { b.poll(); std::thread::sleep(Duration::from_millis(2)); }
        assert!(!b.loading());
    }
    fn fixture(tag: &str) -> PathBuf {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-files").join(format!("octo-files-{tag}-{}", std::process::id()));
        fs::create_dir_all(path.join("_fav_").join("nested")).unwrap();
        fs::create_dir_all(path.join("1")).unwrap();
        fs::write(path.join("notes.txt"), b"notes").unwrap();
        fs::write(path.join(".hidden"), b"hidden").unwrap();
        path
    }
    #[test]
    fn lists_all_files_and_category_names_as_directories_without_recursing() {
        let root = fixture("lazy");
        let mut b = Browser::new(root, || {});
        ready(&mut b);
        assert_eq!(b.tree.nodes.len(), 5);
        assert!(b.tree.cats.is_empty());
        assert!(b.entries.iter().skip(1).all(|e| e.load == Load::Unloaded));
        let fav = b.tree.nodes.iter().position(|n| n.name == "_fav_").unwrap();
        b.load(fav);
        ready(&mut b);
        assert_eq!(b.tree.nodes[fav].kids.len(), 1);
        assert!(b.entries.iter().any(|e| e.path.ends_with(".hidden")));
    }
    #[test]
    fn errors_can_retry_and_old_location_results_cannot_replace_new_root() {
        let root = fixture("stale");
        let mut b = Browser::new(root.clone(), || {});
        let missing = root.join("missing");
        b.navigate(missing.clone());
        ready(&mut b);
        assert!(matches!(b.entries[0].load, Load::Error(_)));
        fs::create_dir_all(&missing).unwrap();
        b.load(0);
        ready(&mut b);
        assert_eq!(b.entries[0].load, Load::Ready);
        b.go_back();
        ready(&mut b);
        assert_eq!(b.tree.root, root);
        assert!(b.tree.nodes.len() > 1);
        b.go_forward();
        ready(&mut b);
        assert_eq!(b.tree.root, missing);
    }
    #[test]
    fn refresh_keeps_expansion_and_keys_and_removes_deleted_selection() {
        let root = fixture("refresh");
        let mut b = Browser::new(root.clone(), || {});
        ready(&mut b);
        let fav = b.tree.nodes.iter().position(|n| n.name == "_fav_").unwrap();
        b.tree.nodes[fav].open = true;
        b.load(fav);
        ready(&mut b);
        let child = b.tree.nodes[fav].kids[0];
        b.load(0);
        ready(&mut b);
        assert!(b.tree.nodes[fav].open);
        assert_eq!(b.tree.nodes[fav].kids, vec![child]);
        let file = b.tree.nodes.iter().position(|n| n.name == "notes.txt").unwrap();
        b.selected = file;
        fs::remove_file(root.join("notes.txt")).unwrap();
        b.load(0);
        ready(&mut b);
        assert_eq!(b.selected, 0);
        assert!(!b.tree.nodes[0].kids.contains(&file));
    }
    #[test]
    fn every_style_pages_files_with_unique_clickable_keys() {
        let root = fixture("paging");
        for n in 0..20 { fs::write(root.join(format!("file{n:02}.txt")), b"test").unwrap(); }
        let mut b = Browser::new(root, || {});
        ready(&mut b);
        for style in [Style::Arm, Style::Beak, Style::Boxed] {
            let (frame, _) = b.frame(style, 26);
            assert!(frame.ink_row(0).is_some());
            let mut keys = std::collections::HashSet::new();
            assert!(frame.lines.iter().all(|l| keys.insert(l.key)));
            assert!(frame.lines.iter().flat_map(|l| &l.spans).any(|s| s.node.is_some()));
        }
    }
}
