# Two-widget OTF workspace verification — 2026-10-13

Implemented: upper Favourites and lower full-filesystem Files, a remembered
split, native detachment, dock fill/snap-back, individual Shift-click
toggle, gated pair recall/restore, and the existing image-focus workflow.
The accepted favourites glyphs/colours remain the rendering authority.

## Checks

| Check | Result | Evidence boundary |
| --- | --- | --- |
| `cargo test --release --locked` | **42 passed**: 4 filesystem, 24 application, 9 OTF, 5 file-operation tests | Real temporary filesystem fixtures and headless egui input/layout |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Passed | Compile/static checks, including the smoke runner |
| `cargo build --release --locked --bin rusted-calamari` | Passed | Normal executable at `target/release/rusted-calamari.exe` |
| Official `designmd lint DESIGN.md` | 0 errors, 0 warnings | Design token/schema and contrast checks |
| Frontend skill strict project audit | 0 findings | Static audit; not native interaction evidence |
| Native eframe smoke | Passed | Actual Windows viewports and GPU-rendered main-window screenshots; actions driven in-process |

The egui tests cover real press/release sequences for gated triple-click,
Shift-click and dragging a docked header out. Other checks cover lone-widget
fill, split and geometry preservation, old settings loading, persistence
round trips, Files shortcut scope, folded-pane image navigation, keyboard
ink paging, lazy expansion/tide mirroring, and Cancel/Save when changing
libraries with pending edits. Existing OTF and file-operation tests pass.

The filesystem fixtures cover non-media and hidden files, category-looking
directory names, lazy loading, stale results, retryable errors, refresh
with stable node IDs, deleted selection, and every OTF style's paging.

## Native evidence

Local artifacts (ignored build output):
[`target/workspace-smoke-verified-20261013/`](../../target/workspace-smoke-verified-20261013/).
`RESULT.txt` and `progress.txt` record each transition and native geometry.
Screenshots include joined widgets, Files filling the dock, folded image
focus with a floating widget, both detached, recalled/restored pairs,
native snap-back, an unavailable folder, a narrow boxed layout, an empty
folder, visible overflow paging and an unpinned wide beak layout.

The run verified that both child windows are actual immediate native
viewports. It moved one native window into the dock using a viewport
position command and verified that snap-back retained its prior detached
geometry. Pair recall/restore preserved both positions and sizes exactly.

An earlier native run exposed accumulating vertical drift: the Windows
client origin was one pixel below the outer-window origin. Saved placement
now uses the outer origin and client size, matching the viewport builder.
Failed runs remain in `target/workspace-smoke-20261013-164518` (unsupported
child capture) and `target/workspace-smoke-20261013-164802` (geometry drift).

After that native run, a final keyboard-only fix stopped folded Files from
consuming image navigation; its egui regression test and the full 42-test
suite pass. The final normal release executable includes that fix.

## Remaining native input checks

Physical OS dragging, physical triple-click/Shift-click timing, edge resizing
across different DPI monitors, and multi-monitor/off-screen recovery were
**not exercised**. The Computer Use native pipe was unavailable (`os error
2`). The headless input tests and programmatic native relocation do not
establish those physical-input outcomes.

eframe 0.36's Glow immediate-child renderer does not return child screenshot
actions; this run captures the main window and records native child
creation/geometry. Shared widget content has headless rendering coverage;
detached-window appearance has no OS screenshot acceptance evidence.

To reproduce with isolated fixtures and settings, pass a fresh output path:

```powershell
cargo run --release --locked --features workspace-smoke --bin calamari-workspace-smoke -- target/workspace-smoke-new
```

The runner never injects OS input. It creates its own profile/fixtures,
asserts transitions, captures screenshots, writes `RESULT.txt`, and exits.
No live user library or application settings are used.
