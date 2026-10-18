# Changelog

All notable changes to Rustroke Toolkit. Versions follow
[Semantic Versioning](https://semver.org): the major version changes with the
targeted rustroke version.

## [Unreleased]

## [3.0.0] — 2026-10-18

For **rustroke 0.21** (built and tested with 0.21.0).

- Side bars along the window edges show and hide the Widgets, Views,
  Properties and Code panels; the choice is remembered between runs. The
  panels keep their widths while hidden.
- Dialogs can be moved by dragging their title bar (`Modal::movable`).
- The text column of the palette uses the width rustroke 0.21 assigns to a
  stack item, so descriptions keep wrapping inside the panel.

## [2.0.0] — 2026-10-12

For **rustroke 0.20** (built and tested with 0.20.0).

- One Stack replaces the vertical, horizontal, centered and indented
  layouts, containers and flex layouts, as in rustroke 0.20: a stack
  without options is written with the short forms `ui.vertical(..)` /
  `ui.horizontal(..)`, and `horizontal_wrapped`, `vertical_centered` and
  `indent` are read as stacks. The component tree shows a stack's
  direction.
- Stacks: rows, columns and overlays
  in a box (background, border, corner radius, shadow, padding, margin,
  width and height that follow the content, fill the space or are fixed,
  minimum and maximum size, clipping), with gap, wrapping, justification
  and alignment; stack items with grow, shrink, basis, size limits and
  their own alignment.
- Grids gain column widths (`120, 1fr, auto`), cell alignment, the box
  options and grid cells that span columns and rows, placed after the
  previous cell or at a given column and row; they replace flex grids.
- Charts: plots with line series (width, dashed or dotted, filled to a
  baseline, markers, min/max marks), point series (marker shape, size),
  horizontal and vertical reference lines, axis titles, legend, crosshair,
  tooltip, pan and zoom; pie and donut charts. Series are bound to
  `Vec<[f64; 2]>` fields, pies to `Vec<(String, f64)>`; the canvas shows
  example data. A new Charts menu in the palette.
- Views written for rustroke 0.19 are converted when read: `Container`,
  `Flex`/`FlexItem` and `FlexGrid`/`GridCell` become stacks, stack items,
  grids and grid cells; their imports go.
- Optional on/off properties can be left unset (e.g. clipping, which
  rustroke decides by default).
- The toolkit's own interface is built with the new layouts too: the menu
  bar, status bar, panel headers and palette items are stacks, and the
  windows on the canvas are drawn with a `BoxStyle`.
- The README picture shows the dashboard example.

## [1.0.0] — 2026-10-11

First version, for **rustroke 0.19** (built and tested with 0.19.0).

- Views as windows: title, size and resizable, drawn on a grey grid canvas;
  resize handle for resizable windows.
- Views panel with the component tree of each view; new, rename, duplicate,
  delete.
- Widget palette: every widget by category with icon and explanation, and
  a search field; click to add, or drag onto the tree or the canvas.
- Drag and drop: from the palette or the tree to the tree or the canvas,
  into containers when they accept the widget, between views too.
- Windows can be moved on the canvas by their title bar; widgets on the
  canvas are dragged to move them.
- Windows have an optional minimum and maximum size (`MIN_SIZE`,
  `MAX_SIZE`), and `RESIZABLE` applies to the native window too.
- Style properties: text color and font of labels and headings;
  background, border, corner radius and minimum size of buttons, text
  fields, search fields and combo boxes; a fixed size for any widget
  (`ui.add_sized`); button options (selected, shortcut text, frameless);
  text field options (code editor, no wrapping, line numbers, font,
  margin); spinner color, animated progress bars, grid cell spacing.
  Containers and views set the colors (text, accent, widget and field
  backgrounds) and spacing of their content.
- Text color and font of buttons, checkboxes, radio buttons, selectable
  labels, links and collapsing headers; bar color of sliders and progress
  bars; separators with their own line, spacing and orientation.
- Zones: panels on any side of a view (`Panel::show_inside`) with a size,
  minimum and maximum, and the central panel for the rest; containers
  (`Container`) with background, border, corner radius, shadow, inner and
  outer margins, minimum and maximum size, filling the width or height;
  flex layouts that fill the available height or width, flex items with a
  minimum and maximum size.
- Undo and redo of every change (typing merged into one step); cut, copy
  and paste of widgets with their children, also between views.
- Icons for the menus, the palette and the component tree.
- Widgets: labels, headings, links, separators, spaces, buttons,
  checkboxes, radio buttons, selectable labels, combo boxes, sliders, drag
  values, progress bars, spinners, text fields, search fields, time, color
  and date pickers; vertical, horizontal, centered and indented layouts,
  grids, collapsing headers, scroll areas, containers, panels, flex
  layouts, flex grids with cells that span columns and rows; free code
  blocks.
- Properties panel for widgets and windows, with validated binding names.
- Code panel kept in sync both ways, with syntax colors, line numbers and
  Tab indenting; code the toolkit doesn't recognize is kept as written,
  comments included; syntax errors are shown with their position and
  underlined.
- Design and preview modes; neutral dark and light themes.
- Projects saved as a folder (`<view>.rs`, `mod.rs`, `rustroke-toolkit.ron`)
  with native folder dialogs; recent projects; unsaved-changes guard.

[Unreleased]: https://github.com/AndreCusimano/rustroke-toolkit/compare/v3.0.0...HEAD
[3.0.0]: https://github.com/AndreCusimano/rustroke-toolkit/releases/tag/v3.0.0
[2.0.0]: https://github.com/AndreCusimano/rustroke-toolkit/releases/tag/v2.0.0
[1.0.0]: https://github.com/AndreCusimano/rustroke-toolkit/releases/tag/v1.0.0
