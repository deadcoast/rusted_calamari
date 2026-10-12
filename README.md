# Rusted Calamari

A retro image browser and organizer, written in Rust. The image fills the
window. Beside it, the **Octobrachial Tree Format (OTF)** draws your folders
and categories as a CP437 (the IBM PC's original character set) tree, much as
Graph View draws an Obsidian vault. One key moves the image in view into a
category folder. Every other control stays hidden until the pointer goes
looking for it.

Spec for the tree and the interface: [`OCTOBRACHIAL_TREE_FORMAT.md`](OCTOBRACHIAL_TREE_FORMAT.md).

---

## Quickstart

```sh
rustup update stable
cd rusted-calamari
cargo run --release
```

Drop a folder onto the window (or press `Ctrl+O`), then use `←` `→` to
browse. The first build downloads and compiles the dependencies, which takes
a few minutes; later builds take seconds. The last folder you opened reopens
on the next launch.

To open a specific folder at launch:

```sh
cargo run --release -- "D:\Photos"
```

## Quick install

```sh
cargo install --path .
```

This puts `rusted-calamari` in `~/.cargo/bin`, which rustup already adds to
your PATH. Run it from anywhere:

```sh
rusted-calamari
rusted-calamari "D:\Photos"
```

On Windows the executable is `%USERPROFILE%\.cargo\bin\rusted-calamari.exe`.
Right-click it to pin it to Start or the taskbar. Release builds open without
a console window. The fonts and icons are compiled into the executable, so
it also runs from anywhere you copy it.

To update, run `cargo install --path .` again from the repo. To remove it, run
`cargo uninstall rusted-calamari`.

## Requirements

| | |
| --- | --- |
| Rust | 1.95 or newer (egui 0.36 needs it). Check with `rustc --version`. |
| Windows | The MSVC C++ build tools. The rustup installer offers them; otherwise install Visual Studio Build Tools with "Desktop development with C++". |
| macOS | The Xcode command line tools: `xcode-select --install`. |
| Linux | `sudo apt install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev` (or your distro's equivalents), plus `xdg-desktop-portal` with a GTK or KDE backend for the open and save dialogs. |

---

## The first minute

1. **Open a folder.** Drop it on the window, or press `Ctrl+O`. The folder
   becomes the library: the tree's root.
2. **Browse.** `←` `→` step one image, `↑` `↓` five, `[` `]` twenty-five,
   `;` `'` fifty.
3. **File an image.** `z` `x` `c` `v` and `Space` move the image in view into
   a category folder inside the folder you're browsing (created on first
   use). The image drops toward its key on the status line and the next one
   comes up. `Ctrl+Z` undoes the move.
4. **Find the controls.** Move the pointer to the top edge for the menu bar,
   or to the bottom edge for the status line. Move it near the tree's edge
   for the fold tab. `H` lists every key, `.` opens Settings, and `Tab` folds
   the tree away so the image gets the whole window.

To keep the bars on screen, turn on **PIN_UI** in Settings or choose View →
Pin the interface.

## Keys

| Key | Does |
| --- | --- |
| `←` `→` · `↑` `↓` · `[` `]` · `;` `'` | Back/on 1 · 5 · 25 · 50 |
| `z` `x` `c` `v` `Space` | Move the image into a category (rebind in Settings) |
| `Ctrl+Z` / `Backspace` · `Ctrl+Shift+Z` / `Ctrl+Y` | Undo · redo (moves, trash, saves, crops, edits) |
| `Ctrl+K` | Crop: drag a frame, `A` cycles ratios, `Enter` applies, `Esc` cancels |
| `Ctrl+Shift+K` | DANGER crop on/off: drag a box, let go, saved (see below) |
| `Ctrl+R` · `Ctrl+F` | Rotate right · flip horizontal (add `Shift`: rotate left · flip vertical) |
| `Ctrl+S` · `Ctrl+Shift+S` | Save edits into the file · export as a new file |
| `Del` | Move to `<library>/.calamari_trash/` |
| `Tab` · `0` · `Ctrl+1` · `F11` | Fold the tree · fit · actual size · full screen |
| Mouse wheel · drag · double-click | Zoom · pan · fit |
| `Ctrl+O` · `Ctrl+I` | Open a folder · import images into the current folder |
| `H` · `.` · `Esc` · `Alt+F/E/V` | Keys · Settings · close · open a menu |

In the tree: click a folder to browse it (click it again to fold it). Click a
category slot to move the image there. Right-click a slot to browse what's
in it.

The window has no system title bar. Drag the top bar's empty space to move
the window, double-click it to maximize, and drag any edge or corner to
resize. Minimize, maximize and close sit at the right end of the top bar.

## Categories

Categories live in `<library>/.calamari`, one per line:

```ps1
# key  name     cid      tier
spc    _float_  #E6D463  a
-      _fav_    #F0A35E  b
z      1        #e56399  c
x      2        #92CC7C  c
c      3        #7f96ff  c
v      4        #a6cfd5  c
```

The `cid` is the category's colour, used for its key, its slot in the tree
and its files' marks. Tiers `a`, `b`, `c`… are adjacent rows of the same
beak (the fan of category slots off a folder), with `a` on top. In Settings:

- Click a key to rebind it: press the new key and it's saved. A key that
  another category already has swaps between the two.
- Double-click a colour to edit it live.
- Click the rest of a row to rename the category, change its tier or delete
  it.

File → Import/Export categories moves the table between libraries.

## Editing

Crop, rotate and flip stay pending until you save (`Ctrl+S`) or export
(`Ctrl+Shift+S`), so they can be undone step by step before anything
touches the file.

**DANGER crop** is for speed. Turn it on (`Ctrl+Shift+K`, or the toolbar
from View → Show the toolbar), then hold the mouse button on the image,
drag a box and let go. The file is cropped and written immediately, with no
confirmation, while you move on to the next image. Shift keeps the box
square. `Ctrl+Z` puts the original file back exactly. It is off for GIFs,
which would lose their frames.

## Where things live

| What | Where |
| --- | --- |
| Settings and last library | Windows `%APPDATA%\Rusted Calamari\data\app.ron` · macOS `~/Library/Application Support/Rusted-Calamari/` · Linux `~/.local/share/rustedcalamari/app.ron` |
| Category table | `<library>/.calamari` |
| Trash | `<library>/.calamari_trash/` |

Delete `app.ron` to reset the interface to its defaults.

## Formats

Shown and editable: JPG, PNG, WebP, BMP, TIFF, ICO, TGA, QOI, PNM, and GIF
(editable through Export as only). HEIC, AVIF, JXL and video files are listed
and can be moved into categories, but they show as a card because there is
no decoder for them yet.

## Troubleshooting

- **`rustc 1.90.0 is not supported` (or any version under 1.95):** run
  `rustup update stable`, then build again.
- **Linux: the open/save dialogs don't appear:** install
  `xdg-desktop-portal` plus `xdg-desktop-portal-gtk` or
  `xdg-desktop-portal-kde`.
- **The interface looks wrong after a change:** delete `app.ron` (above) and
  relaunch.

## Development

```sh
cargo test --release
```

The tests cover the tree layout and click engine, file operations
(move/undo, rotate/restore, export) and the toolbar layout.

```txt
src/
  main.rs       window setup (borderless)
  app.rs        the viewer: stage, tree pane, menus, dialogs, keys, undo/redo
  theme.rs      shared frames, menus, dialogs
  toolbar.rs    the tool strip: registry, layout, animated buttons
  window.rs     title-bar jobs: drag, resize bands, window buttons
  treeview.rs   tree row motion and eased scrolling
  saver.rs      background writes for DANGER crop
  loader.rs     background image decoding
  fonts.rs      the twelve int10h faces, Phosphor icons
  glyph.rs      character-grid drawing

  lib.rs        the OTF library, `octo`, used by the app and the tests:
  tree.rs       folder tree and category table
  layout.rs     tree -> glyph lines, reflow, row keys
  tide.rs       click fitting: mirror, slide, retract
  paint.rs      token colours
  config.rs     the .calamari file
  ops.rs        disk operations
tests/          otf.rs, sorter.rs
assets/fonts/   Px437 faces
```

## Credits

- Fonts: [The Ultimate Oldschool PC Font Pack](https://int10h.org/oldschool-pc-fonts/)
  by VileR, CC BY-SA 4.0 (`assets/fonts/LICENSE.TXT`).
- Icons: [Phosphor Icons](https://phosphoricons.com) (MIT), via
  [egui-phosphor](https://github.com/amPerl/egui-phosphor) (MIT/Apache-2.0).
- Built on [egui / eframe](https://github.com/emilk/egui).
