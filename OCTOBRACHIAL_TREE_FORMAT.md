# Octobrachial Tree Format (OTF)

The directory display format of Rusted Calamari. Graph View draws Obsidian's
links; OTF draws Rusted Calamari's library. The tree and the image viewer
share one state: a keystroke that files the image in view into `1` changes
a glyph in the tree, and clicking a node in the tree changes which folder
the viewer browses. Neither renders the other.

Everything in this document beyond the request's own terms (`hier`, `cid`,
`_float_`, `_fav_`, the three style names, mirrored output, hotkeys) is a
design addition and is marked **[new]** where it first appears.

---

## 1. Anatomy

```txt
                                   ┌─ [_float_:6]                  ← beak, tier a
○ [photos] ·············· 14 Σ259 ─┼─ [_fav_:3]                     ← beak, tier b
│                                  └─ [1:9, 2:4, 3:7, 4:2]          ← beak, tier c
├─ [scans] ►12                                                        ← folded arm
├─ [screens] ························ 31                              ← arm + suckers
└─ [trips]
   ├─ [2024]
   │  │  ┌─ [tokyo] ········ 8                                        ← mirrored arms
   │  │  ├─ [osaka] ························ 40
   │  │  ├─ [kyoto] ····· 5
   │  └─ [japan] ··········· 11 ─┬─ [_float_:2]                     ← beak hangs down
   │                             └─ [1:5, 2:3, 3:1]
   └─ [2025] ►1
```

| Part | What it is |
| --- | --- |
| **Mantle** `○` **[new]** | The root of the open library. A plain circle with no filled box. Carries its loose files and `Σ` total. Its arms split above and below it so the body sits mid-screen. |
| **Arm** | A plain subdirectory, hung on a spine. |
| **Beak** | The category directories of a node (`_float_`, `_fav_`, `1`..`8`), collapsed into **one row per hier tier**: `[1:9, 2:4, 3:7]`. This is the request's `[img]->[cata]->[hier]` fan made literal: the beak is the set of slots the image in view can be filed into. Categories never appear as arms. |
| **Suckers** **[new]** | One cell per media file on the arm's own row. `·` = loose (in no category), `■` in the category's cid = filed. Over 10 files the row becomes a proportional 10-cell bar (shorter when the tree reflows, see 4b). A directory's sucker row shows at a glance how much of it is filed. |
| **Ink** **[new]** | `~[ink +5] 1/2 ►`. A node shows at most eight things: the beak counts as one, arms fill the rest, overflow pages through the ink row. Clicking ink turns the page. |
| **Fold mark** | `►12` on a closed node: how many children it holds. |

Hier tiers are adjacent, never nested: `a`, `b`, `c` are rows of the same
beak, ordered top to bottom. Tier letters stay in Settings and configuration;
only the category names and counts appear in the tree.

## 2. Tokens and colour

OTF is coloured like source code. Each glyph run is a typed token:

| Token | Colour rule |
| --- | --- |
| `Trunk(depth)` | Spine and connector glyphs cycle a 6-colour ramp by depth, the way rainbow-bracket highlighting colours nesting. The octopus's arms read by colour before you read a name. |
| `CatName(cat)` | The category's cid, bold. |
| `Sucker(cat)` | The category's cid; loose files dim. |
| `Brack`, `Count` | Muted, like punctuation; brackets keep this colour beside an active folder name. |
| `Name` | Primary text. |
| `BeakTrunk(depth)` | The spine colour, with bold single rules for stacked categories. |
| `Mantle`, `Active` | Cool steel blue (`#A9BCD0`), distinct from orange `_fav_` slots. |
| `Cursor` | Muted position indicator. |
| `Ink`, `Fold` | Fixed accents. |

The renderer emits `Line { spans: [Span { cls, text, node }], tags, heads }`.
The window paints spans on a monospace glyph grid in whichever of the
twelve int10h faces is selected, and a click resolves to (row, column) →
span → node, which is how a slot token knows it was hit.

## 3. Mirroring

A spine can hang its arms below the label (`├─ … └─`) or above it,
mirrored: the first arm stays nearest the label, the farthest arm goes on
top and takes the `┌─` corner.

```txt
├─ [2024]                    │  ┌─ [tokyo]
│  ├─ [kyoto]                │  ├─ [osaka]
│  ├─ [osaka]       ⇄        │  ├─ [kyoto]
│  └─ [tokyo]                └─ [japan]
```

The beak flexes separately. Its label pins to the middle tier (`Mid`), to
the last tier with the tiers folded above it (`Up`), or to the first tier
with the tiers hanging below (`Down`). `a` is always the top row of a
beak, so folding `Up` is the request's "hier(a) folds upward" rule.

## 4. The tide (on-click fit) **[new name]**

A click never moves the row under the cursor unless nothing else works.
Opening a node runs this ladder; each step is one render of the visible
tree, so a click costs a handful of linear passes over on-screen rows.

1. **FIT** — preferred arm side, beak `Mid`. The preferred side is down,
   unless the node itself hangs above its parent, in which case it keeps
   growing up.
2. **MIRROR** — the subtree overran the viewport (a snag). Try beak `Up`,
   beak `Down`, then flip the arms to the other side and try the three
   beak folds again. First shape that fits at the pinned row wins.
3. **SLIDE** — no shape fits at the pinned row but the subtree is shorter
   than the viewport: take the shape needing the smallest shift and move
   the row that many lines.
4. **RETRACT** — the subtree is taller than the viewport. Close the
   node's own open descendants, deepest first, re-running steps 1–3 after
   each. (A node remembers its open arms when folded; reopening it is
   where this fires.)
5. **CLIPPED** — a single level is taller than the viewport. Open toward
   the larger room and push the farthest arms off-screen.

Folding a node or paging ink pulls hidden rows back into any blank space
left at the bottom.

## 4b. Reflow and motion **[new]**

The tree is drawn to the width its pane has. When the widest row no longer
fits (the divider is dragged in, the window narrows), it steps down a ladder
and takes the first shape that fits:

| Level | Change |
| --- | --- |
| 0 | Full detail. |
| 1, 2 | Sucker bars shorten to 6, then 3 cells. |
| 3 | The beak stops fanning right and hangs under its label, on the spine, with bold single rules (`├─`, last `└─`). Every style converges on this one spine. |
| 4 | Sucker bars go; the file count stays. |
| 5–7 | Folder names are cut to 16, 10, then 6 columns (`[japa..]`); slot lists tighten (`[1:4,2,3:2]`, empty slots drop `:0`). |

```txt
○ [photos] 3 Σ55
├─ [_float_:2]
├─ [1:4,2:1]
└─ [trips]
   ├─ [japan_lo..]
   │  ├─ [_float_:1]
   │  └─ [1,2,3:2,4]
   └─ [osaka] 30
```

The tide probes through the same reflow (the viewer's context carries the
pane's width in columns), so pinning and mirroring are exact in the shape
that is painted.

Every rendered row carries a key (a folder's label, one tier of a folder's
beak, a box edge, an ink row), so between two renders the view knows which
rows moved, which are new and which are gone. Moved rows glide to their new
place. New rows grow out of the row they belong to (arms out of their
parent's label, beak tiers out of their folder) and fade in. Rows that go
slide back into it and fade out. Scrolling eases at the same rate as the
rows, so the row the tide pins under the pointer stays still while the tree
reshapes around it.

Under the pointer: a folder underlines its `[name]` and lights its whole
subtree with a band in its spine colour and a bar down the left, which
glides from folder to folder; a slot becomes a keycap in its category's
colour; an ink row lights up. A click pulses its row, except the root, which
has no click highlight. When the open folder changes without a tree click
(undo, browsing a slot), the tree scrolls it into view.

## 5. Styles

| Setting | Beak | Arms |
| --- | --- | --- |
| `Octobrachial Arm` | fanned off the label row | spine |
| `Octobrachial Beak` | fanned, and arms join the same fan | no spine anywhere; the tree reads left to right like the request's `->` chains |
| `Boxtobrachial` | tier rows in a box hooked by `──┤` | spine |

```txt
○ [photos] ·· 14 Σ259 ──┤ [_fav_:3]            │
```

## 6. Inside Rusted Calamari

The window is an image browser and organizer in one: the image takes every
pixel the layout can give it; everything else is chrome that arrives on
hover and leaves on the next keystroke. The window has no system frame.

```txt
 hover top     ┌───────────────────────────────────────────────────────┐
 menu bar   →  │ ◙ Rusted Calamari File Edit View Settings Help  – □ ✕ │
               ├─────────────┬──┬──────────────────────────────────────┤
               │ OTF tree    │ <│ ← fold tab                  ╭──────╮ │
               │ beside the  │ ││                             │ ⌗ ⌗ϟ │ │ ← tool strip
               │ image       │ ││ ← divider   image           ╰──────╯ │
               │             │ ││                                      │
               ├─────────────┴──┴──────────────────────────────────────┤
 hover bottom  │ folder │ n of m │ name │ WxH │ MB │ zoom  z 1  x 2 .. │
 status line → └───────────────────────────────────────────────────────┘
```

- **No system frame.** The menu bar is the title bar: drag its empty part
  to move the window, double-click it to maximize or restore. Its right end
  holds minimize, maximize/restore and close (Phosphor icons; close lights
  red). A 4px band on every edge and a corner square resize the window. A
  1px hairline marks the window's edge while it floats; maximized or full
  screen (`F11`), the bands and the hairline step aside.
- **Tool strip.** View > Show the toolbar (or Settings > `Toolbar at the
  top right`) puts a small horizontal strip of icon buttons at the top
  right, just under the menu bar's band, so the two never overlap. It
  follows the interface: it shows with the menu bar, when the pointer comes
  near it, while one of its tools is on, and always when pinned; turned on,
  it shows itself for a moment. Tools are one list in the app (icon, name,
  shortcut, group, state); the strip sizes itself from it, puts a rule
  between groups, right-aligns, and wraps onto another row before it would
  run past the stage. Toasts stack under it. Buttons: hover lifts and
  lights the icon, press sinks and shrinks it, a click sends a ring out, an
  engaged tool fills with its colour; after a short hover, the name, the
  shortcut and a line on what it does appear under the button. Icons are
  Phosphor (MIT) through egui-phosphor, bold cut. Tools now: **Crop**
  (`Ctrl+K`) and **DANGER crop** (`Ctrl+Shift+K`).
- **DANGER crop.** Speed over precision. With it on, press on the image,
  drag a box, let go: the file is cropped and written at once, no
  confirmation. Shift keeps the box square; the red label shows the pixel
  size while dragging. The picture grows out of the box into its new frame
  and a red ring marks it written. The write runs in the background (one
  worker, in order), so the next box or the next image never waits;
  anything else that touches files waits for queued writes first. A click
  without a drag does nothing. The tool stays on while you browse; the
  picture carries a red hairline while it is armed. Right or middle drag
  pans. `Esc` lets go of a box, then turns the tool off. `Ctrl+Z` undoes a
  crop (the file gets its old bytes back), `Ctrl+Shift+Z` (or `Ctrl+Y`)
  redoes it. Pending edits on the image go into the same write; if a write
  fails (a locked file, say), they come back as pending edits and any boxes
  already queued on that file are dropped rather than cut in the wrong
  place. GIF keeps its frames, so DANGER crop stays off for it (Crop and
  Export as still work).
- **Undo and redo.** Every undoable step (move to a category, trash, save,
  DANGER crop, an edit) knows how to reverse itself and yields the step
  that reverses it back, so undo and redo trade entries and cover all of
  them. A redone move leaves the folder the way the first move did (the
  next image steps up). Anything new clears what was undone.

- **Fold.** The tree sits beside the image. A tab at the top of the
  divider, just under the menu bar's band, arrives with the menu bar or
  when the pointer comes near it: `<` folds the tree away (image focus,
  below), `>` at the window's left edge brings it back. `Tab` does both.
  The image folds too (`Shift+Tab`, View > Fold the image): the tree takes
  the whole window and the tab moves to the right edge, where `<` brings
  the image back. One of the two always stays.
- **Divider.** A two-cell margin separates the tree from the image, with a
  1px rule down its centre. The margin opens and closes with either fold.
  On hover the rule turns rust, thickens and shows a grip; drag it to set
  the tree's width, double-click it to go back to hugging the tree. Drag it
  all the way left and the tree folds; all the way right and the image
  folds; the window's edge on the side that will fold tints rust while the
  pointer is past it, and dragging back undoes the fold before you let go.
  With the image folded, grab the window's right edge to pull it back.
- **Image focus.** Folding the tree makes the window the picture's frame.
  The window collapses round the image, edge to edge, with no padding and
  no bars: a portrait picture gets a portrait window. As you browse it
  takes each image's shape, keeping its centre. The picture fills a size
  budget: the window as it was when the tree folded, scaled whenever you
  resize the window by hand (the side you drag sets the scale, and the
  window settles back round the picture a moment after you stop). The
  maximize button, `F11` and a double-click on the top bar step through
  three sizes: **Fit** (the budget), **Large** (the same, as large as the
  screen's work area allows, centred on it) and **Full** (true full screen,
  the picture centred, with bars only where its shape differs from the
  screen's); the next step goes back to Fit, as does `Esc` in full screen.
  The maximize button's icon says where it goes next. A window or menu that
  needs more room than the picture gives (Help beside a portrait picture)
  grows the window while it is open. The menu bar narrows with the window:
  the path goes first, then the name beside the logo, then Help and
  Settings, which keep their keys; the status line's chips keep their keys
  and drop their names. The category colour, which has no room under the
  picture, becomes a 2px frame inside its edge. Unfolding the tree gives
  the window back its size from before (re-maximized, if it was). The
  budget and that size are remembered between sessions. On Windows the
  work area comes from the system and excludes the taskbar; elsewhere it is
  estimated from the monitor size.
- **Crops come first.** While Crop or DANGER crop is on, the floating
  interface yields to the picture: with the pointer on or at the picture,
  or a frame being dragged, the menu bar, status line, tool strip and fold
  tab neither come up nor take clicks, and the window's resize bands are
  off, so every edge of the picture can be reached, including in focus,
  where the picture's edges are the window's. Nothing is padded or shrunk
  to make room. A menu or window opened on purpose (`Alt+F`, `.`) still
  opens, and pinned bars keep their own room. The crop bar sits under the
  picture's middle, or over it when the frame's bottom edge runs under it,
  and fades and lets the pointer through whenever the pointer comes within
  reach of a frame edge or corner.
- **Pinned.** `PIN_UI` keeps the menu bar and status line up, and the stage
  shrinks to sit between them instead of under them.
- **Chrome.** Menus and dialogs share one frame: a bold 2px rule with
  rounded corners (the `╭─╮ ╰─╯` shape, drawn as vector because no CP437
  face carries those glyphs). The rule is the window's edge: the background
  fills to its outer line and stops there; only a blurred shadow falls
  beyond it. Dialogs open with ` ■ Title ` on their first row (the square
  closes them) and a divider under it. Nothing in the interface takes
  keyboard focus except text fields, so `Tab` and the arrow keys always
  belong to the viewer. Every divider stops one cell short
  of the rule on each side. Menus and dialogs fade and lift in as they
  open. Shortcuts sit right-aligned; the hovered row inverts in rust.
- **Category rows in Settings.** Each key is a keycap in its category's
  colour (`+` when it has none). Clicking a keycap opens a small popover
  under it: `[1]  z  press the new key`. The next key pressed becomes the
  category's key and `.calamari` is written. A key another category holds
  is swapped, never duplicated (a toast says who has what now); a key the
  app keeps (`h`, `0`) or a Ctrl/Alt chord shakes the popover with the
  reason. Backspace unbinds the key; Esc, the keycap again or a click
  elsewhere leaves it as it was. Double-clicking a colour (the hex or its
  swatch) opens an editor under it: hex field, picker, the twelve swatches.
  The category takes the colour as it changes, so the tree, keycaps and
  status chips recolour while you drag. Done, Enter or a click outside
  keeps it and writes `.calamari`; Revert or Esc puts the old colour back.
  The rest of a row opens the full form (name, key, colour, tier, delete).
- **Title.** Off by default. Settings > `Title over the image` (or View >
  Show the title) writes the folder and the image's number, `japan 003`,
  large on the image's lower left.
- **Status line.** Left: folder, position, file name, pixel size, file
  size, zoom, and `edited, not saved` when there are pending edits. Right:
  one keycap chip per bound category in its cid. Clicking a chip moves
  the image there; a keyboard move drops the image toward its chip, which
  flashes even while the line is hidden.

| You do | The window does |
| --- | --- |
| `z x c v` / `SPACE` (any key bound to a category) | The image is moved on disk into `<active>/<category>/` and drops toward its chip; the slot token in the tree flashes and its count ticks up; the next image is already up. A name clash gets `(2)`; nothing is overwritten. Pending edits are saved first so they travel with the file. |
| tree: click a slot token | Same move, by mouse. |
| tree: right-click a slot token | Browse that category folder; the token shows `◄n/m`. |
| tree: click a folder | Browse it; it opens through the tide; click again to fold it. |
| tree: click ink | Next page of arms. |
| `← → ↑ ↓ [ ] ; '` | ±1, ±5, ±25, ±50 through the active folder. |
| `Ctrl+K` | Crop tool: drag a frame, drag its edges or corners, drag inside to move it. `A` or the `◄ ►` in the crop bar cycles free, original, 1:1, 4:3, 3:2, 16:9, 3:4, 2:3, 9:16. `Enter` applies, `Esc` cancels. |
| `Ctrl+Shift+K` | DANGER crop on or off: drag a box on the image, let go, it is cropped and saved. |
| `Ctrl+R` / `Ctrl+Shift+R`, `Ctrl+F` / `Ctrl+Shift+F` | Rotate right / left, flip horizontal / vertical. |
| `Ctrl+S` | Save the pending edits into the file (JPEG at quality 95, EXIF orientation baked in). Undo restores the old bytes. |
| `Ctrl+Shift+S` | Export as: a new file with the edits applied; PNG, JPEG, WebP, BMP or TIFF, picked by extension. The original is untouched. |
| File > Revert edits | Drop the pending edits on this image. |
| `Ctrl+Z` / `Backspace` | Undo the last edit, save, DANGER crop, move or trash. |
| `Ctrl+Shift+Z` / `Ctrl+Y` | Redo. |
| `Delete` | Move to `<library>/.calamari_trash/`. |
| wheel / drag / `0` / `Ctrl+1` / double-click | Zoom / pan / fit / actual size / fit. |
| `Tab` · `Shift+Tab` · `H` · `.` · `Esc` · `Alt+F/E/V` | Fold the tree (image focus) · fold the image · keys · settings · close (or leave full screen) · open a menu. |
| `F11` · top bar drag · top bar double-click | Full screen (in focus: Fit, Large, Full) · move the window · maximize (in focus: the same steps as F11). |
| `Ctrl+O` · `Ctrl+I` · drop a folder or files | Open a folder · import images into the active folder · either. |
| close the window with pending edits | A dialog offers save all and quit, quit without saving, or cancel. |

Edits are pending until saved: crop, rotate and flip apply to a
screen-resolution preview at once and to the full-resolution file only on
Save or Export as, so a crop can be undone step by step before anything
touches the disk.

The active folder always shows every category as a slot, including ones
not yet created on disk (`4:0` dimmed); the folder is created on the first
move into it. Other folders show only the categories they actually contain.

Menus: File (open folder, import images, save, save all, export as, revert
edits, import/export categories, export manifest `.csv`, export categorised
images, quit), Edit (undo, redo, crop, DANGER crop, rotate, flip, move to
trash), View (fold the tree, fold the image, fit, actual size, title,
toolbar, tree style, pin the interface, full screen or the next focus
size), Settings, Help.
Settings holds PIN_UI, the tree, title and toolbar toggles, the twelve
int10h faces, size, style, and `[create]` > `[classification_category]`
listing every category: click a keycap to rebind it, double-click a colour
to change it, click the rest of the row to edit or delete it,
`+ category` to add one (name, key captured by pressing it, colour with
swatch cycling, tier).

The window reopens the last library and settings on launch.

Category table file, `<library>/.calamari` **[new]**:

```txt
# key  name     cid      tier
spc    _float_  #E6D463  a
-      _fav_    #F0A35E  b
z      1        #e56399  c
x      2        #92CC7C  c
c      3        #7f96ff  c
v      4        #a6cfd5  c
```

`_fav_` has no key or cid in the request; `#F0A35E` is a placeholder.

## 7. Open edges

- Category directories that themselves contain subdirectories are counted
  into their beak total; right-click browses only the category folder's
  own files.
- Video, HEIC, AVIF and JXL files can be filed into categories but show as a card; there
  is no decoder for them yet.
