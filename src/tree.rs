//! Library arena. Directories are nodes; media files are entries on their
//! directory (suckers), each carrying the category it is sorted into. The
//! category is the in-app cid; nothing about it is written to the files.

use crate::config::Cat;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const IMAGES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "ico", "tga", "qoi", "pnm"];
pub const OTHER_MEDIA: &[&str] = &["heic", "avif", "jxl", "mp4", "mov", "mkv", "webm", "avi", "m4v"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Side {
    Up,
    Down,
}

impl Side {
    pub fn flip(self) -> Side {
        match self {
            Side::Up => Side::Down,
            Side::Down => Side::Up,
        }
    }
}

/// Where a node's label pins onto its beak fan. `Up` folds the tiers
/// upward (label on the last tier row), `Down` hangs them below.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fold {
    Mid,
    Up,
    Down,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub cat: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Node {
    pub name: String,
    pub parent: Option<usize>,
    pub kids: Vec<usize>,
    /// Category this directory *is* (its name matches a category).
    pub own_cat: Option<usize>,
    /// Media files directly in this directory, sorted by name.
    pub files: Vec<FileEntry>,
    pub open: bool,
    /// Which side of its parent this arm hangs on.
    pub side: Side,
    /// Ink page when the node has more than eight arms.
    pub page: usize,
    pub beak: Fold,
}

pub struct Tree {
    pub root: PathBuf,
    pub nodes: Vec<Node>,
    pub cats: Vec<Cat>,
}

fn ext(name: &str) -> String {
    name.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default()
}

pub fn is_media(name: &str) -> bool {
    let e = ext(name);
    IMAGES.contains(&e.as_str()) || OTHER_MEDIA.contains(&e.as_str())
}

pub fn is_decodable(name: &str) -> bool {
    IMAGES.contains(&ext(name).as_str())
}

impl Tree {
    pub fn scan(root: &Path, cats: Vec<Cat>) -> io::Result<Tree> {
        let mut t = Tree { root: root.to_path_buf(), nodes: Vec::new(), cats };
        t.add(root, None, None)?;
        t.nodes[0].open = true;
        Ok(t)
    }

    fn add(&mut self, p: &Path, parent: Option<usize>, inherited: Option<usize>) -> io::Result<usize> {
        let name = p
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| p.display().to_string());
        let own_cat = self.cats.iter().position(|c| c.name == name);
        let eff = own_cat.or(inherited);
        let id = self.nodes.len();
        self.nodes.push(Node {
            name,
            parent,
            kids: Vec::new(),
            own_cat,
            files: Vec::new(),
            open: false,
            side: Side::Down,
            page: 0,
            beak: Fold::Mid,
        });
        let mut entries: Vec<_> = fs::read_dir(p)?.filter_map(Result::ok).collect();
        entries.sort_by_key(|e| e.file_name());
        let mut dirs = Vec::new();
        for e in entries {
            let n = e.file_name().to_string_lossy().into_owned();
            if n.starts_with('.') {
                continue;
            }
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_dir() {
                dirs.push(e.path());
            } else if ft.is_file() && is_media(&n) {
                self.nodes[id].files.push(FileEntry { name: n, cat: eff });
            }
        }
        let mut kids = Vec::with_capacity(dirs.len());
        for d in dirs {
            if let Ok(k) = self.add(&d, Some(id), eff) {
                kids.push(k);
            }
        }
        kids.sort_by_key(|&k| self.rank(k));
        self.nodes[id].kids = kids;
        Ok(id)
    }

    /// Categories first, by tier then name; plain directories after.
    fn rank(&self, id: usize) -> (u8, char, String) {
        let n = &self.nodes[id];
        match n.own_cat {
            Some(c) => (0, self.cats[c].tier, n.name.clone()),
            None => (1, '~', n.name.clone()),
        }
    }

    pub fn tier(&self, id: usize) -> Option<char> {
        self.nodes[id].own_cat.map(|c| self.cats[c].tier)
    }

    /// The category a directory's files belong to: its own, or the
    /// nearest category ancestor's.
    pub fn eff_cat(&self, mut id: usize) -> Option<usize> {
        loop {
            if let Some(c) = self.nodes[id].own_cat {
                return Some(c);
            }
            id = self.nodes[id].parent?;
        }
    }

    pub fn total(&self, id: usize) -> usize {
        let n = &self.nodes[id];
        n.files.len() + n.kids.iter().map(|&k| self.total(k)).sum::<usize>()
    }

    pub fn is_under(&self, mut x: usize, id: usize) -> bool {
        loop {
            if x == id {
                return true;
            }
            match self.nodes[x].parent {
                Some(p) => x = p,
                None => return false,
            }
        }
    }

    /// Path relative to the library root, `.` for the root.
    pub fn path(&self, id: usize) -> String {
        let mut parts = Vec::new();
        let mut x = Some(id);
        while let Some(i) = x {
            if self.nodes[i].parent.is_some() {
                parts.push(self.nodes[i].name.as_str());
            }
            x = self.nodes[i].parent;
        }
        parts.reverse();
        if parts.is_empty() { ".".into() } else { parts.join("/") }
    }

    /// Absolute directory path on disk.
    pub fn dir(&self, id: usize) -> PathBuf {
        let rel = self.path(id);
        if rel == "." { self.root.clone() } else { self.root.join(rel) }
    }

    pub fn child(&self, parent: usize, name: &str) -> Option<usize> {
        self.nodes[parent].kids.iter().copied().find(|&k| self.nodes[k].name == name)
    }

    /// Child node for a directory that now exists on disk.
    pub fn ensure_child(&mut self, parent: usize, name: &str) -> usize {
        if let Some(k) = self.child(parent, name) {
            return k;
        }
        let own_cat = self.cats.iter().position(|c| c.name == name);
        let id = self.nodes.len();
        let side = self.nodes[parent].kids.first().map_or(Side::Down, |&k| self.nodes[k].side);
        self.nodes.push(Node {
            name: name.to_string(),
            parent: Some(parent),
            kids: Vec::new(),
            own_cat,
            files: Vec::new(),
            open: false,
            side,
            page: 0,
            beak: Fold::Mid,
        });
        let mut kids = self.nodes[parent].kids.clone();
        kids.push(id);
        kids.sort_by_key(|&k| self.rank(k));
        self.nodes[parent].kids = kids;
        id
    }

    pub fn take_file(&mut self, node: usize, name: &str) -> Option<FileEntry> {
        let i = self.nodes[node].files.iter().position(|f| f.name == name)?;
        Some(self.nodes[node].files.remove(i))
    }

    pub fn put_file(&mut self, node: usize, name: &str) -> usize {
        let cat = self.eff_cat(node);
        let files = &mut self.nodes[node].files;
        let i = files.partition_point(|f| f.name.as_str() < name);
        files.insert(i, FileEntry { name: name.to_string(), cat });
        i
    }

    /// Re-derive category membership after the category table changed.
    pub fn recat(&mut self) {
        for i in 0..self.nodes.len() {
            let name = self.nodes[i].name.clone();
            self.nodes[i].own_cat = if self.nodes[i].parent.is_some() {
                self.cats.iter().position(|c| c.name == name)
            } else {
                None
            };
        }
        for i in 0..self.nodes.len() {
            let c = self.eff_cat(i);
            for f in &mut self.nodes[i].files {
                f.cat = c;
            }
            let mut kids = self.nodes[i].kids.clone();
            kids.sort_by_key(|&k| self.rank(k));
            self.nodes[i].kids = kids;
        }
    }

    /// Resolve `a/b/c` relative to the root. A trailing `~` targets the ink.
    pub fn find(&self, path: &str) -> Option<(usize, bool)> {
        let mut id = 0;
        let mut ink = false;
        for seg in path.split('/').filter(|s| !s.is_empty() && *s != ".") {
            if seg == "~" {
                ink = true;
                continue;
            }
            id = self.child(id, seg)?;
        }
        Some((id, ink))
    }
}
