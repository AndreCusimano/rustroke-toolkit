# Changelog

All notable changes to Rustroke Toolkit. Versions follow
[Semantic Versioning](https://semver.org): the major version changes with the
targeted rustroke version.

## [Unreleased]

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

[Unreleased]: https://github.com/AndreCusimano/rustroke-toolkit/compare/v1.0.0...HEAD
[1.0.0]: https://github.com/AndreCusimano/rustroke-toolkit/releases/tag/v1.0.0
