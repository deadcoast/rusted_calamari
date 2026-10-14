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
   with the fold tab just under it, or to the bottom edge for the status
   line. `H` lists every key and `.` opens Settings.
5. **Focus on the pictures.** `Tab` folds the tree away, and the window
   closes round the image (see [Image focus](#image-focus)). `Tab` again
   brings the tree back.

To keep the bars on screen, turn on **PIN_UI** in Settings or choose View →
Pin the interface.

## Two Octobrachial trees

**Favourites**, above, keeps the existing library/category tree and sorting
hotkeys. **Files**, below, browses the full filesystem using the same OTF
arms, folding and overflow pages. By default it follows the app's current
image folder, including folders opened from the hover menu and Favourites.
Category names such as `_fav_` are ordinary folders in Files.

Click the **Anchor** beside the Files header's detach button to keep Files
at its own location when the app changes folders. A filled anchor and
`Files (anchored)` show that syncing is paused. Click it again to immediately
follow the app's current folder. The setting persists across launches and
works in both docked and detached windows. Anchored Files restores its own
saved location. Cancelled or failed folder opens never change its location.

Hover either widget's header and drag it out to make a separate native
window. The remaining widget fills the dock. Drag the detached header back
over the left dock and release to snap it into its original slot. The
subtle horizontal divider changes the joined split. Detached windows can
be resized from their edges.

Shift-click a widget's hover header to toggle docked/floating placement;
the header's `[>]` / `[<]` button and View menu offer the same action.
Closing a detached widget docks it. Each widget remembers its last
detached size and position separately from the joined split.

`Tab` and the fold tab still fold the dock and give the image priority.
Detached widgets stay available. With both detached, the main window acts
folded: **triple-click the fold tab** to recall both, then triple-click again
to restore their detached positions. The gesture cannot detach an ordinary
joined or mixed layout.

In **Files**:

- Click a folder to expand/fold it; double-click or press `Enter` to make it
  the browsing root. Click media to preview it. Other files show details
  and are never launched. Click the `~[ink ...]` row to cycle overflow pages.
- The toolbar offers Back, Forward, Parent, Refresh, Path, Drives, Library
  and View images. Hover a glyph for its label and shortcut. `V` in this
  toolbar opens the selected folder as the image library.
- When Files has focus, arrows navigate its tree; `Home`/`End` and
  `PageUp`/`PageDown` move selection. `Enter` on the ink row advances its
  page. `Backspace` or `Alt+Up` goes to the parent; `Alt+Left`/`Alt+Right`
  traverse history, `Ctrl+L` edits the path and `F5` refreshes.
- Folder browsing leaves the image library in place. Previewing media
  outside it opens that media's parent as a library; pending image edits
  use Save/Open without saving/Cancel. Sorting hotkeys keep their existing
  meaning except while typing a path.

Folder listings load on demand in the background. Unreadable folders show
an error and can be refreshed. Links are followed only when explicitly
opened as a new browsing root. Refresh after changes made outside the app.

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
| `Tab` · `Shift+Tab` | Fold the tree (image focus) · fold the image (tree only) |
| `F11` | Full screen; in image focus, step Fit → Large → Full |
| `0` · `Ctrl+1` | Fit · actual size |
| Mouse wheel · drag · double-click | Zoom · pan · fit |
| `Ctrl+O` · `Ctrl+I` | Open a folder · import images into the current folder |
| `H` · `.` · `Esc` · `Alt+F/E/V` | Keys · Settings · close (or leave full screen) · open a menu |

In the tree: click a folder to browse it (click it again to fold it). Click a
category slot to move the image there. Right-click a slot to browse what's
in it.

Drag the divider to set the tree's width. Drag it all the way left to fold
the tree, or all the way right to fold the image; the edge that will fold
tints while you're past it.

The window has no system title bar. Drag the top bar's empty space to move
the window, double-click it to maximize, and drag any edge or corner to
resize. Minimize, maximize and close sit at the right end of the top bar.

## Image focus

Fold the tree (`Tab`, the `<` tab, or drag the divider all the way left)
and the window becomes the picture's frame. It closes round the image edge
to edge, with no bars: a portrait picture gets a portrait window. As you
browse, it takes each image's shape and keeps its centre. The menu bar,
status line and toolbar still come up on hover, over the picture.

The maximize button (or `F11`, or a double-click on the top bar) steps
through three sizes:

| Size | The window |
| --- | --- |
| **Fit** | Wraps the picture at the size the window had when you folded the tree |
| **Large** | Wraps it as large as the screen allows, taskbar excluded |
| **Full** | Full screen, picture centred, bars only where its shape differs from the screen's |

One more press (or `Esc` in full screen) goes back to Fit. Resizing the
window by hand sets a new size for Fit; it settles round the picture as
soon as you let go. Help, Settings and other windows that need more room
than a narrow picture gives grow the window while they're open. Unfolding
the tree gives the window back its earlier size.

## Categories

Categories live in `<library>/.calamari`, one per line:

```calamari
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

While either crop tool is on, the menu bar, status line, toolbar and fold
tab stay out of the way whenever the pointer is on the picture, so every
edge can be grabbed, even in image focus. The picture is never shrunk to
make room. The crop bar moves off the frame's edge and lets the pointer
through when you reach for an edge under it.

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

The tests cover the tree layout and click engine, filesystem loading and
keyboard paging, docking/recall gestures, settings compatibility, unsaved
edits, file operations, image focus sizing and toolbar layout.

For an isolated native-window smoke run with screenshots, use a fresh
output directory:

```sh
cargo run --release --features workspace-smoke --bin calamari-workspace-smoke -- target/workspace-smoke-check
```

This exercises actual native placement and rendering through application
actions. It does not inject physical mouse input. The current evidence and
remaining native-input checks are in
[`docs/verification/workspace-2026-10-13.md`](docs/verification/workspace-2026-10-13.md).

```txt
src/
  main.rs       window setup (borderless)
  app.rs        the viewer: stage, tree pane, menus, dialogs, keys, undo/redo
  dock.rs       widget placement, split and gated pair recall/restore
  workspace.rs  dock/native hosts, dragging and shared library actions
  browserpane.rs Files widget controls, navigation and OTF presentation
  focus.rs      image focus: the window shaped round the picture
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
  fsbrowser.rs  lazy read-only filesystem tree and background listings
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
