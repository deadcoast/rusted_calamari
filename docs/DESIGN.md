---
version: alpha
name: Rusted Calamari
description: An image-first desktop workspace with movable CP437 Octobrachial trees.
colors:
  primary: "#A9BCD0"
  stage: "#17181A"
  panel: "#1C1D20"
  iron: "#25272B"
  edge: "#62656B"
  text: "#DDD8CC"
  muted: "#8E8B84"
  steel: "{colors.primary}"
  rust: "#B2522A"
  favourite: "#F0A35E"
typography:
  mono:
    fontFamily: "Px437 IBM Model3x Alt1"
rounded:
  DEFAULT: "6px"
  control: "2px"
omitted:
  - section: spacing
    reason: Native layout uses measured character cells and rows, not CSS lengths.
components:
  stage:
    backgroundColor: "{colors.stage}"
  tree:
    backgroundColor: "{colors.panel}"
    textColor: "{colors.text}"
    typography: "{typography.mono}"
  treeRoot:
    textColor: "{colors.steel}"
  treeBrackets:
    textColor: "{colors.muted}"
  favourite:
    textColor: "{colors.favourite}"
  widgetHeader:
    backgroundColor: "{colors.iron}"
    textColor: "{colors.text}"
  dockDivider:
    backgroundColor: "{colors.edge}"
  dockPreview:
    backgroundColor: "{colors.primary}"
  dialog:
    backgroundColor: "{colors.iron}"
    textColor: "{colors.text}"
    rounded: "{rounded.DEFAULT}"
  dialogAction:
    textColor: "{colors.rust}"
    rounded: "{rounded.control}"
---

# Rusted Calamari Design System

## Overview

The reference is a DOS character-grid image workstation: CP437 tree arms,
syntax-like colour, and quiet iron panels around a photograph. The product
is an English desktop image browser and sorter. Windows is the primary
verification platform; no additional geographic market is assumed.

The accepted favourites OTF is the visual authority. Its category slots,
single bold category connectors, steel root accent, plain `○` root marker,
muted brackets, and absence of visible tier letters must survive container
changes. The image keeps priority. Controls reveal themselves on hover.

Runtime tokens remain canonical (ownership model B): this file records
their roles; it does not generate Rust or introduce a second theme.

## Colors

`src/theme.rs` owns stage, panel, iron, edge, text, muted and rust. The tree
palette, steel navigation accent and depth ramp live in `src/paint.rs`.
Category colours belong to `.calamari`/`src/config.rs`; orange identifies
`_fav_`. Navigation must not reuse that orange as the root treatment.
Both widgets use the same panel, depth ramp and punctuation colours.
Dock previews use steel; the joined divider uses the existing edge colour
at reduced opacity. The photograph's stage remains neutral.

## Typography

`src/fonts.rs` owns the twelve selectable int10h CP437 faces and size list.
`src/glyph.rs::Grid` measures the selected face, aligns text and click cells,
and supplies the common row rhythm. Native pixel values here mean UI points
at scale 1.0. Keep user-selected fonts in both native widgets. Ordinary file
names retain their original spelling; full paths remain available through
the location field and details tooltip. Phosphor remains the application
icon family.

## Layout

Favourites occupies the upper dock slot; Files occupies the lower slot.
The horizontal split is resizable and remembered. One docked widget fills
the dock; zero docked widgets give the main window to image focus. Detached
widgets have independent native positions and sizes. Reattaching restores
their original slots and split without recreating content state.

Headers reserve a row even when faded, avoiding shifting tree hit targets.
Each content widget owns its scroll position and width reflow. The shared
vertical image divider and folding transitions retain their existing jobs.
The Files widget uses OTF's eight-arm/ink paging rather than an unbounded
rendered directory list; only requested folders are read.

## Elevation & Depth

Joined widgets share one flat panel. Use a single subtle dividing rule,
without cards, shadows or doubled borders around each widget. Detached
windows use the existing native-window hairline and resize bands. Menus
and dialogs retain `src/theme.rs` frames and shadows.

## Shapes

Tree structure uses single CP437 rules. Stacked category rules keep their
bold treatment. The root is a plain `○`, never the inverse `◙` character.
Application dialog corners remain 6 points; small toolbar targets use
2-point corners. These are existing runtime shapes, not a new style.

## Components

`src/dock.rs` owns placement and pair-recall gestures. `src/workspace.rs`
hosts both widget headers and their dock/native containers. The existing
favourites painter remains in `src/app.rs`; `src/browserpane.rs` owns Files
content and `src/fsbrowser.rs` owns read-only filesystem loading.

Hover headers offer drag and explicit Dock/Detach actions. Shift-click
toggles their placement. The View menu exposes the same operations. Native
close returns a widget to the dock, preserving access and its content.
The fold tab has a gated triple-click recall/restore gesture; ordinary
folding is unchanged when the pair gesture is unavailable.

Files has an Anchor toggle in the shared hover header, immediately before
Dock/Detach. The existing Phosphor anchor is outlined with the normal text
colour when off, filled with steel when on; `Files (anchored)` supplies a
text state as well. Anchor controls folder-following independently of
window placement. It never adds a background to a tree root.

Files shows loading, empty, unavailable and populated folders explicitly.
Unavailable folders can retry. Links are labelled and never traversed
recursively. Selecting a normal media file previews it; other file types
remain browsable without launching programs. Changing libraries uses the
shared unsaved-edits dialog. No filesystem write belongs to file browsing.

Motion remains functional: existing tree row interpolation, scroll easing,
panel folding and header fades. Detachment does not reset any of these
controllers. Neither widget adds decorative continuous animation.

## Do's and Don'ts

- Keep the accepted OTF favourites rendering and sorting hotkeys intact.
- Preserve detached geometry separately from the dock slot and split.
- Keep full-filesystem navigation out of category classification logic.
- Do not put a filled square behind the root circle.
- Do not make docking or window closure lose pending image edits.
- Do not call headless tests proof of physical mouse or multi-monitor behaviour.
