//! Filesystem widget content. The same instance is painted docked or in
//! a native viewport; scrolling, expansion and selection survive either.

use crate::{glyph::{Grid, run_at}, theme, treeview::Motion};
use eframe::egui::{self, Color32, CursorIcon, Id, Key, Modifiers, Pos2, Rect, Sense, Stroke, TextEdit, Vec2};
use octo::{fsbrowser::{Browser, EntryKind, Load, roots, size_label}, layout::{Ctx, Frame, Kind, Style}, paint, tide::{self, View}, tree::is_media};
use std::path::{Path, PathBuf};

pub enum Action { Preview(PathBuf), OpenFolder(PathBuf) }

pub struct Files {
    pub browser: Option<Browser>,
    view: View,
    motion: Motion,
    dx: f32,
    scroll_acc: f32,
    generation: u64,
    pub focused: bool,
    location: String,
    editing: bool,
    focus_address: bool,
    drives_open: bool,
    keyboard: Vec<Key>,
}

impl Default for Files {
    fn default() -> Self {
        Self { browser: None, view: View { rows: 20, scroll: 0 }, motion: Motion::default(), dx: 0.0, scroll_acc: 0.0,
            generation: 0, focused: false, location: String::new(), editing: false, focus_address: false,
            drives_open: false, keyboard: Vec::new() }
    }
}

/// Shared character-grid toolbar target. No layout or colour change on
/// press; the tooltip and accessible label describe the small glyph.
pub fn button(ui: &egui::Ui, g: &Grid, r: Rect, id: Id, glyph: &str, label: &str, enabled: bool) -> egui::Response {
    let resp = ui.interact(r, id, if enabled { Sense::CLICK } else { Sense::hover() });
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    if enabled && resp.hovered() {
        ui.painter().rect_filled(r, 2.0, theme::IRON_HI);
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    g.text(ui.painter(), r.center() - g.size(glyph.chars().count(), 1) * 0.5, glyph, if enabled { theme::TEXT } else { theme::DIM }, 1.0);
    resp.on_hover_text(label)
}

impl Files {
    pub fn start(&mut self, ctx: &egui::Context, path: PathBuf) {
        if let Some(browser) = &mut self.browser { browser.navigate(path); }
        else {
            let ctx = ctx.clone();
            self.browser = Some(Browser::new(path, move || ctx.request_repaint()));
        }
    }
    pub fn poll(&mut self) {
        if let Some(b) = &mut self.browser { b.poll(); }
    }
    pub fn refresh(&mut self) {
        if let Some(b) = &mut self.browser {
            let id = if b.entries[b.selected].kind == EntryKind::Directory { b.selected } else { b.tree.nodes[b.selected].parent.unwrap_or(0) };
            b.load(id);
        }
    }
    pub fn refresh_open(&mut self) { if let Some(b) = &mut self.browser { b.refresh_open(); } }
    /// Consume only file-navigation keys before the viewer sees them.
    /// Sorting hotkeys and Tab keep their established application meaning.
    pub fn keys(&mut self, ctx: &egui::Context) {
        if !self.focused { return; }
        if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::L)) {
            self.location = self.browser.as_ref().map_or(String::new(), |b| b.tree.root.display().to_string());
            self.editing = true;
            self.focus_address = true;
        }
        if self.editing || ctx.egui_wants_keyboard_input() { return; }
        if let Some(b) = &mut self.browser {
            if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowLeft)) { b.go_back(); }
            if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowRight)) { b.go_forward(); }
            if ctx.input_mut(|i| i.consume_key(Modifiers::ALT, Key::ArrowUp)) { b.go_up(); }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5)) { self.refresh(); }
        for key in [Key::ArrowUp, Key::ArrowDown, Key::ArrowLeft, Key::ArrowRight, Key::Enter, Key::Home, Key::End, Key::PageUp, Key::PageDown, Key::Backspace] {
            if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, key)) { self.keyboard.push(key); }
        }
    }

    fn toggle(&mut self, id: usize, ink: bool, style: Style, cx: &Ctx) {
        let Some(b) = &mut self.browser else { return };
        if ink { tide::click(&mut b.tree, &mut self.view, style, cx, id, true); return; }
        if b.entries[id].kind != EntryKind::Directory { return; }
        match b.entries[id].load {
            Load::Unloaded | Load::Error(_) => { b.tree.nodes[id].open = true; b.load(id); }
            _ => { tide::click(&mut b.tree, &mut self.view, style, cx, id, false); }
        }
    }

    fn activate(&mut self, id: usize) -> Option<Action> {
        let b = self.browser.as_mut()?;
        let entry = &b.entries[id];
        match entry.kind {
            EntryKind::Directory => { let p = entry.path.clone(); b.navigate(p); None }
            EntryKind::File if is_media(&b.tree.nodes[id].name) => Some(Action::Preview(entry.path.clone())),
            // A link is followed only by explicit navigation. Making it a
            // new root prevents recursive traversal of junction/link cycles.
            EntryKind::Link => { let p = entry.path.clone(); b.navigate(p); None }
            _ => None,
        }
    }

    fn keyboard_actions(&mut self, frame: &Frame, cx: &Ctx, style: Style) -> Option<Action> {
        let mut action = None;
        for key in std::mem::take(&mut self.keyboard) {
            let b = self.browser.as_mut()?;
            let generation = b.generation;
            let visible: Vec<usize> = frame.lines.iter().flat_map(|l| l.heads.iter().copied()).collect();
            let index = visible.iter().position(|&n| n == b.selected).unwrap_or(0);
            let mut selected = b.selected;
            match key {
                Key::ArrowDown => selected = visible[(index + 1).min(visible.len().saturating_sub(1))],
                Key::ArrowUp => selected = visible[index.saturating_sub(1)],
                Key::Home => selected = visible[0],
                Key::End => selected = *visible.last().unwrap_or(&0),
                Key::PageDown => self.view.scroll = (self.view.scroll + self.view.rows).min(frame.lines.len().saturating_sub(self.view.rows)),
                Key::PageUp => self.view.scroll = self.view.scroll.saturating_sub(self.view.rows),
                Key::Backspace => b.go_up(),
                Key::ArrowLeft => {
                    if b.tree.nodes[selected].open && selected != 0 { b.tree.nodes[selected].open = false; }
                    else { selected = b.tree.nodes[selected].parent.unwrap_or(0); }
                }
                Key::ArrowRight => {
                    if b.tree.nodes[selected].open && !b.tree.nodes[selected].kids.is_empty() { selected = b.tree.nodes[selected].kids[0]; }
                    else { self.toggle(selected, false, style, cx); }
                }
                Key::Enter => action = self.activate(selected),
                _ => {}
            }
            let b = self.browser.as_mut()?;
            if b.generation != generation { break; }
            if selected < b.entries.len() { b.selected = selected; }
            if let Some(row) = frame.row(selected) {
                if row < self.view.scroll { self.view.scroll = row; }
                else if row >= self.view.scroll + self.view.rows { self.view.scroll = row + 1 - self.view.rows; }
            }
        }
        action
    }

    pub fn show(&mut self, ui: &mut egui::Ui, rect: Rect, g: &Grid, style: Style, library: Option<&Path>, now: f64) -> Option<Action> {
        let ctx = ui.ctx().clone();
        let mut action = None;
        if self.browser.is_none() { return None; }
        let generation = self.browser.as_ref().unwrap().generation;
        if self.generation != generation {
            self.generation = generation;
            self.motion.reset(); self.view.scroll = 0; self.dx = 0.0;
        }
        let nav = Rect::from_min_size(rect.min + Vec2::new(g.cw, 0.0), Vec2::new((rect.width() - 2.0 * g.cw).max(0.0), g.rh * 1.3));
        if self.editing {
            let edit = ui.put(nav, TextEdit::singleline(&mut self.location).font(g.font.clone()).hint_text("Folder path").id(Id::new("files-address")));
            if self.focus_address { edit.request_focus(); self.focus_address = false; }
            if ctx.input(|i| i.key_pressed(Key::Escape)) { self.editing = false; edit.surrender_focus(); }
            if ctx.input(|i| i.key_pressed(Key::Enter)) {
                let path = PathBuf::from(self.location.trim());
                if !path.as_os_str().is_empty() {
                    let b = self.browser.as_mut().unwrap();
                    let path = if path.is_absolute() { path } else { b.tree.root.join(path) };
                    b.navigate(path);
                }
                self.editing = false; edit.surrender_focus();
            }
        } else {
            let b = self.browser.as_ref().unwrap();
            let items = [
                ("<", "Back (Alt+Left)", !b.back.is_empty()),
                (">", "Forward (Alt+Right)", !b.forward.is_empty()),
                ("^", "Parent folder (Alt+Up)", b.tree.root.parent().is_some()),
                ("R", "Refresh selected folder (F5)", true),
                ("/", "Enter folder path (Ctrl+L)", true),
                ("D", "Drives", true),
                ("L", "Current library", library.is_some()),
                ("V", "View selected folder's images", b.entries[b.selected].kind == EntryKind::Directory),
            ];
            let width = (nav.width() / items.len() as f32).min(g.cw * 3.0);
            let mut clicked = None;
            for (i, (glyph, label, enabled)) in items.into_iter().enumerate() {
                let r = Rect::from_min_size(nav.min + Vec2::new(width * i as f32, 0.0), Vec2::new(width, nav.height()));
                if button(ui, g, r, Id::new("file-nav").with(i), glyph, label, enabled).clicked() && enabled { clicked = Some(i); }
            }
            let b = self.browser.as_mut().unwrap();
            if let Some(i) = clicked {
                self.focused = true;
                match i {
                    0 => b.go_back(), 1 => b.go_forward(), 2 => b.go_up(),
                    3 => self.refresh(),
                    4 => { self.location = b.tree.root.display().to_string(); self.editing = true; self.focus_address = true; }
                    5 => self.drives_open = !self.drives_open,
                    6 => if let Some(p) = library { b.navigate(p.to_path_buf()); },
                    7 => action = Some(Action::OpenFolder(b.entries[b.selected].path.clone())),
                    _ => {}
                }
            }
        }
        if self.drives_open {
            let paths = roots();
            let items: Vec<_> = paths.iter().enumerate().map(|(i, p)| theme::Item::Act { label: p.display().to_string(), key: String::new(), act: i, on: true }).collect();
            let out = theme::dropdown(&ctx, g, "file-drives", nav.left_bottom(), &items, 1.0);
            if let Some(i) = out.click { self.browser.as_mut().unwrap().navigate(paths[i].clone()); self.drives_open = false; }
            else if ctx.input(|i| i.key_pressed(Key::Escape) || i.pointer.primary_clicked()) && !out.hovered { self.drives_open = false; }
        }
        let tree_rect = Rect::from_min_max(Pos2::new(rect.left(), nav.bottom() + g.rh * 0.3), Pos2::new(rect.right(), (rect.bottom() - g.rh * 1.6).max(nav.bottom())));
        let rows = (tree_rect.height() / g.rh).floor().max(1.0) as usize;
        self.view.rows = rows;
        let cols = ((tree_rect.width() - g.cw * 2.0) / g.cw).floor().max(8.0) as usize;
        let (frame, cx) = self.browser.as_ref().unwrap().frame(style, cols);
        action = self.keyboard_actions(&frame, &cx, style).or(action);
        // Navigation may have replaced the arena while handling Enter/Back.
        if self.browser.as_ref().unwrap().generation != generation { ctx.request_repaint(); return action; }
        let b = self.browser.as_ref().unwrap();
        self.view.scroll = self.view.scroll.min(frame.lines.len().saturating_sub(rows));
        let resp = ui.interact(tree_rect, Id::new("file-tree"), Sense::CLICK | Sense::DRAG);
        if resp.hovered() {
            self.dx += ctx.input(|i| i.smooth_scroll_delta.x);
            self.scroll_acc += ctx.input(|i| i.smooth_scroll_delta.y);
            while self.scroll_acc.abs() >= g.rh {
                if self.scroll_acc > 0.0 { self.view.scroll = self.view.scroll.saturating_sub(1); self.scroll_acc -= g.rh; }
                else { self.view.scroll = (self.view.scroll + 1).min(frame.lines.len().saturating_sub(rows)); self.scroll_acc += g.rh; }
            }
        }
        let slack = (frame.width() as f32 * g.cw + g.cw * 2.0 - tree_rect.width()).max(0.0);
        self.dx = (self.dx + resp.drag_delta().x).clamp(-slack, 0.0);
        self.motion.step(&b.tree, &frame, self.view.scroll, rows, now);
        if !self.motion.settled { ctx.request_repaint(); }
        let p = ui.painter().with_clip_rect(tree_rect);
        let origin = Pos2::new(tree_rect.left() + g.cw + self.dx, tree_rect.top());
        let mut hit = None;
        for d in self.motion.visible(&frame, rows) {
            let line = match (d.idx, d.ghost) { (Some(i), _) => &frame.lines[i], (_, Some(i)) => &self.motion.ghosts[i].line, _ => continue };
            let y = origin.y + d.y * g.rh;
            if d.idx.is_some() && let Some(q) = resp.hover_pos().filter(|q| q.y >= y && q.y < y + g.rh) {
                p.rect_filled(Rect::from_min_size(Pos2::new(tree_rect.left(), y), Vec2::new(tree_rect.width(), g.rh)), 0.0, Color32::from_white_alpha(6));
                if let Some(si) = run_at(&line.spans, |s| s.text.chars().count(), g.col(origin.x, q.x))
                    && let Some(id) = line.spans[si].node {
                    hit = Some((id, line.kind == Kind::Ink));
                    ctx.set_cursor_icon(CursorIcon::PointingHand);
                }
            }
            let runs: Vec<_> = line.spans.iter().map(|s| {
                let c = paint::colour(&b.tree, s.cls).0;
                (s.text.clone(), Color32::from_rgb(c.0, c.1, c.2))
            }).collect();
            g.line(&p, Pos2::new(origin.x, y), &runs, d.a);
        }
        if let Some((id, ink)) = hit && (resp.clicked() || resp.double_clicked()) {
            self.focused = true;
            self.browser.as_mut().unwrap().selected = id;
            if ink { self.toggle(id, true, style, &cx); }
            else if resp.double_clicked() { action = self.activate(id).or(action); }
            else if self.browser.as_ref().unwrap().entries[id].kind == EntryKind::Directory { self.toggle(id, false, style, &cx); }
            else if self.browser.as_ref().unwrap().entries[id].kind == EntryKind::File { action = self.activate(id).or(action); }
        }
        let b = self.browser.as_ref().unwrap();
        let selected = &b.entries[b.selected];
        let footer = Rect::from_min_max(Pos2::new(rect.left() + g.cw, rect.bottom() - g.rh * 1.4), rect.max - Vec2::new(g.cw, 0.0));
        let p = ui.painter().with_clip_rect(footer);
        p.line_segment([footer.left_top(), footer.right_top()], Stroke::new(1.0, theme::EDGE.gamma_multiply(0.3)));
        let text = match &selected.load {
            Load::Error(e) => format!("Can't read folder: {e}. R to retry."),
            _ if selected.kind == EntryKind::File => format!("{}  {}", selected.path.display(), size_label(selected.bytes)),
            _ => selected.path.display().to_string(),
        };
        g.text(&p, footer.min + Vec2::new(0.0, g.rh * 0.2), &text, theme::MUTED, 1.0);
        ui.interact(footer, Id::new("file-details"), Sense::hover()).on_hover_text(text);
        action
    }
}
