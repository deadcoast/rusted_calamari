//! Rusted Calamari: an image browser and organizer. The image owns the
//! window. The Octobrachial Tree folds out beside it; every other control
//! stays hidden until the pointer goes looking for it.

use crate::focus::{self, Focus, Stage};
use crate::fonts::{self, FACES, SIZES};
use crate::glyph::{Grid, Run, run_at};
use crate::loader::{Loader, Slot};
use crate::saver::Saver;
use crate::theme::{
    self, DIM, EDGE, IRON, IRON_HI, Item, MUTED, PANEL, ROUND, RULE, RUST, RUST_HI, Row, SHADOW,
    STAGE, TEXT, WARN, chip, chipa, dialog, dropdown, r, ra, rd,
};
use crate::toolbar::{self, Tool};
use crate::treeview::Motion;
use crate::window::{self, Ctl};
use eframe::egui::{
    self, Area, Color32, CursorIcon, Event, Id, Key, LayerId, Order, Pos2, Rect, Sense, Stroke,
    StrokeKind, TextEdit, Vec2, ViewportCommand,
};
use egui_phosphor::bold as ph;
use image::DynamicImage;
use octo::config::{self, Cat, Rgb};
use octo::layout::{Cls, Ctx, Kind, Line, Style, render};
use octo::ops::{self, Op};
use octo::paint::colour;
use octo::tide::{self, View, balance};
use octo::tree::{IMAGES, OTHER_MEDIA, Tree, is_decodable};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[path = "workspace.rs"]
mod workspace;
#[cfg(feature = "workspace-smoke")]
#[path = "workspace_smoke.rs"]
mod workspace_smoke;
#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
use crate::browserpane::Files;
use crate::dock::{Dock, FoldClicks, FoldGesture, Widget};

const STYLES: [Style; 3] = [Style::Arm, Style::Beak, Style::Boxed];
const STYLE_NAMES: [&str; 3] = ["Octobrachial Arm", "Octobrachial Beak", "Boxtobrachial"];
const SWATCHES: [&str; 12] = [
    "#e56399", "#92CC7C", "#7f96ff", "#a6cfd5", "#E6D463", "#F0A35E", "#c792ea", "#ff8a65",
    "#4dd0e1", "#aed581", "#f06292", "#9fa8da",
];
/// Keys the app keeps; categories cannot take them.
const RESERVED: [&str; 2] = ["h", "0"];
const ASPECTS: [(&str, f32); 9] = [
    ("free", 0.0),
    ("original", -1.0),
    ("1:1", 1.0),
    ("4:3", 4.0 / 3.0),
    ("3:2", 1.5),
    ("16:9", 16.0 / 9.0),
    ("3:4", 0.75),
    ("2:3", 2.0 / 3.0),
    ("9:16", 9.0 / 16.0),
];
const SPIN: [&str; 4] = ["▖", "▘", "▝", "▗"];
/// Preview resolution for pending edits.
const PREVIEW: u32 = 2560;

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_premultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

/// A path without Windows' verbatim prefix: `canonicalize` there returns
/// `\\?\C:\...`, which reads badly in the menu bar and never equals the
/// `C:\...` paths that dialogs and drag and drop hand us.
fn plain(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    match s.strip_prefix(r"\\?\") {
        Some(rest) if rest.as_bytes().get(1) == Some(&b':') => PathBuf::from(rest),
        _ => p,
    }
}

/// A file DANGER crop can write back in place (GIF keeps its frames, so
/// it only exports).
fn writable(name: &str) -> bool {
    is_decodable(name) && !name.to_ascii_lowercase().ends_with(".gif")
}

fn rgb(c: Rgb) -> Color32 {
    Color32::from_rgb(c.0, c.1, c.2)
}

fn hexs(c: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", c.0, c.1, c.2)
}

fn run(s: impl Into<String>, c: Color32) -> Run {
    (s.into(), c)
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub font: usize,
    pub size: usize,
    pub style: usize,
    pub pin_ui: bool,
    /// Tree folded out beside the image. Folded, the app goes into image
    /// focus (see `focus`).
    pub tree: bool,
    /// The image beside the tree; folded, the tree takes the window.
    pub image: bool,
    /// Image focus: the size pictures fill, and the window to give back
    /// when the tree unfolds (size, maximized).
    pub focus_budget: Option<[f32; 2]>,
    pub home: Option<([f32; 2], bool)>,
    /// Width the user dragged the tree to; None hugs the tree.
    pub panel_w: Option<f32>,
    /// Folder and position written over the image ("japan 003").
    pub title: bool,
    /// The tool strip at the top right.
    pub toolbar: bool,
    pub library: Option<PathBuf>,
    pub dock: Dock,
    pub files_root: Option<PathBuf>,
    /// Keep Files independent of the app's current image folder.
    pub files_anchored: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            font: 0,
            size: 2,
            style: 0,
            pin_ui: false,
            tree: true,
            image: true,
            focus_budget: None,
            home: None,
            panel_w: None,
            title: false,
            toolbar: false,
            library: None,
            dock: Dock::default(),
            files_root: None,
            files_anchored: false,
        }
    }
}

struct Lib {
    tree: Tree,
    active: usize,
    cursor: usize,
}

/// Something done that can be taken back. Undo and redo share the type:
/// reversing an entry yields the entry that reverses it again, so the two
/// stacks trade entries and never drift apart.
enum Undo {
    /// The file went from `from/from_name` to `to/to_name`.
    Move {
        from: usize,
        to: usize,
        from_name: String,
        to_name: String,
    },
    /// The file went from `node/name` to the trash, as `trashed`.
    Trash {
        node: usize,
        name: String,
        trashed: PathBuf,
    },
    /// The file came back from the trash to `node/name`.
    Untrash { node: usize, name: String },
    /// `node/name` held `bytes` before it was written (a save, a DANGER crop).
    Saved {
        node: usize,
        name: String,
        bytes: Vec<u8>,
    },
    /// `op` was added to the file's pending edits.
    Op { path: PathBuf, op: Op },
    /// `op` was taken off the file's pending edits.
    Unop { path: PathBuf, op: Op },
}

/// The picture shown for a file while its new decode is on the way: after a
/// DANGER crop, the old texture cut to the kept region, so the stage never
/// blanks. `from` is the selection on screen; the picture grows from it.
struct Snap {
    tex: egui::TextureHandle,
    uv: Rect,
    px: [u32; 2],
    from: Rect,
    t0: f64,
}

struct Quick {
    path: PathBuf,
    ir: Rect,
    a: Pos2,
    b: Pos2,
}

/// Unsaved edits on one file, previewed at screen resolution.
struct Pending {
    ops: Vec<Op>,
    base: DynamicImage,
    full: [u32; 2],
    tex: egui::TextureHandle,
}

#[derive(Clone, Copy, PartialEq)]
enum Grab {
    Move,
    /// Bit 0: left edge, 1: right, 2: top, 3: bottom.
    Edge(u8),
    New,
}

struct Crop {
    path: PathBuf,
    r: [f32; 4],
    aspect: usize,
    grab: Option<(Grab, [f32; 4], Pos2)>,
}

struct Fly {
    tex: Option<egui::TextureHandle>,
    /// Part of the texture shown (all of it, or a DANGER cut).
    uv: Rect,
    from: Rect,
    cat: usize,
    t0: f64,
}

struct Flash {
    node: usize,
    cat: usize,
    t0: f64,
}

#[derive(Clone, Copy, PartialEq)]
enum Menu {
    File,
    Edit,
    View,
}

#[derive(Clone, Copy, PartialEq)]
enum Act {
    Menu(Menu),
    Settings,
    Help,
    Close,
    Open,
    ImportImages,
    Save,
    SaveAll,
    ExportAs,
    Revert,
    ImportCats,
    ExportCats,
    Manifest,
    ExportSet,
    Quit,
    Undo,
    CropTool,
    Edit(Op),
    Trash,
    Tree,
    RecallTrees,
    Widget(Widget),
    /// Fold or unfold the image (the tree takes the window).
    Image,
    Fit,
    Actual,
    Style(bool),
    Pin,
    Title,
    Font(bool),
    Size(bool),
    EditCat(usize),
    NewCat,
    FormKey,
    Swatch(bool),
    Tier(bool),
    FormSave,
    FormDelete,
    FormCancel,
    SaveAllQuit,
    DiscardQuit,
    Aspect(bool),
    CropApply,
    CropCancel,
    Throw(usize),
    CidEdit(usize),
    KeyEdit(usize),
    Redo,
    Danger,
    Toolbar,
    Window(Ctl),
    Fullscreen,
}

/// A category's key being reassigned from its keycap: the next key pressed
/// takes the slot.
struct KeyEdit {
    cat: usize,
    at: Rect,
    opened: f64,
    err: Option<(String, f64)>,
}

/// A category colour being edited in place: changes apply live; Esc puts
/// the original back.
struct CidEdit {
    cat: usize,
    at: Rect,
    orig: Rgb,
    hex: String,
    opened: f64,
}

struct Form {
    edit: Option<usize>,
    name: String,
    key: Option<String>,
    cid: String,
    tier: char,
    capturing: bool,
    err: Option<String>,
}

enum Dialog {
    None,
    Help,
    Settings,
    Form(Form),
    Unsaved,
}

#[derive(Clone, Copy)]
enum TreeAct {
    Arm(usize),
    Ink(usize),
    Throw(usize),
    Browse(usize),
}

pub struct App {
    s: Settings,
    font_on: Option<usize>,
    lib: Option<Lib>,
    loader: Loader,
    saver: Saver,
    egui: egui::Context,
    view: View,
    dialog: Dialog,
    menu: Option<Menu>,
    chrome_hover: bool,
    quiet: bool,
    zoom: f32,
    pan: Vec2,
    undo: Vec<Undo>,
    redo: Vec<Undo>,
    fly: Vec<Fly>,
    flash: Option<Flash>,
    landing: Option<(usize, f64)>,
    toast: Option<(String, Color32, f64)>,
    img_rect: Option<Rect>,
    chips: Vec<(usize, Pos2)>,
    tree_acc: f32,
    tree_dx: f32,
    tree_hold: f32,
    hold_frames: u8,
    typed: bool,
    pending: HashMap<PathBuf, Pending>,
    crop: Option<Crop>,
    /// DANGER crop is armed: a drag on the image crops and saves.
    danger: bool,
    /// The DANGER box being dragged: on which file, against which picture
    /// rect, from and to (screen).
    quick: Option<Quick>,
    /// DANGER cuts standing in for their files, one per file.
    snaps: HashMap<PathBuf, Snap>,
    cid_edit: Option<CidEdit>,
    key_edit: Option<KeyEdit>,
    /// Row motion and eased scroll for the tree.
    motion: Motion,
    /// Columns the tree pane has; the tree reflows to it (see `layout::fit`).
    tree_cols: usize,
    /// Folder the tree last followed, and the row last clicked (pulse).
    followed: Option<usize>,
    pressed: Option<(octo::layout::Key, f64)>,
    /// The subtree band under the pointer: first and last row, colour.
    band: Option<(f32, f32, Color32)>,
    tools_hover: bool,
    tools_woke: Option<f64>,
    /// The tool strip as last drawn; the toast stacks under it.
    toolbar_rect: Option<Rect>,
    /// The window is full screen (for the View menu's wording).
    win_full: bool,
    /// Image focus: the window as the picture's frame.
    focus: Focus,
    /// Room the open dialogs, menus and popovers asked for last frame.
    need: Vec2,
    /// A crop is on and the pointer is at the picture: the floating
    /// interface steps aside (see `ui`).
    yielding: bool,
    quitting: bool,
    files: Files,
    dock_drag: Option<workspace::DockDrag>,
    float_drags: [Option<workspace::FloatDrag>; 2],
    fold_clicks: FoldClicks,
    fold_action: Option<Act>,
    widget_keys: Vec<Event>,
    main_rect: Option<Rect>,
    snap_zone: Option<Rect>,
    next_library: Option<(PathBuf, Option<PathBuf>)>,
    #[cfg(feature = "workspace-smoke")]
    smoke: Option<workspace_smoke::Smoke>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, arg: Option<PathBuf>) -> App {
        let s: Settings = cc
            .storage
            .and_then(|st| eframe::get_value(st, "rusted-calamari"))
            .unwrap_or_default();
        Self::with_settings(cc.egui_ctx.clone(), s, arg)
    }

    fn with_settings(ctx: egui::Context, mut s: Settings, arg: Option<PathBuf>) -> App {
        s.dock.sanitize();
        let app_font = s.font;
        let focus_budget = s.focus_budget.map(Vec2::from);
        let focus_home = s.home.map(|(h, m)| (Vec2::from(h), m));
        // Fonts go in before the first frame: the icon families must exist
        // the first time anything draws an icon.
        fonts::install(&ctx, s.font);
        let mut v = egui::Visuals::dark();
        v.text_cursor.stroke = Stroke::new(2.0, RUST_HI);
        v.selection.bg_fill = RUST;
        v.panel_fill = STAGE;
        ctx.set_visuals(v);
        let mut app = App {
            s,
            font_on: Some(app_font),
            lib: None,
            loader: Loader::new(ctx.clone()),
            saver: Saver::new(ctx.clone()),
            egui: ctx.clone(),
            view: View {
                rows: 40,
                scroll: 0,
            },
            dialog: Dialog::None,
            menu: None,
            chrome_hover: false,
            quiet: false,
            zoom: 1.0,
            pan: Vec2::ZERO,
            undo: Vec::new(),
            redo: Vec::new(),
            fly: Vec::new(),
            flash: None,
            landing: None,
            toast: None,
            img_rect: None,
            chips: Vec::new(),
            tree_acc: 0.0,
            tree_dx: 0.0,
            tree_hold: 0.0,
            hold_frames: 3,
            typed: false,
            pending: HashMap::new(),
            crop: None,
            danger: false,
            quick: None,
            snaps: HashMap::new(),
            cid_edit: None,
            key_edit: None,
            motion: Motion::default(),
            tree_cols: 0,
            followed: None,
            pressed: None,
            band: None,
            tools_hover: false,
            tools_woke: None,
            toolbar_rect: None,
            win_full: false,
            focus: Focus::new(focus_budget, focus_home),
            need: Vec2::ZERO,
            yielding: false,
            quitting: false,
            files: Files::default(),
            dock_drag: None,
            float_drags: [None, None],
            fold_clicks: FoldClicks::default(),
            fold_action: None,
            widget_keys: Vec::new(),
            main_rect: None,
            snap_zone: None,
            next_library: None,
            #[cfg(feature = "workspace-smoke")]
            smoke: None,
        };
        let start = arg.or_else(|| app.s.library.clone()).filter(|p| p.is_dir());
        if let Some(p) = start {
            app.open_library(&p, 0.0);
        }
        // A successful unanchored library open has already positioned Files.
        // Anchored sessions keep their independently saved browser location.
        if app.files.browser.is_none() {
            let files_root = app
                .s
                .files_root
                .clone()
                .or_else(|| app.s.library.clone())
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            app.files.start(&ctx, files_root);
        }
        app
    }

    fn style(&self) -> Style {
        STYLES[self.s.style % 3]
    }

    fn say(&mut self, s: impl Into<String>, c: Color32, now: f64) {
        self.toast = Some((s.into(), c, now));
    }

    /// The viewer's context for the tree, reflowed to `cols` (0: full detail).
    fn ctx_of(lib: &Lib, cols: usize) -> Ctx {
        Ctx {
            cols,
            ..Ctx::viewer(lib.active, lib.cursor)
        }
    }

    /// Log an undoable step. Anything new forfeits what was undone.
    fn record(&mut self, u: Undo) {
        self.undo.push(u);
        self.redo.clear();
        self.files.refresh_open();
    }

    // ── library ──────────────────────────────────────────────────────

    fn open_library(&mut self, p: &Path, now: f64) -> bool {
        self.settle(true);
        let p = plain(p.canonicalize().unwrap_or_else(|_| p.to_path_buf()));
        match Tree::scan(&p, config::load(&p)) {
            Ok(mut tree) => {
                balance(&mut tree, self.style(), &Ctx::viewer(0, 0));
                self.lib = Some(Lib {
                    tree,
                    active: 0,
                    cursor: 0,
                });
                self.s.library = Some(p);
                self.view.scroll = 0;
                self.undo.clear();
                self.redo.clear();
                self.motion.reset();
                self.followed = None;
                self.snaps.clear();
                self.quick = None;
                self.cid_edit = None;
                self.key_edit = None;
                self.fly.clear();
                self.flash = None;
                self.crop = None;
                self.pending.clear();
                self.tree_hold = 0.0;
                self.reset_view();
                self.sync_files_to_folder();
                true
            }
            Err(e) => {
                self.say(format!("Can't open {}: {e}", p.display()), WARN, now);
                false
            }
        }
    }

    fn reset_view(&mut self) {
        self.zoom = 1.0;
        self.pan = Vec2::ZERO;
    }

    fn current(&self) -> Option<PathBuf> {
        let lib = self.lib.as_ref()?;
        let f = lib.tree.nodes[lib.active].files.get(lib.cursor)?;
        Some(lib.tree.dir(lib.active).join(&f.name))
    }

    fn current_cat(&self) -> Option<usize> {
        let lib = self.lib.as_ref()?;
        lib.tree.nodes[lib.active].files.get(lib.cursor)?.cat
    }

    fn current_px(&self) -> Option<[u32; 2]> {
        let p = self.current()?;
        if let Some(pd) = self.pending.get(&p) {
            return Some(ops::dims(pd.full[0], pd.full[1], &pd.ops));
        }
        if let Some(s) = self.snap_live(&p) {
            return Some(s.px);
        }
        match self.loader.get(&p) {
            Some(Slot::Ready { size, .. }) => Some(*size),
            _ => None,
        }
    }

    /// The DANGER cut stands in for `p` while its write is queued and until
    /// the loader has an answer for the new file (or a while has passed).
    /// Pending edits, which are always made on the written file, outrank it.
    fn snap_live(&self, p: &Path) -> Option<&Snap> {
        let s = self.snaps.get(p)?;
        let now = self.egui.input(|i| i.time);
        (self.saver.busy(p) || (self.loader.get(p).is_none() && now - s.t0 < 10.0)).then_some(s)
    }

    /// The file at `p` moved or changed: its cut no longer stands in for it.
    fn drop_snap(&mut self, p: &Path) {
        self.snaps.remove(p);
        if self.quick.as_ref().is_some_and(|q| q.path == p) {
            self.quick = None;
        }
    }

    fn step(&mut self, d: isize) {
        let Some(lib) = self.lib.as_mut() else { return };
        let n = lib.tree.nodes[lib.active].files.len();
        if n == 0 {
            return;
        }
        let c = (lib.cursor as isize + d).clamp(0, n as isize - 1) as usize;
        if c != lib.cursor {
            lib.cursor = c;
            self.crop = None;
            self.quick = None;
            self.reset_view();
        }
    }

    /// Throw the active image into category `cat` of the active folder.
    /// Unsaved edits are written first so they travel with the file.
    fn classify(&mut self, cat: usize, now: f64) {
        self.settle(true);
        if let Some(path) = self.current()
            && self.pending.contains_key(&path)
            && !self.save(&path, now)
        {
            return;
        }
        let from = self.img_rect;
        let Some(lib) = self.lib.as_mut() else { return };
        let a = lib.active;
        let Some(f) = lib.tree.nodes[a].files.get(lib.cursor).cloned() else {
            return;
        };
        let dir = lib.tree.dir(a);
        let src = dir.join(&f.name);
        let cname = lib.tree.cats[cat].name.clone();
        // What's on screen flies: a DANGER cut still waiting on its decode
        // flies as the cut.
        let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let shown = match (self.snap_live(&src), self.loader.texture(&src)) {
            (Some(s), _) => Some((s.tex.clone(), s.uv)),
            (None, Some(t)) => Some((t, full)),
            (None, None) => None,
        };
        let Some(lib) = self.lib.as_mut() else { return };
        match ops::move_into(&src, &dir.join(&cname), &f.name) {
            Ok(n) => {
                let (tex, uv) = shown.map_or((None, full), |(t, uv)| (Some(t), uv));
                self.loader.forget(&src);
                self.snaps.remove(&src);
                self.quick = None;
                lib.tree.take_file(a, &f.name);
                let d = lib.tree.ensure_child(a, &cname);
                lib.tree.put_file(d, &n);
                let len = lib.tree.nodes[a].files.len();
                lib.cursor = lib.cursor.min(len.saturating_sub(1));
                self.record(Undo::Move {
                    from: a,
                    to: d,
                    from_name: f.name,
                    to_name: n,
                });
                match from {
                    Some(r) => self.fly.push(Fly {
                        tex,
                        uv,
                        from: r,
                        cat,
                        t0: now,
                    }),
                    None => self.landing = Some((cat, now)),
                }
                self.flash = Some(Flash {
                    node: d,
                    cat,
                    t0: now,
                });
                self.crop = None;
                self.reset_view();
            }
            Err(e) => self.say(format!("Couldn't move {}: {e}", f.name), WARN, now),
        }
    }

    fn trash(&mut self, now: f64) {
        self.settle(true);
        let Some(path) = self.current() else { return };
        self.pending.remove(&path);
        self.crop = None;
        let lib = self.lib.as_mut().unwrap();
        let a = lib.active;
        let name = lib.tree.nodes[a].files[lib.cursor].name.clone();
        let td = ops::trash_dir(&lib.tree);
        match ops::move_into(&path, &td, &name) {
            Ok(n) => {
                self.loader.forget(&path);
                self.snaps.remove(&path);
                self.quick = None;
                lib.tree.take_file(a, &name);
                let len = lib.tree.nodes[a].files.len();
                lib.cursor = lib.cursor.min(len.saturating_sub(1));
                self.record(Undo::Trash {
                    node: a,
                    name,
                    trashed: td.join(n),
                });
            }
            Err(e) => self.say(format!("Couldn't trash {name}: {e}"), WARN, now),
        }
    }

    // ── edits ────────────────────────────────────────────────────────

    fn preview(
        ctx: &egui::Context,
        base: &DynamicImage,
        ops: &[Op],
        path: &Path,
    ) -> egui::TextureHandle {
        let img = ops::apply(base.clone(), ops).to_rgba8();
        let ci = egui::ColorImage::from_rgba_unmultiplied(
            [img.width() as usize, img.height() as usize],
            img.as_raw(),
        );
        ctx.load_texture(
            format!("edit:{}", path.display()),
            ci,
            egui::TextureOptions::LINEAR,
        )
    }

    fn push_op(&mut self, ctx: &egui::Context, op: Op, now: f64) {
        let Some(path) = self.current() else { return };
        match self.add_op(ctx, &path, op) {
            Ok(()) => {
                self.record(Undo::Op { path, op });
                self.reset_view();
            }
            Err(e) => self.say(e, WARN, now),
        }
    }

    /// Put `op` on a file's pending edits and refresh its preview.
    fn add_op(&mut self, ctx: &egui::Context, path: &Path, op: Op) -> Result<(), String> {
        self.settle(true);
        self.add_op_now(ctx, path, op)
    }

    /// `add_op` for callers that have already settled queued writes.
    fn add_op_now(&mut self, ctx: &egui::Context, path: &Path, op: Op) -> Result<(), String> {
        if !is_decodable(&path.to_string_lossy()) {
            return Err("This format can't be edited".into());
        }
        if !self.pending.contains_key(path) {
            let img = ops::decode(path).map_err(|e| format!("Can't edit: {e}"))?;
            let full = [img.width(), img.height()];
            let base = if full[0].max(full[1]) > PREVIEW {
                img.thumbnail(PREVIEW, PREVIEW)
            } else {
                img
            };
            let tex = Self::preview(ctx, &base, &[], path);
            self.pending.insert(
                path.to_path_buf(),
                Pending {
                    ops: Vec::new(),
                    base,
                    full,
                    tex,
                },
            );
        }
        let p = self.pending.get_mut(path).unwrap();
        p.ops.push(op);
        p.tex = Self::preview(ctx, &p.base, &p.ops, path);
        Ok(())
    }

    fn save(&mut self, path: &Path, now: f64) -> bool {
        self.settle(true);
        let Some(p) = self.pending.get(path) else {
            return true;
        };
        match ops::save_ops(path, &p.ops) {
            Ok(bytes) => {
                self.saved(path, bytes);
                true
            }
            Err(e) => {
                self.say(format!("Couldn't save: {e}"), WARN, now);
                false
            }
        }
    }

    /// After `path` was written over: its edits are in the file now, the
    /// old bytes go on the undo stack.
    fn saved(&mut self, path: &Path, bytes: Vec<u8>) {
        self.pending.remove(path);
        self.loader.forget(path);
        self.undo
            .retain(|u| !matches!(u, Undo::Op { path: q, .. } if q == path));
        if let Some(lib) = &self.lib {
            let name = path
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if let Some(node) = (0..lib.tree.nodes.len())
                .find(|&i| Some(lib.tree.dir(i).as_path()) == path.parent())
            {
                self.record(Undo::Saved { node, name, bytes });
            }
        }
    }

    fn save_all(&mut self, now: f64) -> bool {
        self.settle(true);
        let paths: Vec<PathBuf> = self.pending.keys().cloned().collect();
        let n = paths.len();
        let ok = paths.iter().filter(|p| self.save(p, now)).count();
        if n > 1 {
            self.say(format!("Saved {ok} of {n} images"), MUTED, now);
        }
        ok == n
    }

    fn export_as(&mut self, now: f64) {
        self.settle(true);
        let Some(path) = self.current() else { return };
        let stem = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "image".into());
        let ext = path
            .extension()
            .map(|s| s.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_else(|| "png".into());
        let Some(dest) = rfd::FileDialog::new()
            .set_title("Export as")
            .set_directory(path.parent().unwrap_or(Path::new(".")))
            .set_file_name(format!("{stem}_export.{ext}"))
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("WebP", &["webp"])
            .add_filter("BMP", &["bmp"])
            .add_filter("TIFF", &["tif", "tiff"])
            .save_file()
        else {
            return;
        };
        let ops = self
            .pending
            .get(&path)
            .map(|p| p.ops.clone())
            .unwrap_or_default();
        match ops::export_ops(&path, &ops, &dest) {
            Ok(()) => {
                // An export into the library shows up where it landed.
                if let Some(lib) = self.lib.as_mut()
                    && let Some(node) = (0..lib.tree.nodes.len())
                        .find(|&i| Some(lib.tree.dir(i).as_path()) == dest.parent())
                {
                    let name = dest
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    if octo::tree::is_media(&name)
                        && !lib.tree.nodes[node].files.iter().any(|f| f.name == name)
                    {
                        lib.tree.put_file(node, &name);
                    }
                }
                self.say(format!("Exported {}", dest.display()), MUTED, now);
            }
            Err(e) => self.say(format!("Couldn't export: {e}"), WARN, now),
        }
    }

    fn undo(&mut self, ctx: &egui::Context, now: f64) {
        self.settle(true);
        let Some(u) = self.undo.pop() else { return };
        match self.invert(ctx, u) {
            Ok((back, at)) => {
                self.redo.push(back);
                self.focus(at);
            }
            Err((e, u)) => {
                // Kept: it may be the only copy of what the file was.
                self.undo.push(u);
                self.say(format!("Couldn't undo: {e}"), WARN, now);
            }
        }
    }

    fn redo(&mut self, ctx: &egui::Context, now: f64) {
        self.settle(true);
        let Some(u) = self.redo.pop() else { return };
        // A redone move or trash leaves the folder like the first time did:
        // the next image steps up, the view stays put.
        let leaves = matches!(u, Undo::Move { .. } | Undo::Untrash { .. });
        match self.invert(ctx, u) {
            Ok((back, at)) => {
                self.undo.push(back);
                if leaves {
                    if let Some(lib) = self.lib.as_mut() {
                        let len = lib.tree.nodes[lib.active].files.len();
                        lib.cursor = lib.cursor.min(len.saturating_sub(1));
                    }
                    self.crop = None;
                    self.quick = None;
                    self.reset_view();
                } else {
                    self.focus(at);
                }
            }
            Err((e, u)) => {
                self.redo.push(u);
                self.say(format!("Couldn't redo: {e}"), WARN, now);
            }
        }
    }

    /// Show the image at (folder, index).
    fn focus(&mut self, at: Option<(usize, usize)>) {
        let (Some((node, i)), Some(lib)) = (at, self.lib.as_mut()) else {
            return;
        };
        if lib.active != node || lib.cursor != i {
            let changed_folder = lib.active != node;
            lib.active = node;
            lib.cursor = i;
            self.crop = None;
            self.quick = None;
            self.reset_view();
            if changed_folder {
                self.sync_files_to_folder();
            }
        }
    }

    /// Reverse one entry on disk and in the tree. Returns the entry that
    /// reverses this and where the file now is; on failure, the entry back,
    /// untouched, with the reason.
    #[allow(clippy::type_complexity)]
    fn invert(
        &mut self,
        ctx: &egui::Context,
        u: Undo,
    ) -> Result<(Undo, Option<(usize, usize)>), (String, Undo)> {
        match u {
            Undo::Op { path, op } => {
                if let Some(p) = self.pending.get_mut(&path) {
                    p.ops.pop();
                    if p.ops.is_empty() {
                        self.pending.remove(&path);
                    } else {
                        p.tex = Self::preview(ctx, &p.base, &p.ops, &path);
                    }
                }
                Ok((Undo::Unop { path, op }, None))
            }
            Undo::Unop { path, op } => match self.add_op_now(ctx, &path, op) {
                Ok(()) => Ok((Undo::Op { path, op }, None)),
                Err(e) => Err((e, Undo::Unop { path, op })),
            },
            other => {
                let Some(lib) = self.lib.as_mut() else {
                    return Err(("no folder is open".into(), other));
                };
                match other {
                    Undo::Move {
                        from,
                        to,
                        from_name,
                        to_name,
                    } => {
                        let src = lib.tree.dir(to).join(&to_name);
                        match ops::move_into(&src, &lib.tree.dir(from), &from_name) {
                            Ok(n) => {
                                self.pending.remove(&src);
                                self.loader.forget(&src);
                                self.snaps.remove(&src);
                                lib.tree.take_file(to, &to_name);
                                let i = lib.tree.put_file(from, &n);
                                Ok((
                                    Undo::Move {
                                        from: to,
                                        to: from,
                                        from_name: to_name,
                                        to_name: n,
                                    },
                                    Some((from, i)),
                                ))
                            }
                            Err(e) => Err((
                                e.to_string(),
                                Undo::Move {
                                    from,
                                    to,
                                    from_name,
                                    to_name,
                                },
                            )),
                        }
                    }
                    Undo::Trash {
                        node,
                        name,
                        trashed,
                    } => match ops::move_into(&trashed, &lib.tree.dir(node), &name) {
                        Ok(n) => {
                            let i = lib.tree.put_file(node, &n);
                            Ok((Undo::Untrash { node, name: n }, Some((node, i))))
                        }
                        Err(e) => Err((
                            e.to_string(),
                            Undo::Trash {
                                node,
                                name,
                                trashed,
                            },
                        )),
                    },
                    Undo::Untrash { node, name } => {
                        let path = lib.tree.dir(node).join(&name);
                        let td = ops::trash_dir(&lib.tree);
                        match ops::move_into(&path, &td, &name) {
                            Ok(n) => {
                                self.pending.remove(&path);
                                self.loader.forget(&path);
                                self.snaps.remove(&path);
                                lib.tree.take_file(node, &name);
                                Ok((
                                    Undo::Trash {
                                        node,
                                        name,
                                        trashed: td.join(n),
                                    },
                                    None,
                                ))
                            }
                            Err(e) => Err((e.to_string(), Undo::Untrash { node, name })),
                        }
                    }
                    Undo::Saved { node, name, bytes } => {
                        let p = lib.tree.dir(node).join(&name);
                        let swapped = std::fs::read(&p)
                            .and_then(|cur| ops::write_atomic(&p, &bytes).map(|_| cur));
                        match swapped {
                            Ok(cur) => {
                                self.pending.remove(&p);
                                self.loader.forget(&p);
                                self.snaps.remove(&p);
                                let i = lib.tree.nodes[node]
                                    .files
                                    .iter()
                                    .position(|f| f.name == name)
                                    .unwrap_or(0);
                                Ok((
                                    Undo::Saved {
                                        node,
                                        name,
                                        bytes: cur,
                                    },
                                    Some((node, i)),
                                ))
                            }
                            Err(e) => Err((e.to_string(), Undo::Saved { node, name, bytes })),
                        }
                    }
                    Undo::Op { .. } | Undo::Unop { .. } => unreachable!(),
                }
            }
        }
    }

    fn start_crop(&mut self) {
        if let Some(path) = self.current()
            && is_decodable(&path.to_string_lossy())
        {
            self.settle(true);
            self.danger = false;
            self.quick = None;
            self.reset_view();
            self.crop = Some(Crop {
                path,
                r: [0.0, 0.0, 1.0, 1.0],
                aspect: 0,
                grab: None,
            });
        }
    }

    /// DANGER crop: cut `r` (normalized against the picture as shown) out
    /// of the file and write it, no questions. The write runs in the
    /// background; the stage shows the cut now, grown out of `sel`.
    fn quick_crop(&mut self, path: &Path, r: [f32; 4], sel: Rect, now: f64) {
        let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        // The picture the box was drawn on: pending edits first (they are
        // always made on the file as written), then a cut, then the decode.
        let shown = if let Some(pd) = self.pending.get(path) {
            Some((
                pd.tex.clone(),
                full,
                ops::dims(pd.full[0], pd.full[1], &pd.ops),
            ))
        } else if let Some(s) = self.snap_live(path) {
            Some((s.tex.clone(), s.uv, s.px))
        } else if let Some(Slot::Ready { tex, size, .. }) = self.loader.get(path) {
            Some((tex.clone(), full, *size))
        } else {
            None
        };
        let Some((tex, uv, px)) = shown else { return };
        // Pending edits go into the same write, as a save would take them;
        // if the write fails they come back (see `settle`).
        let mut ops = self.pending.remove(path).map(|p| p.ops).unwrap_or_default();
        self.undo
            .retain(|u| !matches!(u, Undo::Op { path: q, .. } if q == path));
        ops.push(Op::Crop(r));
        self.saver.submit(path.to_path_buf(), ops);
        let cut = Rect::from_min_max(
            uv.lerp_inside(Vec2::new(r[0], r[1])),
            uv.lerp_inside(Vec2::new(r[2], r[3])),
        );
        self.snaps.insert(
            path.to_path_buf(),
            Snap {
                tex,
                uv: cut,
                px: ops::dims(px[0], px[1], &[Op::Crop(r)]),
                from: sel,
                t0: now,
            },
        );
        self.reset_view();
    }

    /// Take in finished background writes. With `wait`, first let every
    /// queued write finish: called before anything else touches files.
    fn settle(&mut self, wait: bool) {
        for d in self.saver.poll(wait) {
            match d.res {
                Ok(bytes) => self.saved(&d.path, bytes),
                Err(e) => {
                    let name = d
                        .path
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.say(format!("Couldn't crop {name}: {e}"), WARN, 0.0);
                    self.drop_snap(&d.path);
                    self.loader.forget(&d.path);
                    // Edits that rode along with the crop go back to being
                    // pending on the file, with their undo steps.
                    let ctx = self.egui.clone();
                    let keep = d.ops.len().saturating_sub(1);
                    for &op in &d.ops[..keep] {
                        if self.add_op_now(&ctx, &d.path, op).is_ok() {
                            self.undo.push(Undo::Op {
                                path: d.path.clone(),
                                op,
                            });
                        }
                    }
                }
            }
        }
    }

    // ── tree ─────────────────────────────────────────────────────────

    fn tree_act(&mut self, act: TreeAct, now: f64) {
        let style = self.style();
        let lv = self.tree_cols;
        match act {
            TreeAct::Throw(c) => self.classify(c, now),
            TreeAct::Ink(id) => {
                let lib = self.lib.as_mut().unwrap();
                let cx = Self::ctx_of(lib, lv);
                tide::click(&mut lib.tree, &mut self.view, style, &cx, id, true);
                self.tree_hold = 0.0;
            }
            TreeAct::Arm(id) | TreeAct::Browse(id) => {
                self.tree_hold = 0.0;
                self.crop = None;
                let lib = self.lib.as_mut().unwrap();
                if id == lib.active {
                    let cx = Self::ctx_of(lib, lv);
                    tide::click(&mut lib.tree, &mut self.view, style, &cx, id, false);
                    return;
                }
                // Keep the clicked row under the pointer while the slot rows move.
                let old = Self::ctx_of(lib, lv);
                let r = render(&lib.tree, style, &old)
                    .row_of(id)
                    .map(|x| x as isize - self.view.scroll as isize);
                lib.active = id;
                lib.cursor = 0;
                let cx = Self::ctx_of(lib, lv);
                if let (Some(r), Some(nr)) = (r, render(&lib.tree, style, &cx).row_of(id)) {
                    self.view.scroll = (nr as isize - r).max(0) as usize;
                }
                // The tree has already put this folder where the pointer is.
                self.followed = Some(id);
                let n = &lib.tree.nodes[id];
                let has_arms = n.kids.iter().any(|&k| lib.tree.nodes[k].own_cat.is_none());
                if !n.open && has_arms {
                    tide::click(&mut lib.tree, &mut self.view, style, &cx, id, false);
                }
                self.reset_view();
                self.sync_files_to_folder();
            }
        }
    }

    /// Width the tree wants: its widest row at full detail, held while
    /// browsing so the image doesn't twitch as sucker rows shorten.
    fn tree_width(&mut self, g: &Grid, full: Rect) -> f32 {
        if let Some(w) = self.s.panel_w {
            return w.clamp(g.cw * 24.0, (full.width() - g.cw * 24.0).max(g.cw * 24.0));
        }
        let Some(lib) = self.lib.as_ref() else {
            return (g.cw * 36.0).min(full.width() * 0.5);
        };
        let f = render(&lib.tree, self.style(), &Self::ctx_of(lib, 0));
        let want = f.width() as f32 * g.cw + g.cw * 4.0;
        if self.hold_frames > 0 {
            self.hold_frames -= 1;
            self.tree_hold = 0.0;
        }
        self.tree_hold = self.tree_hold.max(want);
        self.tree_hold.clamp(g.cw * 24.0, full.width() * 0.5)
    }

    /// What a click on run `si` of `line` does.
    fn hit(line: &Line, si: usize, secondary: bool) -> Option<TreeAct> {
        let s = &line.spans[si];
        let slot = line.spans[..=si].iter().rev().find_map(|x| match x.cls {
            Cls::CatName(c) => Some((c, x.node)),
            _ => None,
        });
        match s.cls {
            Cls::Ink => s.node.map(TreeAct::Ink),
            Cls::CatName(_) | Cls::Count | Cls::Sucker(None) | Cls::Cursor
                if line.kind == Kind::Beak && slot.is_some() =>
            {
                let (c, node) = slot.unwrap();
                if secondary {
                    node.map(TreeAct::Browse)
                } else {
                    Some(TreeAct::Throw(c))
                }
            }
            _ => s.node.map(TreeAct::Arm),
        }
    }

    /// The runs that make up the slot at run `si`: its name, count and
    /// cursor. Returns (first, last, category).
    fn slot_runs(line: &Line, si: usize) -> Option<(usize, usize, usize)> {
        let (a, c) = (0..=si).rev().find_map(|j| match line.spans[j].cls {
            Cls::CatName(c) => Some((j, c)),
            _ => None,
        })?;
        let mut b = a;
        while b + 1 < line.spans.len()
            && matches!(
                line.spans[b + 1].cls,
                Cls::Count | Cls::Sucker(None) | Cls::Cursor
            )
        {
            b += 1;
        }
        Some((a, b, c))
    }

    fn tree_pane(&mut self, ui: &mut egui::Ui, rect: Rect, x0: f32, g: &Grid, now: f64) {
        let style = self.style();
        let ctx = ui.ctx().clone();
        let Some(lib) = self.lib.as_ref() else { return };
        // Rows start below the menu bar's band and end above the status
        // line's, so the bars never sit on the root row when they appear.
        let (pad_t, pad_b) = (g.rh * 0.4, g.rh * 0.5);
        let inner = Rect::from_min_max(
            Pos2::new(x0 + g.cw * 1.5, rect.top() + pad_t),
            Pos2::new(rect.right() - g.cw, rect.bottom() - pad_b),
        );
        // The tree reshapes to the width it has instead of running off it;
        // the tide reflows to the same width, so clicks land where they show.
        let cols = (inner.width() / g.cw).floor().max(8.0) as usize;
        self.tree_cols = cols;
        let frame = render(&lib.tree, style, &Self::ctx_of(lib, cols));
        let rows = ((inner.height() / g.rh).floor() as usize).max(1);
        self.view.rows = rows;
        self.view.scroll = self.view.scroll.min(frame.lines.len().saturating_sub(1));
        // Follow the open folder when something other than a tree click
        // moved it (undo, browse): bring its row into view.
        if self.followed != Some(lib.active) {
            self.followed = Some(lib.active);
            if let Some(r) = frame.row_of(lib.active)
                && (r < self.view.scroll || r >= self.view.scroll + rows)
            {
                self.view.scroll = r.saturating_sub(rows / 3);
            }
        }

        let resp = ui.interact(rect, Id::new("otf-pane"), Sense::CLICK | Sense::DRAG);
        let slack = (frame.width() as f32 * g.cw - inner.width()).max(0.0);
        if resp.hovered() {
            self.tree_dx += ui.input(|i| i.smooth_scroll_delta.x);
            self.tree_acc += ui.input(|i| i.smooth_scroll_delta.y);
            while self.tree_acc >= g.rh {
                self.tree_acc -= g.rh;
                self.view.scroll = self.view.scroll.saturating_sub(1);
            }
            while self.tree_acc <= -g.rh {
                self.tree_acc += g.rh;
                if self.view.scroll + rows < frame.lines.len() {
                    self.view.scroll += 1;
                }
            }
        }
        self.tree_dx = (self.tree_dx + resp.drag_delta().x).clamp(-slack, 0.0);
        let inner = inner.translate(Vec2::new(self.tree_dx, 0.0));
        self.motion
            .step(&lib.tree, &frame, self.view.scroll, rows, now);
        if !self.motion.settled {
            ctx.request_repaint();
        }

        let p = ui.painter().with_clip_rect(rect);
        p.rect_filled(rect, 0.0, PANEL);
        let view = Rect::from_min_max(
            Pos2::new(rect.left(), inner.top()),
            Pos2::new(rect.right(), inner.top() + rows as f32 * g.rh),
        )
        .intersect(rect);
        let pv = p.with_clip_rect(view);
        let drawn = self.motion.visible(&frame, rows);
        let y_of = |y: f32| inner.top() + y * g.rh;
        // The line and run under a point: settled rows only.
        let under = |q: Pos2| -> Option<(usize, usize)> {
            if !view.contains(q) {
                return None;
            }
            let d = drawn
                .iter()
                .filter(|d| d.idx.is_some() && d.a > 0.5)
                .find(|d| q.y >= y_of(d.y) && q.y < y_of(d.y) + g.rh)?;
            let i = d.idx?;
            let si = run_at(
                &frame.lines[i].spans,
                |s| s.text.chars().count(),
                g.col(inner.left(), q.x),
            )?;
            Some((i, si))
        };
        let hover_line = resp
            .hover_pos()
            .filter(|q| view.contains(*q))
            .and_then(|q| {
                drawn
                    .iter()
                    .filter(|d| d.idx.is_some() && d.a > 0.5)
                    .find(|d| q.y >= y_of(d.y) && q.y < y_of(d.y) + g.rh)
                    .and_then(|d| d.idx)
            });
        let hover = resp.hover_pos().and_then(under);
        let target = hover.and_then(|(i, si)| Self::hit(&frame.lines[i], si, false));
        if target.is_some() {
            ctx.set_cursor_icon(CursorIcon::PointingHand);
        }

        // A folder under the pointer lights its whole subtree: a band in its
        // spine colour with a bar down the left, gliding from one folder to
        // the next.
        let over = match target {
            Some(TreeAct::Arm(n)) => Some(n),
            _ => None,
        };
        if let Some(n) = over {
            let (lo, hi) = frame.span(&lib.tree, n);
            if lo <= hi {
                let depth =
                    std::iter::successors(lib.tree.nodes[n].parent, |&x| lib.tree.nodes[x].parent)
                        .count();
                let c = rgb(colour(&lib.tree, Cls::Trunk(depth)).0);
                let t = self.motion.pos(frame.lines[lo].key).unwrap_or(lo as f32);
                let b = self.motion.pos(frame.lines[hi].key).unwrap_or(hi as f32) + 1.0;
                self.band = Some((t, b, c));
            }
        }
        let a_band = ctx.animate_bool_with_time(Id::new("otf-band"), over.is_some(), 0.14);
        if let Some((t, b, c)) = self.band
            && a_band > 0.01
        {
            let t = ctx.animate_value_with_time(Id::new("otf-band-t"), t, 0.1) - self.motion.scroll;
            let b = ctx.animate_value_with_time(Id::new("otf-band-b"), b, 0.1) - self.motion.scroll;
            let band = Rect::from_min_max(
                Pos2::new(rect.left(), y_of(t)),
                Pos2::new(rect.right(), y_of(b)),
            );
            pv.rect_filled(band, 0.0, c.gamma_multiply(0.07 * a_band));
            pv.rect_filled(
                Rect::from_min_max(
                    band.min + Vec2::new(1.0, 1.0),
                    Pos2::new(band.left() + 3.0, band.bottom() - 1.0),
                ),
                1.0,
                c.gamma_multiply(0.85 * a_band),
            );
        }

        for d in &drawn {
            let (line, i) = match (d.idx, d.ghost) {
                (Some(i), _) => (&frame.lines[i], Some(i)),
                (None, Some(gi)) => (&self.motion.ghosts[gi].line, None),
                _ => continue,
            };
            let y = y_of(d.y);
            let row = Rect::from_min_size(Pos2::new(rect.left(), y), Vec2::new(rect.width(), g.rh));
            if i.is_some() && i == hover_line {
                pv.rect_filled(row, 0.0, Color32::from_white_alpha(6));
            }
            if let Some((k, t0)) = self.pressed
                && k == line.key
                && !line.heads.contains(&0)
                && now - t0 < 0.4
            {
                pv.rect_filled(
                    row,
                    0.0,
                    RUST.gamma_multiply(0.35 * (1.0 - ((now - t0) / 0.4) as f32)),
                );
                ctx.request_repaint();
            }
            let hot = hover.filter(|(h, _)| Some(*h) == i).map(|(_, si)| si);
            // A slot under the pointer becomes a keycap in its own colour.
            let slot = match (hot, target) {
                (Some(si), Some(TreeAct::Throw(_) | TreeAct::Browse(_))) => {
                    Self::slot_runs(line, si)
                }
                _ => None,
            };
            let x_at = |j: usize| {
                inner.left()
                    + line.spans[..j]
                        .iter()
                        .map(|s| s.text.chars().count())
                        .sum::<usize>() as f32
                        * g.cw
            };
            if let Some((a, b, c)) = slot {
                let r = Rect::from_min_max(
                    Pos2::new(x_at(a) - 2.0, y + 1.0),
                    Pos2::new(x_at(b + 1) + 2.0, y + g.rh - 1.0),
                );
                pv.rect_filled(r, 3.0, rgb(lib.tree.cats[c].cid).gamma_multiply(d.a));
            }
            if let (Some(_), Some(TreeAct::Ink(_))) = (hot, target) {
                for (j, s) in line.spans.iter().enumerate() {
                    if s.cls == Cls::Ink {
                        let r = Rect::from_min_max(
                            Pos2::new(x_at(j) - 2.0, y + 1.0),
                            Pos2::new(x_at(j + 1) + 2.0, y + g.rh - 1.0),
                        );
                        pv.rect_filled(
                            r,
                            3.0,
                            rgb(colour(&lib.tree, Cls::Ink).0).gamma_multiply(0.25 * d.a),
                        );
                    }
                }
            }
            let mut runs: Vec<Run> = Vec::with_capacity(line.spans.len());
            let mut under_line: Option<(f32, f32, Color32)> = None;
            let mut x = inner.left();
            for (j, s) in line.spans.iter().enumerate() {
                let w = s.text.chars().count() as f32 * g.cw;
                let mut col = rgb(colour(&lib.tree, s.cls).0);
                if slot.is_some_and(|(a, b, _)| j >= a && j <= b) {
                    col = STAGE;
                }
                if let Cls::CatName(cat) = s.cls
                    && let Some(f) = &self.flash
                    && s.node == Some(f.node)
                    && cat == f.cat
                    && now - f.t0 < 0.8
                {
                    let k = 1.0 - ((now - f.t0) / 0.8) as f32;
                    pv.rect_filled(
                        Rect::from_min_size(Pos2::new(x - 1.0, y), Vec2::new(w + 2.0, g.rh)),
                        0.0,
                        col.gamma_multiply(k),
                    );
                    if k > 0.45 {
                        col = STAGE;
                    }
                }
                // The folder under the pointer: its [name] underlined.
                if hot.is_some()
                    && over.is_some()
                    && s.node == over
                    && matches!(s.cls, Cls::Brack | Cls::Name | Cls::Active)
                {
                    let u = under_line.get_or_insert((x, x, col));
                    u.1 = x + w;
                }
                runs.push((s.text.clone(), col));
                if matches!(s.cls, Cls::BeakTrunk(_)) {
                    // Thicken the single horizontal rule by one physical
                    // pixel without changing the CP437 cell or hit width.
                    g.text(
                        &pv,
                        Pos2::new(x, y + 1.0 / ctx.pixels_per_point()),
                        &s.text,
                        col,
                        d.a,
                    );
                }
                x += w;
            }
            g.line(&pv, Pos2::new(inner.left(), y), &runs, d.a);
            if let Some((x0, x1, c)) = under_line {
                let uy = (y + g.rh - 2.0).round() + 0.5;
                pv.line_segment(
                    [Pos2::new(x0, uy), Pos2::new(x1, uy)],
                    Stroke::new(1.0, c.gamma_multiply(0.8 * d.a)),
                );
            }
        }
        let above = self.view.scroll;
        let below = frame.lines.len().saturating_sub(self.view.scroll + rows);
        if above > 0 {
            let t = format!("▲ {above}");
            g.text(
                &p,
                Pos2::new(
                    rect.right() - (t.chars().count() as f32 + 2.0) * g.cw,
                    rect.top() + g.rh * 0.2,
                ),
                &t,
                DIM,
                1.0,
            );
        }
        if below > 0 {
            let t = format!("▼ {below}");
            g.text(
                &p,
                Pos2::new(
                    rect.right() - (t.chars().count() as f32 + 2.0) * g.cw,
                    rect.bottom() - g.rh * 1.2,
                ),
                &t,
                DIM,
                1.0,
            );
        }

        let mut act = None;
        if (resp.clicked() || resp.secondary_clicked())
            && let Some(q) = resp.interact_pointer_pos()
            && let Some((i, si)) = under(q)
        {
            act = Self::hit(&frame.lines[i], si, resp.secondary_clicked());
            if act.is_some() {
                self.pressed = Some((frame.lines[i].key, now));
            }
        }
        if let Some(a) = act {
            self.tree_act(a, now);
        }
    }

    // ── stage ────────────────────────────────────────────────────────

    fn stage(&mut self, ui: &mut egui::Ui, rect: Rect, g: &Grid, now: f64, focused: bool) {
        self.img_rect = None;
        let Some(lib) = self.lib.as_ref() else { return };
        let node = &lib.tree.nodes[lib.active];
        let p = ui.painter().with_clip_rect(rect);
        let Some(f) = node.files.get(lib.cursor) else {
            let first = format!("No images in {}", node.name);
            let hint = if self.s.tree {
                "Pick another folder in the tree"
            } else {
                "Press Tab to open the tree"
            };
            theme::card(
                &p,
                g,
                rect.center(),
                &[(first.as_str(), TEXT), (hint, MUTED)],
            );
            return;
        };
        let name = f.name.clone();
        let path = lib.tree.dir(lib.active).join(&name);
        let cropping = self.crop.as_ref().is_some_and(|c| c.path == path);
        let danger = self.danger && !cropping && writable(&name);
        let resp = ui.interact(rect, Id::new("stage"), Sense::CLICK | Sense::DRAG);
        let full_uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

        // Pending edits (always made on the file as written), then a DANGER
        // cut waiting on its decode, then the decode.
        let live = self.snap_live(&path).map(|s| (s.tex.clone(), s.uv, s.px));
        let (tex, uv, orig) = match (self.pending.get(&path), live, self.loader.get(&path)) {
            (Some(pd), _, _) => (
                pd.tex.clone(),
                full_uv,
                ops::dims(pd.full[0], pd.full[1], &pd.ops),
            ),
            (None, Some(s), _) => s,
            (None, None, Some(Slot::Ready { tex, size, .. })) => (tex.clone(), full_uv, *size),
            (None, None, Some(Slot::Failed(e))) => {
                let e = e.clone();
                theme::card(
                    &p,
                    g,
                    rect.center(),
                    &[
                        (name.as_str(), TEXT),
                        ("Can't decode this file", WARN),
                        (e.as_str(), MUTED),
                    ],
                );
                self.img_rect = Some(Rect::from_center_size(rect.center(), g.size(24, 5)));
                return;
            }
            (None, None, None) if is_decodable(&name) => {
                let s = SPIN[((now * 8.0) as usize) % 4];
                g.text(
                    &p,
                    rect.center() - Vec2::new(g.cw * 0.5, g.rh * 0.5),
                    s,
                    MUTED,
                    1.0,
                );
                ui.ctx().request_repaint();
                return;
            }
            (None, None, None) => {
                theme::card(
                    &p,
                    g,
                    rect.center(),
                    &[
                        (name.as_str(), TEXT),
                        ("No preview for this format", MUTED),
                        ("It can still go in a category", MUTED),
                    ],
                );
                self.img_rect = Some(Rect::from_center_size(rect.center(), g.size(32, 5)));
                return;
            }
        };
        // Texels of the region shown (all of it, or a DANGER cut of it).
        let ts = tex.size_vec2() * uv.size();
        let fit = (rect.width() / ts.x).min(rect.height() / ts.y);
        if !cropping {
            if resp.double_clicked() && !danger {
                self.reset_view();
            }
            // With DANGER crop armed the left button cuts; the others pan.
            let pans = if danger {
                resp.dragged_by(egui::PointerButton::Secondary)
                    || resp.dragged_by(egui::PointerButton::Middle)
            } else {
                resp.dragged()
            };
            if pans {
                self.pan += resp.drag_delta();
                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
            }
            if resp.hovered() {
                let dy = ui.input(|i| i.smooth_scroll_delta.y);
                if dy != 0.0
                    && let Some(ptr) = resp.hover_pos()
                {
                    let old = self.zoom;
                    self.zoom = (self.zoom * (1.0 + dy * 0.0025)).clamp(0.1, 32.0);
                    let c = rect.center() + self.pan;
                    self.pan += (ptr - c) * (1.0 - self.zoom / old);
                }
            }
        }
        let mut r = Rect::from_center_size(rect.center() + self.pan, ts * fit * self.zoom);
        // After a DANGER crop the picture grows out of the selection into
        // its new frame, then a ring marks it written.
        let grow = self.snaps.get(&path).map(|s| (s.from, (now - s.t0) as f32));
        if let Some((from, age)) = grow
            && age < 0.2
        {
            let e = 1.0 - (1.0 - age / 0.2).powi(3);
            r = Rect::from_min_max(from.min.lerp(r.min, e), from.max.lerp(r.max, e));
            ui.ctx().request_repaint();
        }
        // Lift the picture off the stage; in focus it is the window, edge to
        // edge, with nothing to lift it off.
        if !focused {
            p.rect_filled(r.translate(Vec2::new(0.0, 5.0)).expand(3.0), 3.0, SHADOW);
        }
        p.image(tex.id(), r, uv, Color32::WHITE);
        self.img_rect = Some(r);
        if let Some((_, age)) = grow
            && (0.12..0.75).contains(&age)
        {
            let k = (age - 0.12) / 0.63;
            p.rect_stroke(
                r.expand(2.0 + 6.0 * k),
                2.0,
                Stroke::new(2.0 * (1.0 - k), WARN.gamma_multiply(1.0 - k)),
                StrokeKind::Outside,
            );
            ui.ctx().request_repaint();
        }
        // The image's cid, held in the app: a rule under the picture, or in
        // focus, where there is no room under it, a frame just inside it.
        if let Some(c) = self.current_cat()
            && let Some(cat) = self.lib.as_ref().and_then(|l| l.tree.cats.get(c))
        {
            if focused {
                p.rect_stroke(r, 0.0, Stroke::new(2.0, rgb(cat.cid)), StrokeKind::Inside);
            } else {
                let bar = Rect::from_min_size(
                    Pos2::new(r.left(), r.bottom() + 6.0),
                    Vec2::new(r.width(), 3.0),
                );
                p.rect_filled(bar, 0.0, rgb(cat.cid));
            }
        }
        if self.s.title && !cropping {
            self.title_overlay(&p, g, r);
        }
        if cropping {
            self.crop_overlay(ui, &p, &resp, r, orig, g);
        }
        if danger {
            self.danger_overlay(ui, &p, &resp, r, orig, &path, g, now);
        }
    }

    /// DANGER crop on the stage: press, drag a box, let go. Shift keeps it
    /// square. The box is red because letting go writes the file.
    #[allow(clippy::too_many_arguments)]
    fn danger_overlay(
        &mut self,
        ui: &egui::Ui,
        p: &egui::Painter,
        resp: &egui::Response,
        ir: Rect,
        px: [u32; 2],
        path: &Path,
        g: &Grid,
        now: f64,
    ) {
        let ctx = ui.ctx().clone();
        if resp.hovered()
            && resp
                .hover_pos()
                .is_some_and(|q| ir.expand(12.0).contains(q))
        {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
        }
        // Not while the last cut is still growing into place: the picture
        // under the pointer is moving.
        let settling = self.snaps.get(path).is_some_and(|s| now - s.t0 < 0.2);
        if resp.drag_started_by(egui::PointerButton::Primary)
            && !settling
            && let Some(o) = ctx
                .input(|i| i.pointer.press_origin())
                .or(resp.interact_pointer_pos())
        {
            let a = o.clamp(ir.min, ir.max);
            self.quick = Some(Quick {
                path: path.to_path_buf(),
                ir,
                a,
                b: a,
            });
        }
        if let Some(a) = self.quick.as_ref().filter(|k| k.path == path).map(|k| k.a)
            && resp.dragged_by(egui::PointerButton::Primary)
            && let Some(q) = resp.interact_pointer_pos()
        {
            let mut b = q.clamp(ir.min, ir.max);
            if ctx.input(|i| i.modifiers.shift) {
                let d = q - a;
                let room = Vec2::new(
                    if d.x >= 0.0 {
                        ir.right() - a.x
                    } else {
                        a.x - ir.left()
                    },
                    if d.y >= 0.0 {
                        ir.bottom() - a.y
                    } else {
                        a.y - ir.top()
                    },
                );
                let side = d.x.abs().min(d.y.abs()).min(room.x).min(room.y);
                b = a + Vec2::new(side * d.x.signum(), side * d.y.signum());
            }
            if let Some(k) = self.quick.as_mut() {
                k.b = b;
            }
        }
        let norm = |s: Rect| {
            [
                ((s.left() - ir.left()) / ir.width()).clamp(0.0, 1.0),
                ((s.top() - ir.top()) / ir.height()).clamp(0.0, 1.0),
                ((s.right() - ir.left()) / ir.width()).clamp(0.0, 1.0),
                ((s.bottom() - ir.top()) / ir.height()).clamp(0.0, 1.0),
            ]
        };
        if resp.drag_stopped_by(egui::PointerButton::Primary)
            && let Some(k) = self.quick.take()
        {
            let s = Rect::from_two_pos(k.a, k.b);
            // Only onto the picture the box was drawn on, unmoved; and a
            // click or a twitch is not a crop.
            let same = k.path == path
                && k.ir.min.distance(ir.min) < 0.5
                && k.ir.max.distance(ir.max) < 0.5;
            if same && s.width() >= 6.0 && s.height() >= 6.0 {
                self.quick_crop(path, norm(s), s, now);
            }
            return;
        }
        let Some((a, b)) = self
            .quick
            .as_ref()
            .filter(|k| k.path == path)
            .map(|k| (k.a, k.b))
        else {
            // Armed: a red hairline round the picture.
            p.rect_stroke(
                ir.expand(1.0),
                0.0,
                Stroke::new(1.0, WARN.gamma_multiply(0.6)),
                StrokeKind::Outside,
            );
            return;
        };
        let s = Rect::from_two_pos(a, b);
        let veil = Color32::from_black_alpha(160);
        p.rect_filled(
            Rect::from_min_max(ir.min, Pos2::new(ir.right(), s.top())),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(Pos2::new(ir.left(), s.bottom()), ir.max),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(ir.left(), s.top()),
                Pos2::new(s.left(), s.bottom()),
            ),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(s.right(), s.top()),
                Pos2::new(ir.right(), s.bottom()),
            ),
            0.0,
            veil,
        );
        p.rect_stroke(s, 0.0, Stroke::new(1.5, WARN), StrokeKind::Outside);
        let l = 12.0_f32.min(s.width() / 3.0).min(s.height() / 3.0);
        let thick = Stroke::new(3.0, WARN);
        for (cx, cy, sx, sy) in [
            (s.left(), s.top(), 1.0, 1.0),
            (s.right(), s.top(), -1.0, 1.0),
            (s.left(), s.bottom(), 1.0, -1.0),
            (s.right(), s.bottom(), -1.0, -1.0),
        ] {
            p.line_segment([Pos2::new(cx, cy), Pos2::new(cx + sx * l, cy)], thick);
            p.line_segment([Pos2::new(cx, cy), Pos2::new(cx, cy + sy * l)], thick);
        }
        let d = ops::dims(px[0], px[1], &[Op::Crop(norm(s))]);
        let label = format!(" {} x {} ", d[0], d[1]);
        let lp = Pos2::new(s.left(), (s.top() - g.rh - 4.0).max(ir.top()));
        p.rect_filled(
            Rect::from_min_size(lp, g.size(label.chars().count(), 1)),
            2.0,
            WARN,
        );
        g.text(p, lp, &label, STAGE, 1.0);
    }

    /// "japan 003": the folder and the image's place in it, set large in the
    /// interface face on the image's lower left, with a hard drop shadow.
    fn title_overlay(&self, p: &egui::Painter, g: &Grid, r: Rect) {
        let Some(lib) = self.lib.as_ref() else { return };
        let n = &lib.tree.nodes[lib.active];
        let digits = n.files.len().to_string().len().max(3);
        let text = format!("{} {:0digits$}", n.name, lib.cursor + 1);
        let size = (g.font.size * 2.4).min(r.height() * 0.12).max(g.font.size);
        let font = egui::FontId::monospace(size);
        let pos = Pos2::new(r.left() + size * 1.2, r.bottom() - size * 1.6);
        let pos = pos.max(r.min);
        p.text(
            pos + Vec2::new(size * 0.08, size * 0.08),
            egui::Align2::LEFT_TOP,
            &text,
            font.clone(),
            Color32::from_black_alpha(170),
        );
        p.text(
            pos,
            egui::Align2::LEFT_TOP,
            &text,
            font,
            Color32::from_rgb(0xf2, 0xee, 0xe4),
        );
    }

    fn crop_overlay(
        &mut self,
        ui: &egui::Ui,
        p: &egui::Painter,
        resp: &egui::Response,
        ir: Rect,
        px: [u32; 2],
        g: &Grid,
    ) {
        let Some(c) = self.crop.as_mut() else { return };
        let to_screen = |n: [f32; 4]| {
            Rect::from_min_max(
                ir.lerp_inside(Vec2::new(n[0], n[1])),
                ir.lerp_inside(Vec2::new(n[2], n[3])),
            )
        };
        let to_norm = |q: Pos2| {
            [
                ((q.x - ir.left()) / ir.width()).clamp(0.0, 1.0),
                ((q.y - ir.top()) / ir.height()).clamp(0.0, 1.0),
            ]
        };
        let ratio = match ASPECTS[c.aspect].1 {
            x if x < 0.0 => Some(px[0] as f32 / px[1] as f32),
            0.0 => None,
            x => Some(x),
        };
        let hit = |s: Rect, q: Pos2| -> Grab {
            let t = 10.0;
            let mut m = 0u8;
            if (q.x - s.left()).abs() < t && q.y > s.top() - t && q.y < s.bottom() + t {
                m |= 1
            }
            if (q.x - s.right()).abs() < t && q.y > s.top() - t && q.y < s.bottom() + t {
                m |= 2
            }
            if (q.y - s.top()).abs() < t && q.x > s.left() - t && q.x < s.right() + t {
                m |= 4
            }
            if (q.y - s.bottom()).abs() < t && q.x > s.left() - t && q.x < s.right() + t {
                m |= 8
            }
            if m != 0 {
                Grab::Edge(m)
            } else if s.contains(q) {
                Grab::Move
            } else {
                Grab::New
            }
        };
        let s = to_screen(c.r);
        if let Some(h) = resp.hover_pos()
            && c.grab.is_none()
        {
            ui.ctx().set_cursor_icon(match hit(s, h) {
                Grab::Move => CursorIcon::Move,
                Grab::Edge(m) if m & 3 != 0 && m & 12 != 0 => {
                    if (m & 1 != 0) == (m & 4 != 0) {
                        CursorIcon::ResizeNwSe
                    } else {
                        CursorIcon::ResizeNeSw
                    }
                }
                Grab::Edge(m) if m & 3 != 0 => CursorIcon::ResizeHorizontal,
                Grab::Edge(_) => CursorIcon::ResizeVertical,
                Grab::New => CursorIcon::Crosshair,
            });
        }
        // Hit-test where the button went down, not where the drag was
        // recognised a few points later: an edge grabbed at the picture's
        // border stays that edge.
        if resp.drag_started()
            && let Some(q) = ui
                .ctx()
                .input(|i| i.pointer.press_origin())
                .or(resp.interact_pointer_pos())
        {
            c.grab = Some((hit(s, q), c.r, q));
        }
        if resp.dragged()
            && let (Some((grab, r0, q0)), Some(q)) = (c.grab, resp.interact_pointer_pos())
        {
            let [nx, ny] = to_norm(q);
            let mut r = r0;
            match grab {
                Grab::Move => {
                    let (w, h) = (r0[2] - r0[0], r0[3] - r0[1]);
                    r[0] = (r0[0] + (q.x - q0.x) / ir.width()).clamp(0.0, 1.0 - w);
                    r[1] = (r0[1] + (q.y - q0.y) / ir.height()).clamp(0.0, 1.0 - h);
                    r[2] = r[0] + w;
                    r[3] = r[1] + h;
                }
                Grab::New => {
                    let [ax, ay] = to_norm(q0);
                    r = [ax.min(nx), ay.min(ny), ax.max(nx), ay.max(ny)];
                }
                Grab::Edge(m) => {
                    if m & 1 != 0 {
                        r[0] = nx.min(r[2] - 0.01)
                    }
                    if m & 2 != 0 {
                        r[2] = nx.max(r[0] + 0.01)
                    }
                    if m & 4 != 0 {
                        r[1] = ny.min(r[3] - 0.01)
                    }
                    if m & 8 != 0 {
                        r[3] = ny.max(r[1] + 0.01)
                    }
                }
            }
            if let Some(a) = ratio
                && grab != Grab::Move
            {
                // Hold the ratio in pixels: height follows width.
                let w_px = (r[2] - r[0]) * px[0] as f32;
                let mut h_n = w_px / a / px[1] as f32;
                let from_bottom = matches!(grab, Grab::Edge(m) if m & 4 != 0)
                    || (grab == Grab::New && ny < to_norm(q0)[1]);
                if from_bottom {
                    h_n = h_n.min(r[3]);
                    r[1] = r[3] - h_n;
                } else {
                    h_n = h_n.min(1.0 - r[1]);
                    r[3] = r[1] + h_n;
                }
                let w_n = h_n * a * px[1] as f32 / px[0] as f32;
                if matches!(grab, Grab::Edge(m) if m & 1 != 0) {
                    r[0] = r[2] - w_n
                } else {
                    r[2] = r[0] + w_n
                }
            }
            c.r = r;
        }
        if resp.drag_stopped() {
            c.grab = None;
        }

        // Dim outside, thirds inside, rust corners, pixel size above.
        let s = to_screen(c.r);
        let veil = Color32::from_black_alpha(165);
        p.rect_filled(
            Rect::from_min_max(ir.min, Pos2::new(ir.right(), s.top())),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(Pos2::new(ir.left(), s.bottom()), ir.max),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(ir.left(), s.top()),
                Pos2::new(s.left(), s.bottom()),
            ),
            0.0,
            veil,
        );
        p.rect_filled(
            Rect::from_min_max(
                Pos2::new(s.right(), s.top()),
                Pos2::new(ir.right(), s.bottom()),
            ),
            0.0,
            veil,
        );
        let thin = Stroke::new(1.0, Color32::from_white_alpha(55));
        for k in [1.0 / 3.0, 2.0 / 3.0] {
            let x = s.left() + s.width() * k;
            let y = s.top() + s.height() * k;
            p.line_segment([Pos2::new(x, s.top()), Pos2::new(x, s.bottom())], thin);
            p.line_segment([Pos2::new(s.left(), y), Pos2::new(s.right(), y)], thin);
        }
        p.rect_stroke(s, 0.0, Stroke::new(1.0, RUST_HI), StrokeKind::Outside);
        let l = 14.0_f32.min(s.width() / 3.0).min(s.height() / 3.0);
        let thick = Stroke::new(3.0, RUST_HI);
        for (cx, cy, sx, sy) in [
            (s.left(), s.top(), 1.0, 1.0),
            (s.right(), s.top(), -1.0, 1.0),
            (s.left(), s.bottom(), 1.0, -1.0),
            (s.right(), s.bottom(), -1.0, -1.0),
        ] {
            p.line_segment([Pos2::new(cx, cy), Pos2::new(cx + sx * l, cy)], thick);
            p.line_segment([Pos2::new(cx, cy), Pos2::new(cx, cy + sy * l)], thick);
        }
        let d = ops::dims(px[0], px[1], &[Op::Crop(c.r)]);
        let label = format!(" {} x {} ", d[0], d[1]);
        let lp = Pos2::new(s.left(), (s.top() - g.rh - 4.0).max(ir.top()));
        p.rect_filled(
            Rect::from_min_size(lp, g.size(label.chars().count(), 1)),
            0.0,
            RUST,
        );
        g.text(p, lp, &label, STAGE, 1.0);
    }

    fn want(&mut self) {
        let Some(lib) = self.lib.as_ref() else { return };
        let files = &lib.tree.nodes[lib.active].files;
        if files.is_empty() {
            self.loader.want(&[]);
            return;
        }
        let dir = lib.tree.dir(lib.active);
        let c = lib.cursor as isize;
        let mut v = Vec::new();
        for d in [0isize, 1, -1, 2, 3, -2, 5, -5, 25, 50] {
            let i = c + d;
            if i >= 0 && (i as usize) < files.len() {
                v.push(dir.join(&files[i as usize].name));
            }
        }
        self.loader.want(&v);
    }

    // ── input ────────────────────────────────────────────────────────

    fn key_name(k: Key) -> Option<String> {
        if k == Key::Space {
            return Some("spc".into());
        }
        let n = k.name();
        let mut ch = n.chars();
        match (ch.next(), ch.next()) {
            (Some(c), None) if c.is_ascii_alphanumeric() => {
                Some(c.to_ascii_lowercase().to_string())
            }
            _ => None,
        }
    }

    fn keys(&mut self, ctx: &egui::Context, now: f64) {
        if self.s.tree && !self.s.dock.floating(Widget::Files) {
            self.files.keys(ctx);
        }
        let typing = self.typed || ctx.memory(|m| m.focused().is_some());
        let mut events = ctx.input(|i| i.events.clone());
        events.append(&mut self.widget_keys);
        for ev in events {
            let Event::Key {
                key,
                pressed: true,
                repeat,
                modifiers,
                ..
            } = ev
            else {
                continue;
            };
            // The key popover takes the next key, whatever it is.
            if let Some(ke) = self.key_edit.as_mut() {
                match key {
                    Key::Escape => self.key_edit = None,
                    Key::Backspace | Key::Delete => {
                        let c = ke.cat;
                        self.key_edit = None;
                        self.set_key(c, None, now);
                    }
                    _ if modifiers.command || modifiers.alt => {
                        ke.err = Some(("Plain keys only, no Ctrl or Alt".into(), now))
                    }
                    _ => match Self::key_name(key) {
                        None => ke.err = Some(("Use a letter, a digit or space".into(), now)),
                        Some(n) if RESERVED.contains(&n.as_str()) => {
                            ke.err = Some((format!("{n} belongs to the app"), now))
                        }
                        Some(n) => {
                            let c = ke.cat;
                            self.key_edit = None;
                            self.set_key(c, Some(n), now);
                        }
                    },
                }
                continue;
            }
            if self.cid_edit.is_some() {
                match key {
                    Key::Escape => self.close_cid(false),
                    Key::Enter => self.close_cid(true),
                    _ => {}
                }
                continue;
            }
            if let Dialog::Form(f) = &mut self.dialog
                && f.capturing
            {
                f.capturing = false;
                if key == Key::Escape {
                    continue;
                }
                let Some(n) = Self::key_name(key) else {
                    f.err = Some("Use a letter, a digit or space".into());
                    continue;
                };
                let taken =
                    self.lib.as_ref().and_then(|l| {
                        l.tree.cats.iter().enumerate().find(|(i, c)| {
                            Some(*i) != f.edit && c.key.as_deref() == Some(n.as_str())
                        })
                    });
                if RESERVED.contains(&n.as_str()) {
                    f.err = Some(format!("{n} is used by the app"));
                } else if let Some((_, c)) = taken {
                    f.err = Some(format!("{n} is already {}'s key", c.name));
                } else {
                    f.key = Some(n);
                    f.err = None;
                }
                continue;
            }
            if typing {
                if key == Key::Escape {
                    ctx.memory_mut(|m| {
                        if let Some(id) = m.focused() {
                            m.surrender_focus(id);
                        }
                    });
                }
                continue;
            }
            if !self.s.pin_ui {
                self.quiet = true;
            }
            // Esc first lets go of a DANGER box, then disarms the tool.
            if self.danger
                && key == Key::Escape
                && self.menu.is_none()
                && matches!(self.dialog, Dialog::None)
            {
                if self.quick.take().is_none() {
                    self.danger = false;
                }
                continue;
            }
            // The crop tool owns the keyboard while it is up.
            if self.crop.is_some() {
                match key {
                    Key::Enter => self.act(ctx, Act::CropApply, now),
                    Key::Escape => self.crop = None,
                    Key::A => self.act(ctx, Act::Aspect(!modifiers.shift), now),
                    _ => {}
                }
                continue;
            }
            let cmd = modifiers.command;
            let sh = modifiers.shift;
            let steps = |k: Key| match k {
                Key::ArrowLeft => Some(-1),
                Key::ArrowRight => Some(1),
                Key::ArrowUp => Some(-5),
                Key::ArrowDown => Some(5),
                Key::OpenBracket => Some(25),
                Key::CloseBracket => Some(-25),
                Key::Semicolon => Some(50),
                Key::Quote => Some(-50),
                _ => None,
            };
            if let Some(d) = steps(key) {
                self.menu = None;
                self.step(d);
                continue;
            }
            let a = match key {
                Key::Escape if self.menu.is_some() => {
                    self.menu = None;
                    None
                }
                Key::Escape if !matches!(self.dialog, Dialog::None) => Some(Act::Close),
                // Nothing else to close: Esc leaves full screen.
                Key::Escape if self.win_full => {
                    if self.focus.active() {
                        self.focus.leave_full(ctx, now);
                    } else {
                        window::toggle_fullscreen(ctx);
                    }
                    None
                }
                Key::Escape => None,
                Key::F if modifiers.alt => Some(Act::Menu(Menu::File)),
                Key::E if modifiers.alt => Some(Act::Menu(Menu::Edit)),
                Key::V if modifiers.alt => Some(Act::Menu(Menu::View)),
                Key::H if !cmd => Some(Act::Help),
                Key::Period => Some(Act::Settings),
                Key::Tab if sh => Some(Act::Image),
                Key::Tab => Some(Act::Tree),
                Key::Num0 if !cmd => Some(Act::Fit),
                Key::Num1 if cmd => Some(Act::Actual),
                Key::F11 => Some(Act::Fullscreen),
                Key::O if cmd => Some(Act::Open),
                Key::I if cmd => Some(Act::ImportImages),
                Key::S if cmd && sh => Some(Act::ExportAs),
                Key::S if cmd => Some(Act::Save),
                Key::Q if cmd => Some(Act::Quit),
                Key::Z if cmd && sh => Some(Act::Redo),
                Key::Y if cmd => Some(Act::Redo),
                Key::Z if cmd => Some(Act::Undo),
                Key::K if cmd && sh => Some(Act::Danger),
                Key::K if cmd => Some(Act::CropTool),
                Key::R if cmd => Some(Act::Edit(if sh { Op::RotCcw } else { Op::RotCw })),
                Key::F if cmd => Some(Act::Edit(if sh { Op::FlipV } else { Op::FlipH })),
                Key::Backspace => Some(Act::Undo),
                Key::Delete => Some(Act::Trash),
                _ if !repeat && !cmd && !modifiers.alt => Self::key_name(key).and_then(|n| {
                    self.lib
                        .as_ref()
                        .and_then(|l| {
                            l.tree
                                .cats
                                .iter()
                                .position(|c| c.key.as_deref() == Some(n.as_str()))
                        })
                        .map(Act::Throw)
                }),
                _ => None,
            };
            if let Some(a) = a {
                self.act(ctx, a, now);
            }
        }
    }

    fn drops(&mut self, ctx: &egui::Context, now: f64) {
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .filter(|p| !p.as_os_str().is_empty())
                .collect()
        });
        if dropped.is_empty() {
            return;
        }
        if let Some(d) = dropped.iter().find(|p| p.is_dir()) {
            let d = d.clone();
            self.request_library(d, None, now);
            return;
        }
        self.import_files(&dropped, now);
    }

    fn import_files(&mut self, files: &[PathBuf], now: f64) {
        self.settle(true);
        let Some(lib) = self.lib.as_mut() else {
            self.say("Open a folder first", WARN, now);
            return;
        };
        let dir = lib.tree.dir(lib.active);
        let mut n = 0;
        for f in files {
            let name = f
                .file_name()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if octo::tree::is_media(&name)
                && let Ok(nm) = ops::copy_into(f, &dir)
            {
                lib.tree.put_file(lib.active, &nm);
                n += 1;
            }
        }
        let into = lib.tree.nodes[lib.active].name.clone();
        self.say(format!("Imported {n} into {into}"), MUTED, now);
    }

    // ── actions ──────────────────────────────────────────────────────

    fn act(&mut self, ctx: &egui::Context, a: Act, now: f64) {
        if !matches!(a, Act::Menu(_)) {
            self.menu = None;
        }
        match a {
            Act::Menu(m) => self.menu = if self.menu == Some(m) { None } else { Some(m) },
            Act::Settings => {
                self.dialog = if matches!(self.dialog, Dialog::Settings) {
                    Dialog::None
                } else {
                    Dialog::Settings
                }
            }
            Act::Help => {
                self.dialog = if matches!(self.dialog, Dialog::Help) {
                    Dialog::None
                } else {
                    Dialog::Help
                }
            }
            Act::Close => {
                self.next_library = None;
                self.dialog = match self.dialog {
                    Dialog::Form(_) => Dialog::Settings,
                    _ => Dialog::None,
                }
            }
            Act::Open => {
                if let Some(d) = rfd::FileDialog::new()
                    .set_title("Open folder")
                    .pick_folder()
                {
                    self.request_library(d, None, now);
                }
            }
            Act::ImportImages => {
                let ex: Vec<&str> = IMAGES.iter().chain(OTHER_MEDIA).copied().collect();
                if let Some(fs) = rfd::FileDialog::new()
                    .set_title("Import images")
                    .add_filter("Images and video", &ex)
                    .pick_files()
                {
                    self.import_files(&fs, now);
                }
            }
            Act::Save => {
                if let Some(p) = self.current() {
                    self.save(&p, now);
                }
            }
            Act::SaveAll => {
                self.save_all(now);
            }
            Act::ExportAs => self.export_as(now),
            Act::Revert => {
                self.settle(true);
                if let Some(p) = self.current() {
                    self.pending.remove(&p);
                    self.undo
                        .retain(|u| !matches!(u, Undo::Op { path, .. } if *path == p));
                    self.redo.clear();
                }
            }
            Act::ImportCats => self.import_categories(now),
            Act::ExportCats | Act::Manifest | Act::ExportSet => self.export(a, now),
            Act::Quit => ctx.send_viewport_cmd(ViewportCommand::Close),
            Act::Undo => self.undo(ctx, now),
            Act::CropTool => {
                if self.crop.is_some() {
                    self.crop = None;
                } else {
                    self.start_crop();
                }
            }
            Act::Edit(op) => self.push_op(ctx, op, now),
            Act::Trash => self.trash(now),
            // Tree and image fold in turn; one of them always stays.
            Act::Tree => {
                if self.s.dock.docked_count() == 0 {
                    return;
                }
                if self.s.image {
                    self.s.tree = !self.s.tree;
                } else {
                    self.s.image = true;
                    self.s.tree = true;
                }
            }
            Act::RecallTrees => {
                if self.s.dock.toggle_pair() {
                    self.s.tree = true;
                }
            }
            Act::Widget(w) => self.toggle_widget(w),
            Act::Image => {
                if self.s.dock.docked_count() == 0 {
                    return;
                }
                if self.s.tree {
                    self.s.image = !self.s.image;
                } else {
                    self.s.tree = true;
                    self.s.image = true;
                }
            }
            Act::Fit => self.reset_view(),
            Act::Actual => {
                if let (Some(r), Some(f)) = (self.img_rect, self.current_px()) {
                    self.zoom *= f[0] as f32 / r.width();
                    self.pan = Vec2::ZERO;
                }
            }
            Act::Style(fwd) => {
                self.s.style = if fwd {
                    (self.s.style + 1) % 3
                } else {
                    (self.s.style + 2) % 3
                }
            }
            Act::Pin => self.s.pin_ui = !self.s.pin_ui,
            Act::Title => self.s.title = !self.s.title,
            Act::Font(fwd) => {
                self.s.font = if fwd {
                    (self.s.font + 1) % 12
                } else {
                    (self.s.font + 11) % 12
                }
            }
            Act::Size(fwd) => {
                let n = SIZES.len();
                self.s.size = if fwd {
                    (self.s.size + 1) % n
                } else {
                    (self.s.size + n - 1) % n
                };
                self.hold_frames = 3;
            }
            Act::EditCat(i) => {
                if let Some(c) = self.lib.as_ref().and_then(|l| l.tree.cats.get(i)).cloned() {
                    self.dialog = Dialog::Form(Form {
                        edit: Some(i),
                        name: c.name,
                        key: c.key,
                        cid: hexs(c.cid),
                        tier: c.tier,
                        capturing: false,
                        err: None,
                    });
                }
            }
            Act::NewCat => {
                let used: Vec<String> = self
                    .lib
                    .as_ref()
                    .map(|l| {
                        l.tree
                            .cats
                            .iter()
                            .map(|c| hexs(c.cid).to_lowercase())
                            .collect()
                    })
                    .unwrap_or_default();
                let cid = SWATCHES
                    .iter()
                    .find(|s| !used.contains(&s.to_lowercase()))
                    .unwrap_or(&SWATCHES[0]);
                self.dialog = Dialog::Form(Form {
                    edit: None,
                    name: String::new(),
                    key: None,
                    cid: cid.to_string(),
                    tier: 'c',
                    capturing: false,
                    err: None,
                });
            }
            Act::FormKey => {
                if let Dialog::Form(f) = &mut self.dialog {
                    f.capturing = true;
                    f.err = None;
                }
                ctx.memory_mut(|m| {
                    if let Some(id) = m.focused() {
                        m.surrender_focus(id);
                    }
                });
            }
            Act::Swatch(fwd) => {
                if let Dialog::Form(f) = &mut self.dialog {
                    let n = SWATCHES.len();
                    let i = SWATCHES
                        .iter()
                        .position(|s| s.eq_ignore_ascii_case(&f.cid))
                        .unwrap_or(0);
                    f.cid = SWATCHES[if fwd { (i + 1) % n } else { (i + n - 1) % n }].to_string();
                }
            }
            Act::Tier(fwd) => {
                if let Dialog::Form(f) = &mut self.dialog {
                    let t = f.tier as u8;
                    f.tier = if fwd {
                        t.saturating_add(1).min(b'z') as char
                    } else {
                        t.saturating_sub(1).max(b'a') as char
                    };
                }
            }
            Act::FormSave => self.save_form(),
            Act::FormDelete => {
                if let Dialog::Form(f) = &self.dialog
                    && let Some(i) = f.edit
                    && let Some(lib) = self.lib.as_mut()
                {
                    lib.tree.cats.remove(i);
                    lib.tree.recat();
                    let _ = config::save(&lib.tree.root, &lib.tree.cats);
                    self.flash = None;
                    self.fly.clear();
                    self.motion.reset();
                }
                self.dialog = Dialog::Settings;
            }
            Act::FormCancel => self.dialog = Dialog::Settings,
            Act::SaveAllQuit => {
                if self.save_all(now) {
                    self.finish_unsaved(ctx, now);
                }
            }
            Act::DiscardQuit => {
                self.pending.clear();
                self.finish_unsaved(ctx, now);
            }
            Act::Aspect(fwd) => {
                let px = self.current_px();
                if let Some(c) = self.crop.as_mut() {
                    let n = ASPECTS.len();
                    c.aspect = if fwd {
                        (c.aspect + 1) % n
                    } else {
                        (c.aspect + n - 1) % n
                    };
                    c.r = [0.0, 0.0, 1.0, 1.0];
                    let a = ASPECTS[c.aspect].1;
                    if let Some(px) = px
                        && a > 0.0
                    {
                        // Largest centred rect of that ratio.
                        let img = px[0] as f32 / px[1] as f32;
                        if a > img {
                            let h = img / a;
                            c.r = [0.0, (1.0 - h) / 2.0, 1.0, (1.0 + h) / 2.0];
                        } else {
                            let w = a / img;
                            c.r = [(1.0 - w) / 2.0, 0.0, (1.0 + w) / 2.0, 1.0];
                        }
                    }
                }
            }
            Act::CropApply => {
                if let Some(c) = self.crop.take()
                    && c.r != [0.0, 0.0, 1.0, 1.0]
                {
                    self.push_op(ctx, Op::Crop(c.r), now);
                }
            }
            Act::CropCancel => self.crop = None,
            Act::Throw(c) => self.classify(c, now),
            Act::CidEdit(_) | Act::KeyEdit(_) => {}
            Act::Redo => self.redo(ctx, now),
            Act::Danger => {
                self.danger = !self.danger;
                self.quick = None;
                if self.danger {
                    self.crop = None;
                }
            }
            Act::Toolbar => {
                self.s.toolbar = !self.s.toolbar;
                self.tools_woke = self.s.toolbar.then_some(now);
            }
            // In image focus the maximize button and F11 step through the
            // focus sizes instead.
            Act::Window(Ctl::Maximize) | Act::Fullscreen if self.focus.active() => {
                self.focus.cycle(ctx, now)
            }
            Act::Window(c) => window::apply(ctx, c),
            Act::Fullscreen => window::toggle_fullscreen(ctx),
        }
        if matches!(a, Act::Undo | Act::Redo | Act::ImportImages) {
            self.files.refresh_open();
        }
    }

    /// Give category `cat` the key `k` (None unbinds it). A key another
    /// category holds is swapped, never duplicated. Writes `.calamari`.
    fn set_key(&mut self, cat: usize, k: Option<String>, now: f64) {
        let Some(lib) = self.lib.as_mut() else { return };
        let old = lib.tree.cats[cat].key.clone();
        if old == k {
            return;
        }
        let mut note = None;
        if let Some(k) = &k
            && let Some(j) = lib
                .tree
                .cats
                .iter()
                .position(|c| c.key.as_deref() == Some(k.as_str()))
            && j != cat
        {
            lib.tree.cats[j].key = old.clone();
            let (a, b) = (
                lib.tree.cats[cat].name.clone(),
                lib.tree.cats[j].name.clone(),
            );
            note = Some(match &old {
                Some(o) => format!("{a} took {k}; {b} has {o} now"),
                None => format!("{a} took {k}; {b} has no key now"),
            });
        }
        lib.tree.cats[cat].key = k;
        match config::save(&lib.tree.root, &lib.tree.cats) {
            Err(e) => self.say(
                format!("Kept for this session; couldn't write .calamari: {e}"),
                WARN,
                now,
            ),
            Ok(()) => {
                if let Some(n) = note {
                    self.say(n, MUTED, now);
                }
            }
        }
    }

    fn import_categories(&mut self, now: f64) {
        let Some(lib) = self.lib.as_mut() else { return };
        let Some(f) = rfd::FileDialog::new()
            .set_title("Import categories")
            .pick_file()
        else {
            return;
        };
        let cats = std::fs::read_to_string(&f)
            .map(|t| config::parse(&t))
            .unwrap_or_default();
        if cats.is_empty() {
            self.say("That file has no categories in it", WARN, now);
            return;
        }
        lib.tree.cats = cats;
        lib.tree.recat();
        let _ = config::save(&lib.tree.root, &lib.tree.cats);
        self.flash = None;
        self.fly.clear();
        self.motion.reset();
    }

    fn export(&mut self, what: Act, now: f64) {
        self.settle(true);
        let Some(lib) = self.lib.as_ref() else { return };
        let r = match what {
            Act::Manifest => rfd::FileDialog::new()
                .set_file_name("calamari_manifest.csv")
                .save_file()
                .map(|p| {
                    std::fs::write(&p, ops::manifest(&lib.tree))
                        .map(|_| format!("Exported manifest to {}", p.display()))
                }),
            Act::ExportCats => rfd::FileDialog::new()
                .set_file_name("categories.calamari")
                .save_file()
                .map(|p| {
                    std::fs::write(&p, config::to_text(&lib.tree.cats))
                        .map(|_| format!("Exported categories to {}", p.display()))
                }),
            _ => rfd::FileDialog::new()
                .set_title("Export categorised images")
                .pick_folder()
                .map(|d| {
                    ops::export_set(&lib.tree, &d)
                        .map(|n| format!("Copied {n} categorised images to {}", d.display()))
                }),
        };
        match r {
            Some(Ok(m)) => self.say(m, MUTED, now),
            Some(Err(e)) => self.say(format!("Couldn't export: {e}"), WARN, now),
            None => {}
        }
    }

    fn save_form(&mut self) {
        let Dialog::Form(f) = &mut self.dialog else {
            return;
        };
        let name = f.name.trim().to_string();
        if name.is_empty()
            || name.contains(|c: char| c.is_whitespace() || "/\\:*?\"<>|".contains(c))
        {
            f.err = Some("Name it like a folder, without spaces".into());
            return;
        }
        let Some(cid) = config::hex(&f.cid) else {
            f.err = Some("Colour must be #RRGGBB".into());
            return;
        };
        let Some(lib) = self.lib.as_mut() else {
            f.err = Some("Open a folder first".into());
            return;
        };
        if lib
            .tree
            .cats
            .iter()
            .enumerate()
            .any(|(i, c)| Some(i) != f.edit && c.name == name)
        {
            f.err = Some(format!("{name} already exists"));
            return;
        }
        let cat = Cat {
            key: f.key.clone(),
            name,
            cid,
            tier: f.tier,
        };
        match f.edit {
            Some(i) => lib.tree.cats[i] = cat,
            None => lib.tree.cats.push(cat),
        }
        lib.tree.recat();
        self.motion.reset();
        if let Err(e) = config::save(&lib.tree.root, &lib.tree.cats) {
            f.err = Some(format!(
                "Kept for this session; couldn't write .calamari: {e}"
            ));
            return;
        }
        self.dialog = Dialog::Settings;
    }

    // ── chrome ───────────────────────────────────────────────────────

    /// The menu bar: logo, menus with their Alt letter lit, and on the
    /// right anything unsaved, the library path and the window buttons. It
    /// is also the window's title bar: drag its empty part to move the
    /// window, double-click it to maximize. In a narrow window (image focus
    /// round a portrait picture) the path goes first, then the name beside
    /// the logo, then Help and Settings (`H`, `.`), never File, Edit, View.
    fn menubar(&mut self, ctx: &egui::Context, g: &Grid, full: Rect, alpha: f32) -> Option<Act> {
        let h = g.rh * 1.5;
        let slide = (1.0 - alpha) * h * 0.5;
        if alpha < 0.01 {
            return None;
        }
        let mut act = None;
        let mut hovered = false;
        let mut spots: Vec<(f32, f32, Act)> = Vec::new();
        let titles: [(&str, Act, bool); 5] = [
            ("File", Act::Menu(Menu::File), self.menu == Some(Menu::File)),
            ("Edit", Act::Menu(Menu::Edit), self.menu == Some(Menu::Edit)),
            ("View", Act::Menu(Menu::View), self.menu == Some(Menu::View)),
            (
                "Settings",
                Act::Settings,
                matches!(self.dialog, Dialog::Settings | Dialog::Form(_)),
            ),
            ("Help", Act::Help, matches!(self.dialog, Dialog::Help)),
        ];
        let mut right: Vec<Run> = Vec::new();
        if !self.pending.is_empty() {
            right.push(run(format!("{} unsaved", self.pending.len()), RUST_HI));
            right.push(run("   ", DIM));
        }
        if let Some(l) = &self.lib {
            right.push(run(l.tree.root.display().to_string(), DIM));
        }
        let ctl_w = (h * 1.6).round();
        let room = full.width() - ctl_w * 3.0 - g.cw * 2.0;
        let cols = |logo: usize, n: usize| {
            (1 + logo
                + titles[..n]
                    .iter()
                    .map(|t| t.0.chars().count() + 2)
                    .sum::<usize>()) as f32
                * g.cw
        };
        let logo = if cols(20, 5) <= room { 20 } else { 3 };
        let mut shown = 5;
        while shown > 3 && cols(logo, shown) > room {
            shown -= 1;
        }
        let max_glyph = if self.focus.active() {
            match self.focus.stage {
                Stage::Fit => ph::CORNERS_OUT,
                Stage::Large => ph::FRAME_CORNERS,
                Stage::Full => ph::CORNERS_IN,
            }
        } else {
            window::max_glyph(ctx)
        };
        let live = !self.yielding
            || self.s.pin_ui
            || self.menu.is_some()
            || !matches!(self.dialog, Dialog::None);
        Area::new(Id::new("menubar"))
            .fixed_pos(full.min - Vec2::new(0.0, slide))
            .order(Order::Foreground)
            .interactable(live && alpha > 0.05)
            .show(ctx, |ui| {
                let (rect, resp) =
                    ui.allocate_exact_size(Vec2::new(full.width(), h), Sense::CLICK | Sense::DRAG);
                let p = ui.painter();
                p.rect_filled(rect, 0.0, IRON.gamma_multiply(alpha));
                p.line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(1.0, EDGE.gamma_multiply(alpha)),
                );
                let (ctl, span) = window::controls(ui, rect, ctl_w, alpha, max_glyph);
                if let Some(c) = ctl {
                    act = Some(Act::Window(c));
                }
                let p = ui.painter();
                let y = rect.top() + (h - g.rh) * 0.5;
                let mut x = rect.left() + g.cw;
                if logo == 20 {
                    g.line(
                        p,
                        Pos2::new(x, y),
                        &[("◙ ".into(), RUST_HI), ("Rusted Calamari".into(), TEXT)],
                        alpha,
                    );
                } else {
                    g.text(p, Pos2::new(x, y), "◙", RUST_HI, alpha);
                }
                x += g.cw * logo as f32;
                let hover = resp.hover_pos();
                for &(t, a, open) in &titles[..shown] {
                    let w = (t.chars().count() + 2) as f32 * g.cw;
                    let cell = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, h));
                    let hot = hover.is_some_and(|q| cell.contains(q));
                    if open || hot {
                        p.rect_filled(
                            cell,
                            0.0,
                            (if open { RUST } else { IRON_HI }).gamma_multiply(alpha),
                        );
                    }
                    let first: String = t.chars().take(1).collect();
                    let rest: String = t.chars().skip(1).collect();
                    let menu = matches!(a, Act::Menu(_));
                    // The Alt letter is lit on the three Alt menus.
                    let lit = match (open, menu) {
                        (true, _) => STAGE,
                        (false, true) => RUST_HI,
                        (false, false) => TEXT,
                    };
                    g.line(
                        p,
                        Pos2::new(x + g.cw, y),
                        &[(first, lit), (rest, if open { STAGE } else { TEXT })],
                        alpha,
                    );
                    spots.push((x, x + w, a));
                    x += w;
                }
                let rw = Grid::cols(&right) as f32 * g.cw;
                let rx = (span.left() - rw - g.cw * 2.0).max(x + g.cw * 3.0);
                let clip = Rect::from_min_max(
                    Pos2::new(x + g.cw, rect.top()),
                    Pos2::new(span.left() - g.cw, rect.bottom()),
                );
                // The path only where it has room to say something.
                if clip.width() > g.cw * 12.0 {
                    g.line(&p.with_clip_rect(clip), Pos2::new(rx, y), &right, alpha);
                }
                let on_spot = |q: Pos2| {
                    spots.iter().any(|(a, b, _)| q.x >= *a && q.x < *b) || span.contains(q)
                };
                if resp.clicked()
                    && let Some(q) = resp.interact_pointer_pos()
                    && !span.contains(q)
                {
                    act = act.or(spots
                        .iter()
                        .find(|(a, b, _)| q.x >= *a && q.x < *b)
                        .map(|s| s.2));
                }
                // The empty bar is the title bar.
                if resp.drag_started_by(egui::PointerButton::Primary)
                    && ctx
                        .input(|i| i.pointer.press_origin())
                        .is_some_and(|q| !on_spot(q))
                {
                    ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                }
                if resp.double_clicked() && resp.interact_pointer_pos().is_some_and(|q| !on_spot(q))
                {
                    act = Some(Act::Window(Ctl::Maximize));
                }
                hovered = resp.contains_pointer();
            });
        self.chrome_hover |= hovered;
        if let Some(m) = self.menu {
            let x = spots
                .iter()
                .find(|s| s.2 == Act::Menu(m))
                .map_or(full.left(), |s| s.0);
            let items = self.menu_items(m);
            let o = dropdown(
                ctx,
                g,
                "dropdown",
                Pos2::new(x, full.top() + h - slide),
                &items,
                alpha,
            );
            self.chrome_hover |= o.hovered;
            if o.click.is_some() {
                return o.click;
            }
        }
        act
    }

    /// The tool strip's tools. One entry per tool; the strip lays itself
    /// out from this list.
    fn tools(&self) -> Vec<Tool<Act>> {
        let img = self
            .current()
            .is_some_and(|p| is_decodable(&p.to_string_lossy()));
        let write = self
            .current()
            .is_some_and(|p| writable(&p.to_string_lossy()));
        vec![
            Tool {
                id: "crop",
                icon: ph::CROP,
                badge: None,
                name: "Crop",
                keys: "Ctrl+K",
                note: "frame it, pick a ratio, Enter applies",
                group: 0,
                act: Act::CropTool,
                on: self.crop.is_some(),
                enabled: img,
                tone: RUST,
            },
            Tool {
                id: "danger-crop",
                icon: ph::CROP,
                badge: Some(ph::LIGHTNING),
                name: "DANGER crop",
                keys: "Ctrl+Shift+K",
                note: "drag a box on the image: it crops and saves",
                group: 0,
                act: Act::Danger,
                on: self.danger,
                enabled: write,
                tone: WARN,
            },
        ]
    }

    fn menu_items(&self, m: Menu) -> Vec<Item<Act>> {
        let has_img = self.current().is_some();
        let dirty = self
            .current()
            .is_some_and(|p| self.pending.contains_key(&p));
        let lib = self.lib.is_some();
        let it = |label: &str, key: &str, act: Act, on: bool| Item::Act {
            label: label.into(),
            key: key.into(),
            act,
            on,
        };
        match m {
            Menu::File => vec![
                it("Open folder...", "Ctrl+O", Act::Open, true),
                it("Import images...", "Ctrl+I", Act::ImportImages, lib),
                Item::Rule,
                it("Save", "Ctrl+S", Act::Save, dirty),
                it("Save all", "", Act::SaveAll, !self.pending.is_empty()),
                it("Export as...", "Ctrl+Shift+S", Act::ExportAs, has_img),
                it("Revert edits", "", Act::Revert, dirty),
                Item::Rule,
                it("Import categories...", "", Act::ImportCats, lib),
                it("Export categories...", "", Act::ExportCats, lib),
                it("Export manifest...", "", Act::Manifest, lib),
                it("Export categorised images...", "", Act::ExportSet, lib),
                Item::Rule,
                it("Quit", "Ctrl+Q", Act::Quit, true),
            ],
            Menu::Edit => vec![
                it(
                    "Undo",
                    "Ctrl+Z",
                    Act::Undo,
                    !self.undo.is_empty() || !self.saver.idle(),
                ),
                it("Redo", "Ctrl+Shift+Z", Act::Redo, !self.redo.is_empty()),
                Item::Rule,
                it("Crop...", "Ctrl+K", Act::CropTool, has_img),
                it(
                    if self.danger {
                        "DANGER crop: on"
                    } else {
                        "DANGER crop"
                    },
                    "Ctrl+Shift+K",
                    Act::Danger,
                    has_img || self.danger,
                ),
                it("Rotate right", "Ctrl+R", Act::Edit(Op::RotCw), has_img),
                it(
                    "Rotate left",
                    "Ctrl+Shift+R",
                    Act::Edit(Op::RotCcw),
                    has_img,
                ),
                it("Flip horizontal", "Ctrl+F", Act::Edit(Op::FlipH), has_img),
                it(
                    "Flip vertical",
                    "Ctrl+Shift+F",
                    Act::Edit(Op::FlipV),
                    has_img,
                ),
                Item::Rule,
                it("Move to trash", "Del", Act::Trash, has_img),
            ],
            Menu::View => vec![
                it(
                    if self.s.tree {
                        "Fold the tree"
                    } else {
                        "Unfold the tree"
                    },
                    "Tab",
                    Act::Tree,
                    self.s.dock.docked_count() > 0,
                ),
                it(
                    if self.s.image {
                        "Fold the image"
                    } else {
                        "Unfold the image"
                    },
                    "Shift+Tab",
                    Act::Image,
                    lib,
                ),
                it(
                    if self.s.dock.floating(Widget::Favourites) {
                        "Dock Favourites"
                    } else {
                        "Detach Favourites"
                    },
                    "",
                    Act::Widget(Widget::Favourites),
                    true,
                ),
                it(
                    if self.s.dock.floating(Widget::Files) {
                        "Dock Files"
                    } else {
                        "Detach Files"
                    },
                    "",
                    Act::Widget(Widget::Files),
                    true,
                ),
                it(
                    if self.s.dock.docked_count() == 0 {
                        "Recall both trees"
                    } else {
                        "Restore detached trees"
                    },
                    "3 clicks on fold tab",
                    Act::RecallTrees,
                    self.s.dock.pair_allowed(),
                ),
                it("Fit to window", "0", Act::Fit, has_img),
                it("Actual size", "Ctrl+1", Act::Actual, has_img),
                Item::Rule,
                it(
                    if self.s.title {
                        "Hide the title"
                    } else {
                        "Show the title"
                    },
                    "",
                    Act::Title,
                    has_img,
                ),
                it(
                    if self.s.toolbar {
                        "Hide the toolbar"
                    } else {
                        "Show the toolbar"
                    },
                    "",
                    Act::Toolbar,
                    true,
                ),
                it(
                    &format!("Tree style: {}", STYLE_NAMES[self.s.style % 3]),
                    "",
                    Act::Style(true),
                    true,
                ),
                it(
                    if self.s.pin_ui {
                        "Let the interface fade"
                    } else {
                        "Pin the interface"
                    },
                    "",
                    Act::Pin,
                    true,
                ),
                Item::Rule,
                it(
                    match (self.focus.active(), self.focus.stage, self.win_full) {
                        (true, Stage::Fit, _) => "Enlarge",
                        (true, Stage::Large, _) => "Full screen",
                        (true, Stage::Full, _) | (false, _, true) => "Leave full screen",
                        (false, _, false) => "Full screen",
                    },
                    "F11",
                    Act::Fullscreen,
                    true,
                ),
            ],
        }
    }

    /// Status line: where you are on the left, the slots on the right. The
    /// slot chips are throw targets for the mouse and landing marks for the
    /// keyboard. Their positions are kept even while the line is hidden.
    fn statusbar(&mut self, ctx: &egui::Context, g: &Grid, full: Rect, alpha: f32) -> Option<Act> {
        let h = g.rh * 1.5;
        let Some(lib) = self.lib.as_ref() else {
            self.chips.clear();
            return None;
        };
        let n = &lib.tree.nodes[lib.active];
        let sep = || run(" │ ", EDGE);
        let mut left: Vec<Run> = vec![run(n.name.clone(), RUST_HI)];
        if let Some(f) = n.files.get(lib.cursor) {
            let path = lib.tree.dir(lib.active).join(&f.name);
            left.push(sep());
            left.push(run(
                format!("{} of {}", lib.cursor + 1, n.files.len()),
                MUTED,
            ));
            left.push(sep());
            left.push(run(f.name.clone(), TEXT));
            if let Some(px) = self.current_px() {
                left.push(sep());
                left.push(run(format!("{}x{}", px[0], px[1]), MUTED));
            }
            if let Some(Slot::Ready { bytes, .. }) = self.loader.get(&path) {
                left.push(sep());
                left.push(run(format!("{:.1} MB", *bytes as f64 / 1_048_576.0), MUTED));
            }
            left.push(sep());
            left.push(run(
                if (self.zoom - 1.0).abs() < 0.01 {
                    "fit".to_string()
                } else {
                    format!("{:.0}%", self.zoom * 100.0)
                },
                MUTED,
            ));
            if self.pending.contains_key(&path) {
                left.push(sep());
                left.push(run("edited, not saved", RUST_HI));
            }
        }
        let cats: Vec<(usize, String, String, Color32)> = lib
            .tree
            .cats
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.key.clone().map(|k| (i, k, c.name.clone(), rgb(c.cid))))
            .collect();
        // In a narrow window the chips drop their names and keep the keys.
        let named = |k: &str, nm: &str| (k.chars().count() + nm.chars().count() + 4) as f32 * g.cw;
        let full_w: f32 = cats.iter().map(|(_, k, nm, _)| named(k, nm) + g.cw).sum();
        let compact = full_w > full.width() * 0.55;
        let chip_w = |k: &str, nm: &str| {
            if compact {
                (k.chars().count() + 2) as f32 * g.cw
            } else {
                named(k, nm)
            }
        };
        let rw: f32 = cats.iter().map(|(_, k, nm, _)| chip_w(k, nm) + g.cw).sum();
        let y0 = full.bottom() - h;
        let start = full.right() - rw - g.cw;
        let mut x = start;
        self.chips = cats
            .iter()
            .map(|(i, k, nm, _)| {
                let w = chip_w(k, nm);
                let c = Pos2::new(x + w * 0.5, y0 + h * 0.5);
                x += w + g.cw;
                (*i, c)
            })
            .collect();
        if alpha < 0.01 {
            return None;
        }
        let slide = (1.0 - alpha) * h * 0.5;
        let mut act = None;
        let mut hovered = false;
        Area::new(Id::new("statusbar"))
            .fixed_pos(Pos2::new(full.left(), y0 + slide))
            .order(Order::Foreground)
            .interactable((!self.yielding || self.s.pin_ui) && alpha > 0.05)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(Vec2::new(full.width(), h), Sense::CLICK);
                let p = ui.painter();
                p.rect_filled(rect, 0.0, IRON.gamma_multiply(alpha));
                p.line_segment(
                    [rect.left_top(), rect.right_top()],
                    Stroke::new(1.0, EDGE.gamma_multiply(alpha)),
                );
                let y = rect.top() + (h - g.rh) * 0.5;
                let clip =
                    Rect::from_min_max(rect.min, Pos2::new(start - g.cw * 2.0, rect.bottom()));
                g.line(
                    &p.with_clip_rect(clip),
                    Pos2::new(rect.left() + g.cw, y),
                    &left,
                    alpha,
                );
                let hover = resp.hover_pos();
                let mut x = start;
                for (i, k, nm, c) in &cats {
                    let w = chip_w(k, nm);
                    let cell =
                        Rect::from_min_size(Pos2::new(x, rect.top() + 3.0), Vec2::new(w, h - 6.0));
                    let hot = hover.is_some_and(|q| cell.contains(q));
                    p.rect_filled(
                        cell,
                        2.0,
                        (if hot { IRON_HI } else { STAGE }).gamma_multiply(alpha),
                    );
                    let kw = (k.chars().count() + 2) as f32 * g.cw;
                    p.rect_filled(
                        Rect::from_min_size(cell.min, Vec2::new(kw, cell.height())),
                        2.0,
                        c.gamma_multiply(alpha),
                    );
                    if compact {
                        g.text(p, Pos2::new(x, y), &format!(" {k} "), STAGE, alpha);
                    } else {
                        g.line(
                            p,
                            Pos2::new(x, y),
                            &[(format!(" {k} "), STAGE), (format!(" {nm}"), *c)],
                            alpha,
                        );
                    }
                    if resp.clicked()
                        && let Some(q) = resp.interact_pointer_pos()
                        && cell.contains(q)
                    {
                        act = Some(Act::Throw(*i));
                    }
                    if hot {
                        ctx.set_cursor_icon(CursorIcon::PointingHand);
                    }
                    x += w + g.cw;
                }
                hovered = resp.contains_pointer();
            });
        self.chrome_hover |= hovered;
        act
    }

    /// The fold tab, at the top of the divider just under the menu bar's
    /// band, so it arrives with the menu bar. `<` folds the tree away (image
    /// focus), `>` brings it back from the window's left edge; with the image
    /// folded it sits on the right edge and `<` brings the image back.
    fn fold_tab(
        &mut self,
        ctx: &egui::Context,
        g: &Grid,
        r: Rect,
        glyph: &str,
        act_on: Act,
        alpha: f32,
    ) -> Option<Act> {
        if alpha < 0.01 {
            return None;
        }
        let mut act = None;
        Area::new(Id::new("fold-tab"))
            .fixed_pos(r.min - Vec2::new(0.0, (1.0 - alpha) * g.rh * 0.4))
            .order(Order::Foreground)
            .interactable(!self.yielding && alpha > 0.05)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(r.size(), Sense::CLICK);
                let p = ui.painter();
                let hot = resp.hovered();
                let h = ctx.animate_bool_with_time(Id::new("fold-tab-hot"), hot, 0.1);
                let d = ctx.animate_bool_with_time(
                    Id::new("fold-tab-down"),
                    resp.is_pointer_button_down_on(),
                    0.05,
                );
                p.add(theme::shadow(alpha * 0.8).as_shape(rect, 4.0));
                p.rect_filled(rect, 4.0, lerp_color(IRON, RUST, h).gamma_multiply(alpha));
                p.rect_stroke(
                    rect,
                    4.0,
                    Stroke::new(1.0, EDGE.gamma_multiply(alpha * (1.0 - h))),
                    StrokeKind::Inside,
                );
                // The glyph leans the way the fold goes while hovered.
                let lean = if glyph == "<" { -1.5 } else { 1.5 } * h * (1.0 - d);
                g.text(
                    p,
                    rect.center() - Vec2::new(g.cw * 0.5 - lean, g.rh * 0.5 - d),
                    glyph,
                    lerp_color(RUST_HI, STAGE, h),
                    alpha,
                );
                if hot {
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
                if resp.clicked() {
                    act = match self
                        .fold_clicks
                        .click(ctx.input(|i| i.time), self.s.dock.pair_allowed())
                    {
                        FoldGesture::Single => Some(act_on),
                        FoldGesture::Pair => Some(Act::RecallTrees),
                        FoldGesture::None => None,
                    };
                    self.fold_action = if self.fold_clicks.pending() {
                        Some(act_on)
                    } else {
                        None
                    };
                }
                self.chrome_hover |= resp.contains_pointer();
            });
        act
    }

    fn settings_dialog(
        &mut self,
        ctx: &egui::Context,
        g: &Grid,
        full: Rect,
        alpha: f32,
    ) -> Option<Act> {
        let cats: Vec<Cat> = self
            .lib
            .as_ref()
            .map(|l| l.tree.cats.clone())
            .unwrap_or_else(config::defaults);
        let active = self
            .lib
            .as_ref()
            .map(|l| l.tree.path(l.active))
            .unwrap_or_else(|| "-".into());
        let tick = |b: bool| if b { "[x]" } else { "[ ]" };
        let spin = |label: &str, value: String, prev: Act, next: Act| -> Row<Act> {
            Row::new(vec![
                r(format!("├─ {label:<6} "), DIM),
                ra("◄", RUST_HI, prev),
                r(format!(" {value} "), TEXT),
                ra("►", RUST_HI, next),
            ])
        };
        let mut rows: Vec<Row<Act>> = vec![
            Row::new(vec![
                r("[settings]", RUST_HI),
                r(":style: ", DIM),
                r(format!("\"{}\"", STYLE_NAMES[self.s.style % 3]), TEXT),
            ]),
            Row::act(
                vec![
                    r("├─ ", DIM),
                    r(tick(self.s.pin_ui), RUST_HI),
                    r(" PIN_UI: Turn Active UI Fade OFF", TEXT),
                ],
                Act::Pin,
            ),
            Row::act(
                vec![
                    r("├─ ", DIM),
                    r(tick(self.s.tree), RUST_HI),
                    r(" Octobrachial Tree beside the image", TEXT),
                    r("   Tab", DIM),
                ],
                Act::Tree,
            ),
            Row::act(
                vec![
                    r("├─ ", DIM),
                    r(tick(self.s.title), RUST_HI),
                    r(" Title over the image", TEXT),
                    r("   folder and number", DIM),
                ],
                Act::Title,
            ),
            Row::act(
                vec![
                    r("├─ ", DIM),
                    r(tick(self.s.toolbar), RUST_HI),
                    r(" Toolbar at the top right", TEXT),
                    r("   crop tools", DIM),
                ],
                Act::Toolbar,
            ),
            spin(
                "font",
                FACES[self.s.font % 12].name.to_string(),
                Act::Font(false),
                Act::Font(true),
            ),
            spin(
                "size",
                format!("{}", SIZES[self.s.size % SIZES.len()]),
                Act::Size(false),
                Act::Size(true),
            ),
            spin(
                "style",
                STYLE_NAMES[self.s.style % 3].to_string(),
                Act::Style(false),
                Act::Style(true),
            ),
            Row::new(vec![r("└─ ", DIM), r("[create]", RUST_HI)]),
            Row::new(vec![
                r("   └─ ", DIM),
                r("[classification_category] ", TEXT),
                r(active, MUTED),
            ]),
        ];
        for (i, c) in cats.iter().enumerate() {
            // The key as a keycap in its category's colour: click it to press
            // a new key. The colour edits in place on a double-click. The
            // rest of the row opens the full form.
            let editing = self.key_edit.as_ref().is_some_and(|k| k.cat == i);
            let (key, kw) = match (&c.key, editing) {
                (_, true) => (chipa(" _ ", STAGE, RUST_HI, Act::KeyEdit(i)), 3),
                (Some(k), false) => (
                    chipa(format!(" {k} "), STAGE, rgb(c.cid), Act::KeyEdit(i)),
                    k.chars().count() + 2,
                ),
                (None, false) => (chipa(" + ", MUTED, IRON_HI, Act::KeyEdit(i)), 3),
            };
            rows.push(Row::act(
                vec![
                    r("      ├─ ", DIM),
                    r(format!("{}.", c.tier), MUTED),
                    r(format!("{:<10}", format!("[{}]", c.name)), rgb(c.cid)),
                    r(" ", DIM),
                    key,
                    r(" ".repeat(7 - kw.min(6)), DIM),
                    rd(hexs(c.cid), MUTED, Act::CidEdit(i)),
                    r(" ", DIM),
                    rd("████", rgb(c.cid), Act::CidEdit(i)),
                ],
                Act::EditCat(i),
            ));
        }
        rows.push(Row::act(
            vec![r("      └─ ", DIM), r("+ category", RUST_HI)],
            Act::NewCat,
        ));
        rows.push(Row::blank());
        rows.push(Row::new(vec![r(
            "Click a key or double-click a colour to change it.",
            DIM,
        )]));
        let size = theme::dialog_size(g, "Settings", &rows, 0);
        let pos = Pos2::new(full.center().x - size.x * 0.5, full.top() + g.rh * 3.0);
        let o = dialog(
            ctx,
            g,
            "settings",
            pos,
            "Settings",
            &rows,
            0,
            alpha,
            true,
            Act::Close,
            |_, _| {},
        );
        self.chrome_hover |= o.hovered;
        let now = ctx.input(|x| x.time);
        if let Some((Act::CidEdit(i), at)) = o.double
            && let Some(c) = self.lib.as_ref().and_then(|l| l.tree.cats.get(i))
        {
            self.key_edit = None;
            self.cid_edit = Some(CidEdit {
                cat: i,
                at,
                orig: c.cid,
                hex: hexs(c.cid),
                opened: now,
            });
        }
        if let (Some(Act::KeyEdit(i)), Some(at)) = (o.click, o.at) {
            self.close_cid(true);
            self.key_edit = match &self.key_edit {
                // A second click on the same keycap puts it away.
                Some(k) if k.cat == i => None,
                _ if self.lib.is_some() => Some(KeyEdit {
                    cat: i,
                    at,
                    opened: now,
                    err: None,
                }),
                _ => None,
            };
            return None;
        }
        o.click
    }

    /// The key popover under a clicked keycap: the next key pressed becomes
    /// the category's key. Backspace unbinds it, Esc or a click elsewhere
    /// leaves it as it was. A key that's taken swaps; one the app keeps
    /// shakes the popover.
    fn key_editor(&mut self, ctx: &egui::Context, g: &Grid, now: f64) {
        if !matches!(self.dialog, Dialog::Settings) {
            self.key_edit = None;
            return;
        }
        let (Some(ed), Some(lib)) = (self.key_edit.as_ref(), self.lib.as_ref()) else {
            return;
        };
        let Some(cat) = lib.tree.cats.get(ed.cat) else {
            self.key_edit = None;
            return;
        };
        let cid = rgb(cat.cid);
        let cur = cat.key.clone().unwrap_or_else(|| "-".into());
        let head: Vec<Run> = vec![(format!("[{}]", cat.name), cid), ("  ".into(), DIM)];
        let cap = format!(" {cur} ");
        let tail: Vec<Run> = vec![("  ".into(), DIM), ("press the new key".into(), TEXT)];
        let foot: Vec<Run> = match &ed.err {
            Some((e, _)) => vec![(e.clone(), WARN)],
            None => vec![("Backspace clears it, Esc keeps it".into(), DIM)],
        };
        let top_cols = Grid::cols(&head) + cap.chars().count() + Grid::cols(&tail);
        let cols = top_cols.max(Grid::cols(&foot)) + 4;
        let size = Vec2::new(cols as f32 * g.cw, 3.0 * g.rh);
        // A refused key shakes the popover.
        let shake = ed.err.as_ref().map_or(0.0, |(_, t0)| {
            let age = (now - t0) as f32;
            if age < 0.35 {
                (age * 60.0).sin() * 6.0 * (1.0 - age / 0.35)
            } else {
                0.0
            }
        });
        let pos = ed.at.left_bottom() + Vec2::new(-g.cw * 2.0 + shake, 6.0);
        let pulse = 0.5 + 0.5 * ((now - ed.opened) as f32 * 5.0).cos();
        let area = Area::new(Id::new("key-edit"))
            .fixed_pos(pos)
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                let (body, _) = ui.allocate_exact_size(size, Sense::CLICK | Sense::DRAG);
                let p = ui.painter();
                theme::frame(p, body, 1.0);
                let o = body.min + Vec2::new(g.cw * 2.0, g.rh * 0.5);
                g.line(p, o, &head, 1.0);
                let cx = o.x + Grid::cols(&head) as f32 * g.cw;
                let cell = Rect::from_min_size(
                    Pos2::new(cx, o.y + 1.0),
                    Vec2::new(cap.chars().count() as f32 * g.cw, g.rh - 2.0),
                );
                p.rect_filled(cell, 3.0, cid.gamma_multiply(0.45 + 0.55 * pulse));
                p.rect_stroke(
                    cell.expand(1.5),
                    4.0,
                    Stroke::new(1.5, TEXT.gamma_multiply(pulse)),
                    StrokeKind::Outside,
                );
                g.text(p, Pos2::new(cx, o.y), &cap, STAGE, 1.0);
                g.line(p, Pos2::new(cell.right(), o.y), &tail, 1.0);
                g.line(p, o + Vec2::new(0.0, g.rh), &foot, 1.0);
            });
        ctx.request_repaint();
        self.chrome_hover |= area.response.contains_pointer();
        let outside = ctx.input(|i| i.pointer.primary_clicked())
            && now > ed.opened
            && !ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| area.response.rect.contains(p) || ed.at.contains(p));
        if outside {
            self.key_edit = None;
        }
    }

    fn close_cid(&mut self, keep: bool) {
        let Some(ed) = self.cid_edit.take() else {
            return;
        };
        let Some(lib) = self.lib.as_mut() else { return };
        if let Some(c) = lib.tree.cats.get_mut(ed.cat) {
            if !keep {
                c.cid = ed.orig;
            } else if c.cid != ed.orig {
                let _ = config::save(&lib.tree.root, &lib.tree.cats);
            }
        }
    }

    /// The in-place colour editor under a double-clicked colour: hex field,
    /// picker, swatches. Every change lands on the category at once, so the
    /// tree, the keycaps and the status chips recolour while you drag.
    fn cid_editor(&mut self, ctx: &egui::Context, g: &Grid, now: f64) {
        if !matches!(self.dialog, Dialog::Settings) {
            self.close_cid(true);
            return;
        }
        let (Some(ed), Some(lib)) = (self.cid_edit.as_mut(), self.lib.as_mut()) else {
            return;
        };
        let Some(cat) = lib.tree.cats.get(ed.cat).cloned() else {
            self.cid_edit = None;
            return;
        };
        let mut c = rgb(cat.cid);
        let (mut done, mut revert) = (false, false);
        let font = g.font.clone();
        let button = |ui: &mut egui::Ui, t: &str, col: Color32| {
            let r = ui.add(
                egui::Label::new(egui::RichText::new(t).font(font.clone()).color(col))
                    .sense(Sense::CLICK),
            );
            if r.hovered() {
                ui.painter().rect_filled(r.rect.expand(1.0), 3.0, RUST);
                ui.painter().text(
                    r.rect.left_top(),
                    egui::Align2::LEFT_TOP,
                    t,
                    font.clone(),
                    STAGE,
                );
                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
            }
            r.clicked()
        };
        let area = Area::new(Id::new("cid-edit"))
            .fixed_pos(ed.at.left_bottom() + Vec2::new(-g.cw, 6.0))
            .order(Order::Tooltip)
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(IRON)
                    .stroke(Stroke::new(RULE, EDGE))
                    .corner_radius(ROUND)
                    .inner_margin(egui::Margin::same(12))
                    .shadow(theme::shadow(1.0))
                    .show(ui, |ui| {
                        ui.spacing_mut().slider_width = g.cw * 20.0;
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("[{}]", cat.name))
                                    .font(font.clone())
                                    .color(c),
                            );
                            ui.add_space(g.cw);
                            let te = ui.add(
                                TextEdit::singleline(&mut ed.hex)
                                    .font(font.clone())
                                    .text_color(TEXT)
                                    .desired_width(g.cw * 8.0)
                                    .background_color(STAGE),
                            );
                            if te.changed()
                                && let Some(h) = config::hex(&ed.hex)
                            {
                                c = rgb(h);
                            }
                            if te.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                                done = true;
                            }
                        });
                        ui.add_space(8.0);
                        if egui::widgets::color_picker::color_picker_color32(
                            ui,
                            &mut c,
                            egui::widgets::color_picker::Alpha::Opaque,
                        ) {
                            ed.hex = format!("#{:02X}{:02X}{:02X}", c.r(), c.g(), c.b());
                        }
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            for sw in SWATCHES {
                                let sc = config::hex(sw).map_or(MUTED, rgb);
                                let (r, resp) =
                                    ui.allocate_exact_size(Vec2::splat(g.rh * 0.9), Sense::CLICK);
                                ui.painter().rect_filled(r, 3.0, sc);
                                if resp.hovered() || sc == c {
                                    ui.painter().rect_stroke(
                                        r,
                                        3.0,
                                        Stroke::new(1.5, TEXT),
                                        StrokeKind::Outside,
                                    );
                                }
                                if resp.clicked() {
                                    c = sc;
                                    ed.hex = sw.to_uppercase();
                                }
                            }
                        });
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            done |= button(ui, "[ Done ]", RUST_HI);
                            ui.add_space(g.cw);
                            revert |= button(ui, "[ Revert ]", MUTED);
                        });
                    });
            });
        // Live: the category takes the colour now.
        if let Some(k) = lib.tree.cats.get_mut(ed.cat) {
            k.cid = (c.r(), c.g(), c.b());
        }
        self.chrome_hover |= area.response.contains_pointer();
        let outside = ctx.input(|i| i.pointer.primary_clicked())
            && now > ed.opened
            && !ctx
                .input(|i| i.pointer.interact_pos())
                .is_some_and(|p| area.response.rect.contains(p));
        if revert {
            self.close_cid(false);
        } else if done || outside {
            self.close_cid(true);
        }
    }

    fn form_dialog(
        &mut self,
        ctx: &egui::Context,
        g: &Grid,
        full: Rect,
        alpha: f32,
    ) -> Option<Act> {
        let Dialog::Form(f) = &mut self.dialog else {
            return None;
        };
        let cid = config::hex(&f.cid);
        let title = if f.edit.is_some() {
            "Edit category"
        } else {
            "New category"
        };
        let mut rows: Vec<Row<Act>> = vec![
            Row::new(vec![r("├─ name   ", DIM), r(" ".repeat(20), TEXT)]),
            Row::act(
                vec![
                    r("├─ key    ", DIM),
                    match (&f.key, f.capturing) {
                        (_, true) => r("press a key", RUST_HI),
                        (Some(k), false) => chip(format!(" {k} "), STAGE, cid.map_or(MUTED, rgb)),
                        (None, false) => r(" - ", DIM),
                    },
                    r(
                        if f.capturing {
                            ""
                        } else {
                            "   click, then press the key"
                        },
                        DIM,
                    ),
                ],
                Act::FormKey,
            ),
            Row::new(vec![
                r("├─ colour ", DIM),
                r(" ".repeat(9), TEXT),
                r(" ████  ", cid.map_or(WARN, rgb)),
                ra("◄", RUST_HI, Act::Swatch(false)),
                r(" swatch ", MUTED),
                ra("►", RUST_HI, Act::Swatch(true)),
            ]),
            Row::new(vec![
                r("├─ tier   ", DIM),
                ra("◄", RUST_HI, Act::Tier(false)),
                r(format!(" {} ", f.tier), TEXT),
                ra("►", RUST_HI, Act::Tier(true)),
                r("   a sits on top", DIM),
            ]),
            Row::blank(),
        ];
        let mut buttons = vec![
            r("└─ ", DIM),
            ra("[ Save ]", RUST_HI, Act::FormSave),
            r("  ", DIM),
        ];
        if f.edit.is_some() {
            buttons.push(ra("[ Delete ]", WARN, Act::FormDelete));
            buttons.push(r("  ", DIM));
        }
        buttons.push(ra("[ Cancel ]", MUTED, Act::FormCancel));
        rows.push(Row::new(buttons));
        rows.push(Row::new(vec![r(
            format!("   {}", f.err.clone().unwrap_or_default()),
            WARN,
        )]));
        let size = theme::dialog_size(g, title, &rows, 44);
        let pos = Pos2::new(full.center().x - size.x * 0.5, full.top() + g.rh * 3.0);
        let font = g.font.clone();
        let (name_ref, cid_ref) = (&mut f.name, &mut f.cid);
        let o = dialog(
            ctx,
            g,
            "form",
            pos,
            title,
            &rows,
            44,
            alpha,
            true,
            Act::Close,
            |ui, o| {
                let field = |row: usize, col: usize, w: usize| {
                    Rect::from_min_size(
                        o + Vec2::new(col as f32 * g.cw, row as f32 * g.rh),
                        g.size(w, 1),
                    )
                };
                theme::field_bg(ui.painter(), field(0, 10, 20));
                theme::field_bg(ui.painter(), field(2, 10, 9));
                ui.put(
                    field(0, 10, 20),
                    TextEdit::singleline(name_ref)
                        .font(font.clone())
                        .text_color(TEXT)
                        .frame(egui::Frame::NONE)
                        .margin(Vec2::ZERO),
                );
                ui.put(
                    field(2, 10, 9),
                    TextEdit::singleline(cid_ref)
                        .font(font.clone())
                        .text_color(TEXT)
                        .frame(egui::Frame::NONE)
                        .margin(Vec2::ZERO),
                );
            },
        );
        self.chrome_hover |= o.hovered;
        o.click
    }

    fn help_dialog(
        &mut self,
        ctx: &egui::Context,
        g: &Grid,
        full: Rect,
        alpha: f32,
    ) -> Option<Act> {
        let cats: Vec<Cat> = self
            .lib
            .as_ref()
            .map(|l| l.tree.cats.clone())
            .unwrap_or_else(config::defaults);
        let mut rows: Vec<Row<Act>> = vec![Row::new(vec![r("Categories", RUST_HI)])];
        for c in &cats {
            if let Some(k) = &c.key {
                rows.push(Row::new(vec![
                    r(format!("  {k:<20}"), TEXT),
                    r(format!("move the image to {}", c.name), rgb(c.cid)),
                ]));
            }
        }
        let sec = |rows: &mut Vec<Row<Act>>, head: &str, items: &[(&str, &str)]| {
            rows.push(Row::blank());
            rows.push(Row::new(vec![r(head.to_string(), RUST_HI)]));
            for (k, d) in items {
                rows.push(Row::new(vec![
                    r(format!("  {k:<20}"), TEXT),
                    r(d.to_string(), MUTED),
                ]));
            }
        };
        sec(
            &mut rows,
            "Browse",
            &[
                ("← →", "one back, one on"),
                ("↑ ↓", "five back, five on"),
                ("[ ]", "25 on, 25 back"),
                ("; '", "50 on, 50 back"),
            ],
        );
        sec(
            &mut rows,
            "Edit",
            &[
                ("Ctrl+K", "crop; Enter applies, A changes the ratio"),
                (
                    "Ctrl+Shift+K",
                    "DANGER crop: drag a box, it crops and saves",
                ),
                ("Ctrl+R", "rotate right; with Shift, left"),
                ("Ctrl+F", "flip; with Shift, vertical"),
                ("Ctrl+S", "save edits into the file"),
                ("Ctrl+Shift+S", "export as a new file"),
                ("Ctrl+Z", "undo"),
                ("Ctrl+Shift+Z", "redo"),
                ("Del", "move to trash"),
            ],
        );
        sec(
            &mut rows,
            "View",
            &[
                ("Tab", "fold or unfold the tree"),
                ("wheel, drag", "zoom, pan"),
                ("0", "fit to window"),
                ("Ctrl+1", "actual size"),
                ("F11", "full screen"),
                (
                    "top bar",
                    "drag to move the window, double-click to maximize",
                ),
                (".", "settings"),
            ],
        );
        sec(
            &mut rows,
            "Tree",
            &[
                ("click a folder", "open it; again to fold"),
                ("click a slot", "move the image there"),
                ("right-click a slot", "browse what it holds"),
            ],
        );
        sec(
            &mut rows,
            "Tree windows",
            &[
                ("drag a header", "detach; drag back left to dock"),
                ("Shift+click header", "dock / restore detached position"),
                (
                    "triple-click fold tab",
                    "recall both detached trees; again restores",
                ),
                ("drag the horizontal rule", "resize the joined widgets"),
            ],
        );
        sec(
            &mut rows,
            "Files",
            &[
                (
                    "anchor in hover header",
                    "on: keep folder; off: follow the app",
                ),
                ("click a folder / file", "expand folder / preview media"),
                ("double-click a folder", "browse it as the root"),
                ("Ctrl+L / Alt+Up", "enter a path / parent folder"),
                ("Alt+Left / Alt+Right", "back / forward"),
                ("F5", "refresh the selected folder"),
            ],
        );
        let size = theme::dialog_size(g, "Keys", &rows, 0);
        let pos = Pos2::new(
            full.center().x - size.x * 0.5,
            (full.center().y - size.y * 0.5).max(full.top() + g.rh * 2.0),
        );
        let o = dialog(
            ctx,
            g,
            "help",
            pos,
            "Keys",
            &rows,
            0,
            alpha,
            true,
            Act::Close,
            |_, _| {},
        );
        self.chrome_hover |= o.hovered;
        o.click
    }

    fn unsaved_dialog(
        &mut self,
        ctx: &egui::Context,
        g: &Grid,
        full: Rect,
        alpha: f32,
    ) -> Option<Act> {
        let n = self.pending.len();
        let opening = self.next_library.is_some();
        let rows: Vec<Row<Act>> = vec![
            Row::new(vec![r(
                format!(
                    "{n} image{} {} edits that aren't saved.",
                    if n == 1 { "" } else { "s" },
                    if n == 1 { "has" } else { "have" }
                ),
                TEXT,
            )]),
            Row::blank(),
            Row::new(vec![
                ra(
                    if opening {
                        "[ Save all and open ]"
                    } else {
                        "[ Save all and quit ]"
                    },
                    RUST_HI,
                    Act::SaveAllQuit,
                ),
                r("  ", DIM),
                ra(
                    if opening {
                        "[ Open without saving ]"
                    } else {
                        "[ Quit without saving ]"
                    },
                    WARN,
                    Act::DiscardQuit,
                ),
                r("  ", DIM),
                ra("[ Cancel ]", MUTED, Act::Close),
            ]),
        ];
        let size = theme::dialog_size(g, "Unsaved edits", &rows, 0);
        dialog(
            ctx,
            g,
            "unsaved",
            full.center() - size * 0.5,
            "Unsaved edits",
            &rows,
            0,
            alpha,
            true,
            Act::Close,
            |_, _| {},
        )
        .click
    }

    /// The crop tool's bar. It sits under the picture's middle, or over it
    /// when the frame's bottom edge runs under the bar; and whenever the
    /// pointer comes within reach of a frame edge or corner, or the frame is
    /// being dragged, it fades and lets the pointer through, so no edge of
    /// the picture is ever out of reach.
    fn crop_bar(&mut self, ctx: &egui::Context, g: &Grid, stage: Rect) -> Option<Act> {
        let Some(c) = self.crop.as_ref() else {
            ctx.animate_bool_with_time(Id::new("crop-bar"), false, 0.14);
            return None;
        };
        let rows = vec![Row::new(vec![
            r("Ratio ", MUTED),
            ra("◄", RUST_HI, Act::Aspect(false)),
            r(format!(" {:<8} ", ASPECTS[c.aspect].0), TEXT),
            ra("►", RUST_HI, Act::Aspect(true)),
            r("     ", DIM),
            ra("[ Apply ]", RUST_HI, Act::CropApply),
            r("  ", DIM),
            ra("[ Cancel ]", MUTED, Act::CropCancel),
        ])];
        let size = theme::dialog_size(g, "Crop", &rows, 0);
        let below = Rect::from_min_size(
            Pos2::new(
                stage.center().x - size.x * 0.5,
                stage.bottom() - size.y - g.rh * 2.5,
            ),
            size,
        );
        let above = Rect::from_min_size(Pos2::new(below.left(), stage.top() + g.rh * 2.5), size);
        let band = g.rh.max(16.0);
        let frame = self.img_rect.map(|ir| {
            Rect::from_min_max(
                ir.lerp_inside(Vec2::new(c.r[0], c.r[1])),
                ir.lerp_inside(Vec2::new(c.r[2], c.r[3])),
            )
        });
        let crosses = |bar: Rect, y: f32| y > bar.top() - band && y < bar.bottom() + band;
        let at = match frame {
            Some(f) if crosses(below, f.bottom()) && !crosses(above, f.top()) => above,
            _ => below,
        };
        let near_edge = frame.is_some_and(|f| {
            ctx.input(|i| i.pointer.hover_pos())
                .is_some_and(|p| f.expand(band).contains(p) && !f.shrink(band).contains(p))
        });
        let live = c.grab.is_none() && !near_edge;
        let alpha = ctx
            .animate_bool_with_time(Id::new("crop-bar"), live, 0.12)
            .max(0.12);
        dialog(
            ctx,
            g,
            "cropbar",
            at.min,
            "Crop",
            &rows,
            0,
            alpha,
            live,
            Act::CropCancel,
            |_, _| {},
        )
        .click
    }

    /// Room the open overlays need, read from where they were drawn. A
    /// centred window needs its width either side of the centre; a menu
    /// hangs from its title and needs to reach its far edge; a popover
    /// needs its distance from the centre, both ways.
    fn overlay_need(&self, ctx: &egui::Context, g: &Grid, full: Rect) -> Vec2 {
        let at = |id: &str| ctx.memory(|m| m.area_rect(Id::new(id)));
        let mut need = Vec2::ZERO;
        let centred = match self.dialog {
            Dialog::Settings => Some("settings"),
            Dialog::Form(_) => Some("form"),
            Dialog::Help => Some("help"),
            Dialog::Unsaved => Some("unsaved"),
            Dialog::None => None,
        };
        for id in centred
            .into_iter()
            .chain(self.crop.is_some().then_some("cropbar"))
        {
            if let Some(r) = at(id) {
                need = need.max(Vec2::new(r.width() + g.cw * 4.0, r.height() + g.rh * 6.0));
            }
        }
        if self.menu.is_some()
            && let Some(r) = at("dropdown")
        {
            need = need.max(Vec2::new(
                r.right() - full.left() + g.cw,
                r.bottom() - full.top() + g.rh,
            ));
        }
        let c = full.center().x;
        for (on, id) in [
            (self.cid_edit.is_some(), "cid-edit"),
            (self.key_edit.is_some(), "key-edit"),
        ] {
            if on && let Some(r) = at(id) {
                let half = (r.right() - c).max(c - r.left());
                need = need.max(Vec2::new(
                    half * 2.0 + g.cw * 2.0,
                    r.bottom() - full.top() + g.rh,
                ));
            }
        }
        need
    }

    fn splash(&self, p: &egui::Painter, g: &Grid, rect: Rect) {
        // An original calamari in block glyphs: mantle, fins, eyes, arms.
        let art: [(&str, Color32); 11] = [
            ("       ▄█▄       ", RUST_HI),
            ("      █████      ", RUST_HI),
            ("  ▄▄▄███████▄▄▄  ", RUST),
            ("     ███████     ", RUST),
            ("     ███████     ", RUST),
            ("     █◙███◙█     ", RUST),
            ("     ▀█████▀     ", RUST),
            ("    //|||||\\\\    ", MUTED),
            ("   // ||||| \\\\   ", MUTED),
            ("  //  || ||  \\\\  ", DIM),
            ("  ~   ~   ~   ~  ", DIM),
        ];
        let lines: [(&str, Color32); 4] = [
            ("Rusted Calamari", TEXT),
            ("", TEXT),
            ("Drop a folder here to open it.", MUTED),
            ("Ctrl+O opens one. H lists the keys.", DIM),
        ];
        let h = (art.len() + 2 + lines.len()) as f32 * g.rh;
        let mut y = rect.center().y - h * 0.5;
        for (s, c) in art {
            let w = s.chars().count() as f32 * g.cw;
            g.text(p, Pos2::new(rect.center().x - w * 0.5, y), s, c, 1.0);
            y += g.rh;
        }
        y += g.rh * 2.0;
        for (s, c) in lines {
            let w = s.chars().count() as f32 * g.cw;
            g.text(p, Pos2::new(rect.center().x - w * 0.5, y), s, c, 1.0);
            y += g.rh;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.font_on != Some(self.s.font) {
            fonts::install(&ctx, self.s.font);
            self.font_on = Some(self.s.font);
            self.hold_frames = 3;
        }
        let now = ctx.input(|i| i.time);

        #[cfg(feature = "workspace-smoke")]
        self.smoke_capture(&ctx);
        self.files.poll();
        if self.fold_clicks.poll(now) == FoldGesture::Single
            && let Some(a) = self.fold_action.take()
        {
            self.act(&ctx, a, now);
        }
        if self.fold_clicks.pending() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }

        // Closing waits for queued writes; with unsaved edits it asks first.
        if ctx.input(|i| i.viewport().close_requested()) {
            self.settle(true);
            if !self.pending.is_empty() && !self.quitting {
                ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                self.next_library = None;
                self.dialog = Dialog::Unsaved;
            }
        }

        self.drops(&ctx, now);
        self.loader.poll(&ctx);
        self.settle(false);
        // A DANGER cut steps aside once the file's new decode is in.
        let done: Vec<PathBuf> = self
            .snaps
            .iter()
            .filter(|(p, s)| now - s.t0 > 0.8 && self.snap_live(p).is_none())
            .map(|(p, _)| p.clone())
            .collect();
        for p in done {
            self.snaps.remove(&p);
        }
        self.win_full = window::state(&ctx).1;
        if ctx.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| self.img_rect.is_some_and(|r| r.contains(p)))
        }) {
            self.files.focused = false;
        }
        self.keys(&ctx, now);
        if ctx.input(|i| i.pointer.delta() != Vec2::ZERO) {
            self.quiet = false;
        }
        let g = Grid::new(&ctx, SIZES[self.s.size % SIZES.len()]);
        let full = ui.max_rect();
        self.main_rect = ctx.input(|i| i.viewport().inner_rect);
        ui.painter().rect_filled(full, 0.0, STAGE);

        // What the chrome is doing this frame.
        let ptr = ctx.input(|i| i.pointer.hover_pos());
        let dialog_open = !matches!(self.dialog, Dialog::None);
        let near_top = ptr.is_some_and(|p| p.y < full.top() + g.rh * 2.0);
        let near_bottom = ptr.is_some_and(|p| p.y > full.bottom() - g.rh * 2.5);
        let was_hover = self.chrome_hover;
        self.chrome_hover = false;
        let awake = !self.quiet || self.s.pin_ui;
        // While a crop is on (Crop or DANGER), the floating interface yields
        // to the picture: with the pointer on or at the picture, or a frame
        // being dragged, the menu bar, status line, tool strip and fold tab
        // neither come up nor take clicks, so every edge of the picture can
        // be reached. A menu or window opened on purpose (Alt+F, `.`) still
        // opens; pinned bars have their own room and stay.
        let crop_on = self.crop.is_some() || self.danger;
        let on_pic = ptr.is_some_and(|p| self.img_rect.is_some_and(|r| r.expand(g.rh).contains(p)));
        let framing = self.quick.is_some() || self.crop.as_ref().is_some_and(|c| c.grab.is_some());
        self.yielding = crop_on && (on_pic || framing);
        let floating = awake && !self.yielding;
        let bar_on = self.s.pin_ui
            || dialog_open
            || self.menu.is_some()
            || (floating && (near_top || (was_hover && !near_bottom)));
        let status_on = self.s.pin_ui || (floating && near_bottom);
        let a_bar = ctx.animate_bool_with_time(Id::new("fade-bar"), bar_on, 0.16);
        let a_status = ctx.animate_bool_with_time(Id::new("fade-status"), status_on, 0.16);

        // Layout. Pinned chrome takes its own room; hovered chrome floats over
        // the image. The tree's width animates and the stage takes the rest,
        // so folding the tree grows the image. Folding the image gives the
        // tree the window.
        let bar_h = g.rh * 1.5;
        let top = full.top() + if self.s.pin_ui { bar_h } else { 0.0 };
        let bottom = full.bottom() - if self.s.pin_ui { bar_h } else { 0.0 };
        let lib_on = self.lib.is_some();
        let tree_on = self.s.tree && self.s.dock.docked_count() > 0;
        let img_on = self.s.image || !tree_on;
        let dragging = ctx.is_being_dragged(Id::new("divider"));

        // Image focus: the tree folded, the window becomes the picture's
        // frame (see `focus`). It waits for a divider drag to end.
        let focus_on = lib_on && !tree_on && img_on && !dragging;
        let ctl_w = (bar_h * 1.6).round();
        let want = focus::Want {
            aspect: self.current_px().map(|[w, h]| w as f32 / h.max(1) as f32),
            chrome: Vec2::new(0.0, if self.s.pin_ui { bar_h * 2.0 } else { 0.0 }),
            need: self.need,
            // The menu bar's least: the logo, File Edit View, the window buttons.
            min: Vec2::new((g.cw * 24.0 + ctl_w * 3.0).max(320.0), 200.0),
        };
        let work = if focus_on || self.focus.active() {
            focus::work_area(frame, &ctx)
        } else {
            None
        };
        self.focus.step(&ctx, focus_on, &want, work, now);
        let focused = self.focus.active();

        let open_w = self.tree_width(&g, full);
        let target = match (tree_on, img_on) {
            (false, _) => 0.0,
            (true, false) => full.width(),
            (true, true) => open_w,
        };
        // In focus the window itself changes shape; the panel goes at once.
        let pw = ctx.animate_value_with_time(
            Id::new("panel-w"),
            target,
            if focused { 0.0 } else { 0.22 },
        );
        // The margin between tree and image opens and closes with either
        // fold; its centre line is the divider.
        let open_frac = if open_w > 0.0 {
            (pw / open_w).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let img_frac = ((full.width() - pw) / (g.cw * 6.0)).clamp(0.0, 1.0);
        let gutter = g.cw * 2.0 * open_frac * img_frac;
        let line_x = (full.left() + pw + gutter * 0.5).min(full.right() - 2.0);
        let panel = Rect::from_min_max(
            Pos2::new(full.left(), top),
            Pos2::new(full.left() + pw, bottom),
        );
        let stage = Rect::from_min_max(
            Pos2::new((full.left() + pw + gutter).min(full.right()), top),
            Pos2::new(full.right(), bottom),
        );
        self.snap_zone = self.main_rect.map(|r| {
            Rect::from_min_max(
                Pos2::new(r.left(), r.top() + top),
                Pos2::new(
                    r.left()
                        + if tree_on {
                            pw.max(48.0)
                        } else {
                            (g.cw * 8.0).max(48.0)
                        },
                    r.top() + bottom,
                ),
            )
        });

        let mut acts: Vec<Act> = Vec::new();
        {
            if pw > 1.0 {
                self.docked_trees(ui, panel, &g, now);
            }
            // The divider: drag it to set the tree's width, all the way left
            // to fold the tree, all the way right to fold the image;
            // double-click it to go back to hugging the tree.
            let edge_l = full.left() + g.cw * 8.0;
            let edge_r = full.right() - g.cw * 8.0;
            let split = pw > 1.0 && full.width() - pw > 1.0;
            let zone = if split || dragging {
                Some(Rect::from_center_size(
                    Pos2::new(line_x, panel.center().y),
                    Vec2::new(g.cw * 1.4, panel.height()),
                ))
            } else if !img_on {
                // Image folded: grab the window's right edge to pull it back.
                Some(Rect::from_min_max(
                    Pos2::new(full.right() - 14.0, top),
                    Pos2::new(full.right() - 4.0, bottom),
                ))
            } else {
                None
            };
            if let Some(zone) = zone {
                let dr = ui.interact(zone, Id::new("divider"), Sense::CLICK | Sense::DRAG);
                let hot = dr.hovered() || dr.dragged();
                let a_hot = ctx.animate_bool_with_time(Id::new("divider-hot"), hot, 0.12);
                if hot {
                    ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                }
                if dr.dragged()
                    && let Some(q) = dr.interact_pointer_pos()
                {
                    if q.x < edge_l {
                        self.s.tree = false;
                        self.s.image = true;
                    } else if q.x > edge_r {
                        self.s.image = false;
                        self.s.tree = true;
                    } else {
                        self.s.tree = true;
                        self.s.image = true;
                        let max = (full.width() - g.cw * 24.0).max(g.cw * 24.0);
                        self.s.panel_w = Some((q.x - full.left() - g.cw).clamp(g.cw * 24.0, max));
                    }
                }
                if dr.double_clicked() {
                    self.s.panel_w = None;
                    self.tree_hold = 0.0;
                }
                let fade = if split || dragging {
                    open_frac.max(0.35)
                } else {
                    a_hot
                };
                let col = lerp_color(EDGE.gamma_multiply(0.7), RUST_HI, a_hot).gamma_multiply(fade);
                let p = ui.painter();
                let y0 = top + g.rh * 0.75;
                let y1 = bottom - g.rh * 0.75;
                let lx = line_x.round() + 0.5;
                p.line_segment(
                    [Pos2::new(lx, y0), Pos2::new(lx, y1)],
                    Stroke::new(1.0 + a_hot, col),
                );
                if a_hot > 0.01 {
                    // Grip: three dots where the pointer meets the rule.
                    let gy = dr
                        .hover_pos()
                        .or(dr.interact_pointer_pos())
                        .map_or(panel.center().y, |h| h.y)
                        .clamp(y0 + 12.0, y1 - 12.0);
                    for d in [-7.0, 0.0, 7.0] {
                        p.circle_filled(
                            Pos2::new(line_x, gy + d),
                            2.0,
                            RUST_HI.gamma_multiply(a_hot),
                        );
                    }
                }
                // Past either edge: the side that will fold dims, so the
                // drag says what letting go will do.
                if dragging && let Some(q) = ctx.input(|i| i.pointer.interact_pos()) {
                    let fold = if q.x < edge_l {
                        Some(Rect::from_min_max(
                            full.min,
                            Pos2::new(full.left() + g.cw * 3.0, full.bottom()),
                        ))
                    } else if q.x > edge_r {
                        Some(Rect::from_min_max(
                            Pos2::new(full.right() - g.cw * 3.0, full.top()),
                            full.max,
                        ))
                    } else {
                        None
                    };
                    if let Some(f) = fold {
                        p.rect_filled(f, 0.0, RUST.gamma_multiply(0.18));
                    }
                }
            }
            if !lib_on {
                self.splash(ui.painter(), &g, stage);
            } else if img_on && stage.width() > g.cw * 4.0 {
                // In focus the picture meets the window's edges.
                let pad = if focused { 0.0 } else { g.rh * 0.6 };
                self.stage(ui, stage.shrink(pad), &g, now, focused);
            } else {
                self.img_rect = None;
            }
        }

        // Fold tab: at the top of the divider, under the menu bar's band. It
        // arrives with the menu bar, or when the pointer comes near it.
        if self.files.browser.is_some() {
            let size = Vec2::new(g.cw * 1.8, g.rh * 2.0);
            let ty = full.top() + bar_h + g.rh * 0.5;
            let (x, glyph, act) = match (tree_on, img_on) {
                (false, _) => (full.left() + 8.0, ">", Act::Tree),
                (true, false) => (full.right() - size.x - 8.0, "<", Act::Image),
                (true, true) => (line_x - size.x * 0.5, "<", Act::Tree),
            };
            let tab = Rect::from_min_size(Pos2::new(x.max(full.left() + 6.0), ty), size);
            let near = ptr.is_some_and(|p| {
                tab.expand2(Vec2::new(g.cw * 4.0, g.rh * 2.5)).contains(p)
                    || (tree_on && img_on && (p.x - line_x).abs() < g.cw * 2.0)
            });
            // Never over an open window or menu.
            let tab_on = !dialog_open
                && self.menu.is_none()
                && !dragging
                && !self.yielding
                && (self.s.pin_ui || bar_on || (awake && near));
            let a_tab = ctx.animate_bool_with_time(Id::new("fade-tab"), tab_on, 0.16);
            if let Some(a) = self.fold_tab(&ctx, &g, tab, glyph, act, a_tab) {
                acts.push(a);
            }
        }

        if let Some(a) = self.statusbar(&ctx, &g, full, a_status) {
            acts.push(a);
        }

        // Tool strip: top right, just under the menu bar's band so the two
        // never overlap. It wakes with the rest of the interface, when the
        // pointer comes near it, and stays while one of its tools is on.
        let tools = self.tools();
        let engaged = tools.iter().any(|t| t.on);
        let anchor = Pos2::new(full.right() - g.cw, full.top() + bar_h + g.rh * 0.5);
        let room = (stage.width() - g.cw * 2.0).max(g.rh * 3.0);
        let zone = toolbar::bounds(&g, anchor, room, &tools).expand(g.rh * 1.5);
        let near_tools = ptr.is_some_and(|p| zone.contains(p));
        // Turned on from a menu, it shows itself for a moment.
        let just_on = self.tools_woke.is_some_and(|t0| now - t0 < 1.6);
        if just_on {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        let tb_on = self.s.toolbar
            && self.lib.is_some()
            && !self.yielding
            && (self.s.pin_ui
                || bar_on
                || engaged
                || just_on
                || self.tools_hover
                || (awake && near_tools));
        let a_tb = ctx.animate_bool_with_time(Id::new("fade-tools"), tb_on, 0.16);
        let tb = toolbar::show(&ctx, &g, anchor, room, &tools, a_tb, !self.yielding, now);
        self.tools_hover = tb.hovered;
        if tb.rect.is_positive() {
            self.toolbar_rect = Some(tb.rect);
        }
        if let Some(a) = tb.click {
            acts.push(a);
        }

        // Throws in flight: the image drops toward its slot chip.
        let fp = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("fly")));
        let mut landed = Vec::new();
        for (i, f) in self.fly.iter().enumerate() {
            let t = ((now - f.t0) / 0.26).min(1.0) as f32;
            let e = t * t * (3.0 - 2.0 * t);
            let target = self
                .chips
                .iter()
                .find(|(c, _)| *c == f.cat)
                .map_or(Pos2::new(full.center().x, full.bottom()), |c| c.1);
            let to = Rect::from_center_size(target, Vec2::new(g.cw * 3.0, g.rh));
            let r = Rect::from_min_max(f.from.min.lerp(to.min, e), f.from.max.lerp(to.max, e));
            let cid = self
                .lib
                .as_ref()
                .and_then(|l| l.tree.cats.get(f.cat))
                .map_or(RUST_HI, |c| rgb(c.cid));
            if let Some(tex) = &f.tex {
                fp.image(
                    tex.id(),
                    r,
                    f.uv,
                    Color32::WHITE.gamma_multiply(1.0 - 0.6 * e),
                );
            }
            fp.rect_stroke(r, 0.0, Stroke::new(2.0, cid), StrokeKind::Inside);
            if t >= 1.0 {
                landed.push(i);
            }
        }
        for i in landed.into_iter().rev() {
            let f = self.fly.remove(i);
            self.landing = Some((f.cat, now));
        }
        if let Some((cat, t0)) = self.landing
            && now - t0 < 0.7
            && let Some((_, pos)) = self.chips.iter().find(|(c, _)| *c == cat)
            && let Some(c) = self.lib.as_ref().and_then(|l| l.tree.cats.get(cat))
        {
            let k = 1.0 - ((now - t0) / 0.7) as f32;
            let label = format!(" {} ", c.name);
            let r = Rect::from_center_size(*pos, g.size(label.chars().count(), 1));
            fp.rect_filled(r.expand(2.0), 2.0, rgb(c.cid).gamma_multiply(k));
            g.text(&fp, r.min, &label, STAGE, k);
        }

        // Toast: the outcome of anything with nothing else on screen to show for it.
        if let Some((m, c, t0)) = self.toast.clone() {
            let t0 = if t0 == 0.0 { now } else { t0 };
            self.toast = Some((m.clone(), c, t0));
            let age = now - t0;
            if age < 4.0 {
                let a = (1.0 - (age - 3.2).max(0.0) as f32 / 0.8).clamp(0.0, 1.0);
                let tp = ctx.layer_painter(LayerId::new(Order::Foreground, Id::new("toast")));
                let w = m.chars().count() + 4;
                // Under the tool strip when it is up, so they never stack.
                let mut ty = top + g.rh * 2.0;
                if a_tb > 0.01
                    && let Some(r) = self.toolbar_rect
                {
                    ty = ty.max(r.bottom() + g.rh * 0.6);
                }
                let pos = Pos2::new(
                    (full.right() - (w as f32 + 2.0) * g.cw).max(full.left() + g.cw),
                    ty,
                );
                let box_r =
                    Rect::from_min_size(pos, g.size(w, 1)).expand2(Vec2::new(0.0, g.rh * 0.4));
                tp.rect_filled(
                    box_r.translate(Vec2::new(3.0, 4.0)),
                    2.0,
                    SHADOW.gamma_multiply(a),
                );
                tp.rect_filled(box_r, 2.0, IRON.gamma_multiply(a));
                tp.rect_filled(
                    Rect::from_min_size(box_r.min, Vec2::new(3.0, box_r.height())),
                    0.0,
                    c.gamma_multiply(a),
                );
                g.text(&tp, pos + Vec2::new(g.cw * 2.0, 0.0), &m, TEXT, a);
                ctx.request_repaint();
            } else {
                self.toast = None;
            }
        }

        if let Some(a) = self.crop_bar(&ctx, &g, stage) {
            acts.push(a);
        }
        if let Some(a) = self.menubar(&ctx, &g, full, a_bar) {
            acts.push(a);
        }
        // Each window fades and lifts in when it opens.
        let kind = match self.dialog {
            Dialog::None => 0,
            Dialog::Help => 1,
            Dialog::Settings => 2,
            Dialog::Form(_) => 3,
            Dialog::Unsaved => 4,
        };
        let mut a_dlg = 0.0;
        for k in 1..5 {
            let a = ctx.animate_bool_with_time(Id::new("dlg").with(k), kind == k, 0.14);
            if k == kind {
                a_dlg = a;
            }
        }
        let d = match self.dialog {
            Dialog::Settings => self.settings_dialog(&ctx, &g, full, a_dlg),
            Dialog::Form(_) => self.form_dialog(&ctx, &g, full, a_dlg),
            Dialog::Help => self.help_dialog(&ctx, &g, full, a_dlg),
            Dialog::Unsaved => self.unsaved_dialog(&ctx, &g, full, a_dlg),
            Dialog::None => None,
        };
        if let Some(a) = d {
            acts.push(a);
        }
        if self.cid_edit.is_some() {
            self.cid_editor(&ctx, &g, now);
        }
        if self.key_edit.is_some() {
            self.key_editor(&ctx, &g, now);
        }
        // What the open windows and menus need, for image focus next frame.
        self.need = self.overlay_need(&ctx, &g, full);
        // No system frame: the edges resize, a hairline marks them. Not
        // while cropping: in focus the picture's edges are the window's, and
        // a press there belongs to the crop.
        if !crop_on {
            window::edges(&ctx, full);
        }
        window::hairline(&ctx, full);
        // A click anywhere else closes an open menu.
        if self.menu.is_some()
            && ctx.input(|i| i.pointer.primary_clicked())
            && !self.chrome_hover
            && acts.is_empty()
        {
            self.menu = None;
        }
        for a in acts {
            self.act(&ctx, a, now);
        }

        self.floating_trees(&ctx, now);
        self.dock_preview(&ctx);
        #[cfg(feature = "workspace-smoke")]
        self.smoke_step(&ctx, now);

        self.typed = ctx.memory(|m| m.focused().is_some());
        self.want();
        let anim = !self.fly.is_empty()
            || self.flash.as_ref().is_some_and(|f| now - f.t0 < 0.8)
            || self.landing.is_some_and(|(_, t0)| now - t0 < 0.7)
            || self.snaps.values().any(|s| now - s.t0 < 0.8)
            || !self.saver.idle();
        if anim {
            ctx.request_repaint();
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.s.files_root = self.files.browser.as_ref().map(|b| b.tree.root.clone());
        self.s.focus_budget = Some([self.focus.budget.x, self.focus.budget.y]);
        self.s.home = self.focus.home.map(|(h, m)| ([h.x, h.y], m));
        eframe::set_value(storage, "rusted-calamari", &self.s);
    }
}

#[cfg(test)]
mod tests {
    use super::plain;
    use std::path::PathBuf;

    #[test]
    fn verbatim_prefixes_come_off() {
        assert_eq!(
            plain(PathBuf::from(r"\\?\C:\Users\ryanf\Desktop\ruth-image")),
            PathBuf::from(r"C:\Users\ryanf\Desktop\ruth-image")
        );
        assert_eq!(
            plain(PathBuf::from(r"\\?\UNC\nas\photos")),
            PathBuf::from(r"\\nas\photos")
        );
        assert_eq!(
            plain(PathBuf::from("/home/me/photos")),
            PathBuf::from("/home/me/photos")
        );
    }
}
