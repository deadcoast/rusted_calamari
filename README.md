# Rusted Calamari

An image browser and organiser built in Rust, with a picture-first workspace
and two movable **Octobrachial Tree Format (OTF)** browsers. Sort images into
real folders with a keypress, browse the filesystem through branching CP437
text, and keep either tree beside the image or in its own native window.

| Workspace | Purpose |
| --- | --- |
| **Favourites** · upper tree | Your library and manually configured categories, with hotkeys for sorting images. |
| **Files** · lower tree | A full filesystem browser using the same OTF layout, folding and overflow pages. |
| **Image viewer** | Preview, zoom, crop, rotate and flip; fold the dock to let the window follow the image's shape. |

The interface appears on hover. Move to the top edge for menus, the bottom
edge for status and category keys, or a tree's header for its window controls.
**PIN_UI** keeps the interface visible. Press `H` for help or `.` for Settings.

[Get started](#get-started) · [Workspace](#the-two-tree-workspace) ·
[Files and Anchor](#files-and-anchor) · [Sorting](#sorting-and-categories) ·
[Controls](#controls) · [Development](#development)

## Get started

### Download for Windows

Download the Windows x64 ZIP from [GitHub Releases](https://github.com/deadcoast/rusted_calamari/releases/latest),
extract it, and run `rusted-calamari.exe`. The portable package includes the
app, documentation, icon assets and dependency notices. Rust is only needed
to build from source. A `.sha256` file accompanies each release ZIP.

### Run from source

Use **Rust 1.95 or newer** and your platform's native build tools. Windows
requires the MSVC C++ build tools, available through Visual Studio Build
Tools' **Desktop development with C++** workload. macOS requires the Xcode
command line tools. Linux needs the development libraries for its desktop
window system and an `xdg-desktop-portal` backend for file dialogs.

From this checkout:

```sh
rustup update stable
cargo run --release --locked
```

1. **Open a library.** Drop a folder onto the window or use `Ctrl+O`.
2. **Browse images.** Click the image area, then use `Left` / `Right`.
3. **Sort.** Press `z`, `x`, `c`, `v` or `Space` to move the current image into
   its configured category folder. `Ctrl+Z` undoes the move.
4. **Explore.** Use Files to browse another folder or drive. Turn on its
   Anchor if you want that browser to stay independent of app navigation.
5. **Arrange the workspace.** Drag a tree's hover header out to detach it,
   or press `Tab` to fold the dock and focus on the image.

The app remembers the last library. To choose one at launch:

```sh
cargo run --release --locked -- "D:\Photos"
```

### Install or build an executable

```sh
cargo install --path . --locked --bin rusted-calamari
rusted-calamari "D:\Photos"
```

The default Windows install location is
`%USERPROFILE%\.cargo\bin\rusted-calamari.exe`. Re-run the install command
from an updated checkout to update it; use `cargo uninstall rusted-calamari`
to uninstall.

To build without installing:

```sh
cargo build --release --locked --bin rusted-calamari
```

The Windows executable is `target/release/rusted-calamari.exe`. Release
builds open without a console window, and fonts and icons are embedded.
Windows has native verification evidence; macOS and Linux have not received
the same workspace verification. See the [verification report](docs/verification/workspace-2026-10-13.md).

## The two-tree workspace

Favourites and Files share the left dock, separated by a subtle horizontal
divider. Drag that divider to change their split. Detach either widget and
the other fills the dock; return it and the original upper/lower slots and
saved split are restored.

| Action | Control |
| --- | --- |
| Detach a tree | Drag its hover header out of the dock, or click `[>]`. |
| Dock a floating tree | Drag its header over the left dock and release, or click `[<]`. |
| Toggle a tree's placement | `Shift+click` its hover header or a header control. |
| Resize a floating tree | Drag its window edges or corners. |
| Close a floating tree | Returns it to the dock. |
| Fold/unfold the dock | `Tab` or the fold tab; floating trees remain usable. |
| Fold/unfold the image | `Shift+Tab`, while a tree is docked. |
| Resize the dock | Drag the vertical divider beside the image; double-click it to restore automatic width. |

The **View** menu also provides individual Dock/Detach actions and pair
Recall/Restore actions. Each tree retains its browsing state when moved,
and remembers its floating size and position separately from the dock split.

With **both trees detached**, the main window acts folded and gives the
image priority. **Triple-click the fold tab** to recall both trees; the next
triple-click restores their previous floating positions. This recall/restore
cycle starts only when both are detached. Triple-clicking an ordinary joined
or mixed layout does not detach the pair.

Dragging the image divider all the way left folds the dock; all the way
right folds the image. The edge that will fold tints as you approach it.

### Tree appearance

Choose **Octobrachial Arm**, **Octobrachial Beak** or **Boxtobrachial** through
View or Settings. Settings also provides twelve CP437 font faces and size
controls. Both browsers use the OTF folding, reflow and eight-arm overflow
system; click an `~[ink ...]` row to cycle additional pages.

The root is a plain steel-coloured circle with muted brackets and no
background highlight. Category connectors use bold single lines. Category
colours belong to their categories, and tier letters are configuration
metadata rather than extra subtitles in the tree.

## Files and Anchor

Files browses beyond the image library: folders, drives and other accessible
filesystem paths. Category names such as `_fav_` are ordinary directories
here. Listings include non-media and hidden files and load on demand in the
background.

The **Anchor** in the Files hover header controls folder following:

| Anchor | Behaviour |
| --- | --- |
| **Off** · default | Files follows successful folder changes in the app, including the hover menu's Open folder action, `Ctrl+O`, folder drops and Favourites navigation. |
| **On** · filled icon, `Files (anchored)` | Files keeps its own location while the app changes folders. You can still browse manually inside Files. |

Turning Anchor **off immediately synchronises** Files to the app's current
image folder. Its setting persists across launches; anchored Files restores
its saved independent path. Cancelled or failed folder opens do not move the
browser, and pending edits must be resolved before a library change takes
effect. Manual Files navigation stays where you leave it until another app
folder change occurs.

Anchor works docked or floating. It controls path following; **PIN_UI**
controls interface visibility, and the detach button controls placement.
`Shift+click` on the Anchor follows the header's dock/detach gesture; a normal
click toggles Anchor.

### Browsing and previewing

- **Folders:** click to expand or fold; double-click or press `Enter` to make
  the selected folder the browser root.
- **Media:** click to preview. Media within the current library keeps that
  library open; media outside it opens its parent as a new image library,
  with the usual unsaved-edit prompt.
- **Other files:** select to see their path and size. Files does not execute
  them or open them in external applications.
- **Links:** explicitly open one as a new browser root to follow it. Links
  are not expanded recursively.
- **A folder's images:** use the toolbar's **View images** button to open the
  selected folder as the image library. Browsing folders alone leaves the
  current image library in place.

Files' toolbar labels are clickable glyphs, **not bare-letter shortcuts**:

| Button | Action | Shortcut when Files has focus |
| --- | --- | --- |
| `<` / `>` | Back / Forward | `Alt+Left` / `Alt+Right` |
| `^` | Parent folder | `Alt+Up` or `Backspace` |
| `R` | Refresh selected folder | `F5` |
| `/` | Enter a path | `Ctrl+L` |
| `D` | Drives | — |
| `L` | Current library root | — |
| `V` | View selected folder's images | — |

With Files focused, arrows navigate the tree, `Home` / `End` and
`PageUp` / `PageDown` move selection, and `Enter` opens the selected folder
or advances an ink page. Sorting keys retain their meaning except while
entering text: pressing `v` still sorts into its category.

Click the image or Favourites to return image-navigation focus. Folding the
dock releases Files' navigation keys. After filesystem changes made outside
the app, refresh with `F5`. Unreadable folders show an error and can be
refreshed to retry.

## Sorting and categories

Favourites is the configured library/category tree. Click a folder to browse
it; click it again to fold it. Click a category slot to move the current image
there, or right-click a slot to browse that category.

Sorting **moves the actual file** into a category directory inside the folder
you are browsing, creating that directory on first use. The default category
table lives in `<library>/.calamari`:

```text
# key  name     cid      tier
spc    _float_  #E6D463  a
-      _fav_    #F0A35E  b
z      1        #e56399  c
x      2        #92CC7C  c
c      3        #7f96ff  c
v      4        #a6cfd5  c
```

`spc` means `Space`; `-` means no hotkey. `_fav_` has no default hotkey.
The `cid` is the colour used for a category's key, tree slot and file marks.
Tiers group category slots into adjacent rows of a beak, starting with `a`.

In **Settings**, click a key to rebind it; assigning an occupied key swaps
the two bindings. Double-click a colour to edit it live. Click the rest of a
row to change its name or tier, or remove the category definition.

The **File** menu can import/export category tables, export a CSV manifest,
and copy categorised images into an export folder. `Ctrl+I` imports media by
copying it into the current folder.

`Delete` moves an image into `<library>/.calamari_trash/`, the app's own trash
folder. Undo and redo cover sorting, trash and image edits within the current
library session; their history is cleared when opening another library and
is not saved across launches.

## Controls

These are the Windows bindings. Files takes the navigation keys described
above while it has focus; text fields and crop tools take their own input.

| Key | Action |
| --- | --- |
| `Left` / `Right` | Previous / next image |
| `Up` / `Down` | Back / forward 5 images |
| `[` / `]` | Forward / back 25 images |
| `;` / `'` | Forward / back 50 images |
| `z` / `x` / `c` / `v` / `Space` | Sort into the configured category |
| `Ctrl+Z` or `Backspace` | Undo (`Backspace` goes to the parent in Files) |
| `Ctrl+Shift+Z` or `Ctrl+Y` | Redo |
| `Ctrl+O` / `Ctrl+I` | Open a library / import media |
| `Ctrl+K` | Crop tool: drag a frame, `A` cycles ratios, `Enter` applies, `Esc` cancels |
| `Ctrl+Shift+K` | Toggle DANGER crop |
| `Ctrl+R` / `Ctrl+Shift+R` | Rotate clockwise / anticlockwise |
| `Ctrl+F` / `Ctrl+Shift+F` | Flip horizontally / vertically |
| `Ctrl+S` / `Ctrl+Shift+S` | Save current edits / export as |
| `Delete` | Move to the library's trash folder |
| `Tab` / `Shift+Tab` | Fold the dock / fold the image |
| `F11` | Full screen; in image focus, cycle Fit → Large → Full |
| `0` / `Ctrl+1` | Fit image / actual size |
| `H` / `.` | Help / Settings |
| `Alt+F` / `Alt+E` / `Alt+V` | File / Edit / View menu |
| `Esc` | Dismiss the active tool/menu/dialog, or leave full screen |
| `Ctrl+Q` | Quit |

Over the image, use the mouse wheel to zoom, drag to pan and double-click to
fit. The main window has no system title bar: drag empty space in its top
bar to move it, double-click to maximise, or use its window buttons. Edges
and corners resize it.

## Image focus and editing

Folding the dock turns the main window into the picture's frame. It follows
the image's aspect ratio while keeping its centre. Unpinned controls appear
over the picture on hover; PIN_UI reserves room for the visible interface.
Floating trees continue to operate alongside it.

The maximise button, `F11` or a double-click on the top bar cycles:

| Mode | Window behaviour |
| --- | --- |
| **Fit** | Wraps the picture within the size available when the dock was folded. |
| **Large** | Wraps it as large as the screen allows, excluding the taskbar. |
| **Full** | Fills the screen and centres the picture. |

The next press, or `Esc` from Full, returns to Fit. Resizing manually sets a
new Fit size; the window settles around the image when released. Help and
Settings can temporarily grow a narrow window. Unfolding the dock restores
the earlier main-window size.

Crop, rotate and flip preview as **pending edits**. `Ctrl+S` writes them to
the original; `Ctrl+Shift+S` exports to a chosen file without clearing the
original's pending edits. File also offers Save all and Revert. Changing
libraries or quitting with unsaved edits opens a save/discard/cancel prompt.

While cropping, hover controls move out of the way of the image and its
edges. **DANGER crop** (`Ctrl+Shift+K`) writes the crop immediately when you
release the dragged box. Hold `Shift` for a square. `Ctrl+Z` restores the
original bytes within the session. DANGER crop is disabled for GIFs.

### Media formats

JPG/JPEG, PNG, WebP, BMP, TIFF, ICO, TGA, QOI and PNM can be displayed and
edited. GIFs can be viewed and exported; overwriting a GIF through image
editing is disabled to preserve its frames.

HEIC, AVIF, JXL and video files (MP4, MOV, MKV, WebM, AVI, M4V) are recognised
for organisation and sorting, but appear as cards without a decoder/player.
The Files browser also lists files outside these media formats.

## Settings and recovery

| Data | Location |
| --- | --- |
| Windows preferences | `%APPDATA%\Rusted Calamari\data\app.ron` |
| macOS preferences | `~/Library/Application Support/Rusted-Calamari/app.ron` |
| Linux preferences | `$XDG_DATA_HOME/rustedcalamari/app.ron`, or `~/.local/share/rustedcalamari/app.ron` |
| Library category table | `<library>/.calamari` |
| Library trash | `<library>/.calamari_trash/` |

Preferences include the last library, tree appearance, dock split, floating
placements, Files location and Anchor state.

- **Files stopped following opened folders:** turn off its Anchor.
- **A tree is floating or missing from the dock:** use the View menu's
  Dock/Recall actions. With both detached, triple-click the fold tab.
- **Controls are hidden:** hover the top/bottom edge, or enable PIN_UI.
- **A directory listing is stale or unavailable:** refresh Files, navigate
  to a parent, or enter another accessible path.
- **Rust is older than 1.95:** run `rustup update stable` and rebuild.
- **Linux file dialogs do not appear:** install `xdg-desktop-portal` with a
  GTK or KDE backend.
- **Reset interface preferences:** close the app, rename `app.ron` to keep
  a backup, then relaunch. Library categories and media stay in their folders.

## Development

The application uses Rust 2024, egui/eframe and the `image` crate. The tree
engine is exposed as the **`octo`** library in this package.

```sh
cargo test --release --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo build --release --locked --bin rusted-calamari
```

Tests cover OTF layout and click fitting, filesystem fixtures and navigation,
widget placement and recall, Anchor following, settings persistence,
unsaved-edit transitions, image focus and file operations.

### Icon assets and release packaging

The application icon comes from the supplied transparent logo in
[`assets/logos/`](assets/logos/). The main window and floating trees share its
embedded PNG; Windows executable resources contain a nine-size ICO (16–256px)
and version information. To regenerate the converted assets:

```sh
cargo run --release --locked --example make_icons
```

On Windows with PowerShell 7, build a portable package with:

```powershell
pwsh -File scripts/package-release.ps1
```

The script builds for the installed Windows MSVC host, collects documentation
and dependency notices, and writes a ZIP, SHA-256 checksum and build metadata
under `target/packages/`. It refuses to overwrite earlier packages; pass
`-OutputDirectory` to select a fresh output folder. Generated archives and
executables stay out of Git and are attached to GitHub Releases.

[`scripts/verify-release.ps1`](scripts/verify-release.ps1) checks a built or
extracted Windows x64 executable's icon resources, version metadata and native
launch with an isolated profile. See the [v0.3.0 release verification](docs/verification/release-v0.3.0.md)
for the evidence and package verification steps.

### Native smoke run

For a native smoke run, choose a **fresh output directory**:

```sh
cargo run --release --locked --features workspace-smoke --bin calamari-workspace-smoke -- target/workspace-smoke-check
```

The runner creates isolated fixtures and preferences, drives application
actions, captures main-window screenshots and records native window geometry
and assertions before exiting. It does not inject physical mouse input.
Physical drag/click timing and mixed-DPI or multi-monitor behaviour remain
separate verification work; see the [dated evidence and limitations](docs/verification/workspace-2026-10-13.md).

### Code and design map

| Area | Entry points |
| --- | --- |
| Viewer, menus, shortcuts and edits | [`src/app.rs`](src/app.rs) |
| Docking, native widget hosts and Anchor | [`src/dock.rs`](src/dock.rs), [`src/workspace.rs`](src/workspace.rs) |
| Files UI and background listings | [`src/browserpane.rs`](src/browserpane.rs), [`src/fsbrowser.rs`](src/fsbrowser.rs) |
| OTF model, layout, click fitting and colour | [`src/tree.rs`](src/tree.rs), [`src/layout.rs`](src/layout.rs), [`src/tide.rs`](src/tide.rs), [`src/paint.rs`](src/paint.rs) |
| Native window and image focus | [`src/window.rs`](src/window.rs), [`src/focus.rs`](src/focus.rs) |
| File operations and image workers | [`src/ops.rs`](src/ops.rs), [`src/loader.rs`](src/loader.rs), [`src/saver.rs`](src/saver.rs) |
| Shared rendering and typography | [`src/theme.rs`](src/theme.rs), [`src/glyph.rs`](src/glyph.rs), [`src/fonts.rs`](src/fonts.rs), [`src/treeview.rs`](src/treeview.rs), [`src/toolbar.rs`](src/toolbar.rs) |
| Integration tests and native smoke | [`tests/`](tests/), [`src/smoke_main.rs`](src/smoke_main.rs) |

- [OTF specification](docs/OCTOBRACHIAL_TREE_FORMAT.md): tree grammar and layout rules.
- [Workspace behaviour](docs/UX-CONTRACT.md): interaction and state contracts.
- [Design system](docs/DESIGN.md): visual roles, tokens and their code owners.
- [Workspace verification](docs/verification/workspace-2026-10-13.md): results,
  evidence boundaries and outstanding native checks.

## Credits

- Fonts: [The Ultimate Oldschool PC Font Pack](https://int10h.org/oldschool-pc-fonts/)
  by VileR, CC BY-SA 4.0 ([licence](assets/fonts/LICENSE.TXT)).
- Icons: [Phosphor Icons](https://phosphoricons.com) (MIT), via
  [egui-phosphor](https://github.com/amPerl/egui-phosphor) (MIT/Apache-2.0).
- Built on [egui / eframe](https://github.com/emilk/egui).
