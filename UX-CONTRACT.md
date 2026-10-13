# OTF workspace behaviour

Authority: the accepted favourites design in `OCTOBRACHIAL_TREE_FORMAT.md`,
the existing image-focus contract in `README.md`, and the user's two-widget
request (2026-10-13), including full-filesystem navigation.
Visual roles and runtime token owners are in `DESIGN.md`.

| Capability | Canonical owner | Source of truth | Allowed variants | Verification |
| --- | --- | --- | --- | --- |
| Tree placement | `src/dock.rs`, `src/workspace.rs` | Two-widget request | Upper Favourites, lower Files; native floating | Dock transitions and runtime smoke |
| Tree rendering | `src/layout.rs`, `src/paint.rs`, `src/glyph.rs` | OTF specification | Existing category labels; filesystem labels | Existing OTF tests and filesystem tests |
| File navigation | `src/fsbrowser.rs`, `src/browserpane.rs` | Full-filesystem request | Paths, drives, parent, back/forward | Real directory fixtures, stale/error/refresh tests |
| Image focus | `src/focus.rs`, main application layout | Existing README | Fit, Large, Full | Existing sizing tests and workspace smoke |
| Unsaved edits | `src/app.rs` shared unsaved dialog | Existing edit/save contract | Quit; open another library | Cancel/save/discard navigation checks |

Each widget has one content controller, regardless of its host. Detaching
and docking must preserve favourites scroll/click state, Files root,
expansion, paging, selection and scroll. Geometry persistence keeps the
last settled detached placement, not the position at the docking target.
The dock split and each widget's original top/bottom slot are independent
of floating geometry.

With both widgets joined, a subtle resizable horizontal divider separates
them. With either detached, the other fills the dock. Returning a widget
restores the saved split. Moving a native widget back over the left dock
target and releasing returns it. Shift-clicking its hover header, its
Dock/Detach button, or the corresponding View action toggles placement.
Closing a widget returns it to the dock; it never closes the library.

The main fold button and Tab affect the dock, including when only one
widget remains. Floating widgets keep operating. With both detached, the
main layout and button are folded and image focus is active. Triple-click
recall is allowed only from that state. A subsequent triple-click restores
both prior floating placements. Arbitrary triple-clicks in a normal joined
or mixed layout cannot detach both widgets. Single/double clicks in the
pair-eligible state must not move the button before a third click can land.

Files starts at the saved browser location, or the library on first use.
It can navigate any readable local, mounted or explicitly entered network
path. Enumerating a folder does not recursively scan its descendants.
Listings run on a background worker, with generation/request checks so an
old result cannot replace a new location. Errors are visible and retryable.
Folders that happen to match category names are normal folders here.
Category grouping and file moves remain in Favourites and the existing
sorting controls.

Single-click folders expand/fold them; double-click makes one the browser
root. Single-click media selects it for the image viewer. Non-media files
show path/size details and are never executed. Explicitly opening a link as
a folder uses a new root so link cycles cannot recurse. Folder navigation
alone does not replace the library. Viewing media outside the library or
using View images opens that folder through the existing library workflow,
with Save/Open without saving/Cancel if image edits are pending.

Keyboard file navigation is scoped to the Files pane. Tab keeps its dock
folding meaning; typing in the location field must not trigger sorting
hotkeys. Pointer, text field and shortcut state must not leak between native
viewports. The main image remains visible whenever the dock has no widgets.

Verification boundaries: model tests establish state transitions and
filesystem behaviour. Runtime screenshots establish actual rendering.
Physical dragging, DPI changes and multi-monitor operation need native
input verification; no synthetic test is represented as that evidence.
