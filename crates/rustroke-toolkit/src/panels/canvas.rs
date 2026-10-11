//! The canvas: a grey grid with the current view's window on it, at the
//! window's size. The content is drawn by rtk-preview. A resizable window
//! has a handle in its bottom-right corner to change its size.

use rtk_document::WindowSettings;
use rtk_preview::{DropMarker, Mode, show_view};
use rustroke::{
    Align, BoxStyle, Color, Id, Layout, Point, Rect, ScrollArea, Sense, Shadow, Stroke, TextStyle,
    Ui, Vec2, point, vec2,
};

use crate::app::{DragPayload, Toolkit};

/// Space around the window.
const MARGIN: f32 = 48.0;
/// Height of the window's title bar.
const TITLE_HEIGHT: f32 = 30.0;
/// Distance between grid lines; every fifth line is darker.
const GRID: f32 = 20.0;
/// Size of the resize handle.
const HANDLE: f32 = 16.0;

/// Colors of the grey background and its grid.
struct GridColors {
    background: Color,
    line: Color,
    major: Color,
}

fn grid_colors(dark: bool) -> GridColors {
    if dark {
        GridColors {
            background: Color::from_srgb8(56, 56, 60),
            line: Color::from_srgb8(66, 66, 71),
            major: Color::from_srgb8(80, 80, 86),
        }
    } else {
        GridColors {
            background: Color::from_srgb8(204, 204, 208),
            line: Color::from_srgb8(192, 192, 197),
            major: Color::from_srgb8(176, 176, 182),
        }
    }
}

/// Draws the grey background with its grid over `rect` (only the visible
/// part of it).
fn paint_grid(ui: &mut Ui<'_>, rect: Rect, dark: bool) {
    let colors = grid_colors(dark);
    let visible = rect.intersect(ui.clip_rect());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, colors.background);
    let line = |i: i32| {
        if i % 5 == 0 {
            Stroke::new(1.0, colors.major)
        } else {
            Stroke::new(1.0, colors.line)
        }
    };
    let first = ((visible.min.x - rect.min.x) / GRID).floor() as i32;
    let last = ((visible.max.x - rect.min.x) / GRID).ceil() as i32;
    for i in first..=last {
        let x = rect.min.x + i as f32 * GRID;
        painter.line(point(x, visible.min.y), point(x, visible.max.y), line(i));
    }
    let first = ((visible.min.y - rect.min.y) / GRID).floor() as i32;
    let last = ((visible.max.y - rect.min.y) / GRID).ceil() as i32;
    for i in first..=last {
        let y = rect.min.y + i as f32 * GRID;
        painter.line(point(visible.min.x, y), point(visible.max.x, y), line(i));
    }
}

/// Draws the window frame (shadow, title bar, border); returns the content
/// area.
fn paint_window(ui: &mut Ui<'_>, origin: Point, window: &WindowSettings, selected: bool) -> Rect {
    let style = ui.style();
    let v = &style.visuals;
    let frame = Rect::from_min_size(origin, vec2(window.width, window.height + TITLE_HEIGHT));
    let body = Rect::from_min_size(
        origin + vec2(0.0, TITLE_HEIGHT),
        vec2(window.width, window.height),
    );
    let title_bar = Rect::from_min_size(origin, vec2(window.width, TITLE_HEIGHT));
    let radius = 8.0;
    let title = ui.layout_text(
        &window.title,
        &TextStyle::proportional(13.0).weight(600),
        None,
    );
    let border = if selected {
        Stroke::new(2.0, v.accent)
    } else {
        v.window_stroke
    };
    let window_box = BoxStyle::new()
        .fill(v.background)
        .corner_radius(radius)
        .shadow(Shadow {
            offset: vec2(0.0, 8.0),
            blur: 24.0,
            spread: 0.0,
            color: Color::BLACK.with_alpha(0.45),
        });
    let painter = ui.painter();
    window_box.paint(painter, frame);
    painter.rect_filled(title_bar, radius, v.panel_fill);
    // Square off the title bar's bottom corners.
    painter.rect_filled(
        Rect::from_min_max(
            point(title_bar.min.x, title_bar.max.y - radius),
            title_bar.max,
        ),
        0.0,
        v.panel_fill,
    );
    painter.line(
        point(title_bar.min.x, title_bar.max.y),
        point(title_bar.max.x, title_bar.max.y),
        v.window_stroke,
    );
    // Window buttons, for the look.
    for (i, color) in [(255, 95, 87), (254, 188, 46), (40, 200, 64)]
        .iter()
        .enumerate()
    {
        let center = point(
            title_bar.min.x + 16.0 + i as f32 * 20.0,
            title_bar.center().y,
        );
        painter.circle_filled(center, 6.0, Color::from_srgb8(color.0, color.1, color.2));
    }
    let title_pos = point(
        title_bar.center().x - title.size.x / 2.0,
        title_bar.center().y - title.size.y / 2.0,
    );
    painter.galley(title_pos, title, v.text);
    // The border last, over the title bar.
    BoxStyle::new()
        .stroke(border)
        .corner_radius(radius)
        .paint(painter, frame);
    body
}

/// The resize handle in the bottom-right corner; returns the new size
/// while it is dragged (within the window's minimum and maximum sizes).
fn resize_handle(ui: &mut Ui<'_>, body: Rect, window: &WindowSettings) -> Option<(f32, f32)> {
    let rect = Rect::from_min_size(body.max - vec2(HANDLE, HANDLE), vec2(HANDLE, HANDLE));
    let response = ui.interact(Id::new("rtk-window-resize"), rect, Sense::POINTER_DRAG);
    let v = &ui.style().visuals;
    let color = if response.hovered() || response.dragged() {
        v.accent
    } else {
        v.weak_text
    };
    let painter = ui.painter();
    for i in 1..=3 {
        let d = i as f32 * 4.0;
        painter.line(
            point(rect.max.x - d, rect.max.y - 2.0),
            point(rect.max.x - 2.0, rect.max.y - d),
            Stroke::new(1.5, color),
        );
    }
    if response.hovered() || response.dragged() {
        ui.ctx().set_cursor(rustroke::CursorIcon::ResizeNwSe);
    }
    let delta = response.drag_delta();
    (response.dragged() && (delta.x != 0.0 || delta.y != 0.0)).then(|| {
        let clamp = |v: f32| {
            v.round()
                .clamp(WindowSettings::MIN_SIZE, WindowSettings::MAX_SIZE)
        };
        // Within the window's own minimum and maximum sizes too.
        window.clamp((
            clamp(window.width + delta.x),
            clamp(window.height + delta.y),
        ))
    })
}

/// Draws the outline of a widget being dragged on the canvas, following the
/// pointer, and dims it where it is.
fn paint_ghost(ui: &mut Ui<'_>, rect: Rect, pointer: Point) {
    let accent = ui.style().visuals.accent;
    // Where the widget was grabbed stays under the pointer.
    let grab = ui
        .input()
        .pointer
        .press_origin()
        .map_or(Vec2::ZERO, |o| o - rect.min);
    let ghost = Rect::from_min_size(pointer - grab, rect.size());
    let painter = ui.painter();
    painter.rect_filled(rect.expand(2.0), 3.0, accent.with_alpha(0.08));
    painter.rect_filled(ghost, 3.0, accent.with_alpha(0.15));
    painter.rect_stroke(ghost, 3.0, Stroke::new(1.5, accent));
}

/// Draws a drop marker on the canvas.
fn paint_marker(ui: &mut Ui<'_>, marker: DropMarker) {
    let accent = ui.style().visuals.accent;
    let painter = ui.painter();
    match marker {
        DropMarker::Line(a, b) => painter.line(a, b, Stroke::new(2.5, accent)),
        DropMarker::Area(rect) => {
            painter.rect_filled(rect, 4.0, accent.with_alpha(0.12));
            painter.rect_stroke(rect, 4.0, Stroke::new(2.0, accent));
        }
    }
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    let Some(view) = app.project.views.get(app.current) else {
        ui.label("No views: add one from the Project menu.");
        return;
    };
    let window = view.window.clone();
    let mode = app.mode;
    let dark = app.dark;
    let current = app.current;
    // Where the window sits on the canvas (editor only, not in the code).
    let window_pos = app
        .window_positions
        .get(&view.name)
        .copied()
        .unwrap_or(vec2(MARGIN, MARGIN));
    // A widget being dragged from the palette or the tree.
    let dragged = ui.ctx().dnd_payload::<DragPayload>();
    let drag = dragged.as_ref().and_then(|payload| {
        let kind = app.payload_kind(payload)?;
        let moving = match **payload {
            DragPayload::Node { view, id } if view == current => Some(id),
            _ => None,
        };
        Some((kind, moving))
    });
    let pointer = ui.input().pointer.pos();
    // Measured outside the scroll area, whose size is unbounded.
    let viewport = ui.available_rect().size();
    let mut resized = None;
    let mut moved = None;
    let mut select_window = false;
    let mut target = None;
    let out = ScrollArea::both()
        .auto_shrink(false)
        .show(ui, |ui| {
            let size = vec2(
                viewport.x.max(window_pos.x + window.width + MARGIN),
                viewport
                    .y
                    .max(window_pos.y + window.height + TITLE_HEIGHT + MARGIN),
            );
            let area = ui.allocate_response(size, Sense::POINTER_DRAG);
            paint_grid(ui, area.rect, dark);
            select_window |= area.clicked();

            let origin = area.rect.min + window_pos;
            let window_selected = mode == Mode::Design && app.selected.is_none();
            let body = paint_window(ui, origin, &window, window_selected);
            // Drag the title bar to move the window; click it to select it.
            let title_bar = Rect::from_min_size(origin, vec2(window.width, TITLE_HEIGHT));
            let title = ui.interact(Id::new("rtk-window-title"), title_bar, Sense::POINTER_DRAG);
            select_window |= title.clicked();
            if title.dragged() {
                let delta = title.drag_delta();
                moved = Some(vec2(
                    (window_pos.x + delta.x).max(0.0),
                    (window_pos.y + delta.y).max(0.0),
                ));
                ui.ctx().set_cursor(rustroke::CursorIcon::Grabbing);
            } else if title.hovered() {
                ui.ctx().set_cursor(rustroke::CursorIcon::Grab);
            }

            // The view inside the window, clipped to it like a real window.
            let margin = ui.style().spacing.window_margin;
            let state = app.canvases.entry(view.name.clone()).or_default();
            let selected = if mode == Mode::Design {
                app.selected
            } else {
                None
            };
            let out = ui
                .scope_with_no_advance(body.expand(-margin), Layout::top_down(Align::Min), |ui| {
                    ui.set_clip_rect(body);
                    show_view(ui, view, state, mode, selected)
                })
                .inner;

            // Widgets on the canvas are dragged to move them.
            if let Some((id, response)) = &out.pressed {
                let payload = DragPayload::Node {
                    view: current,
                    id: *id,
                };
                if ui.dnd_set_payload(response, payload)
                    && let (Some(rect), Some(pointer)) = (state.rect(*id), pointer)
                {
                    paint_ghost(ui, rect, pointer);
                }
            }

            if let (Some((kind, moving)), Some(pointer)) = (drag, pointer) {
                target = state.drop_target(view, pointer, kind, moving, body);
                if let Some(target) = target {
                    let clip = ui.clip_rect();
                    ui.set_clip_rect(body);
                    paint_marker(ui, target.marker);
                    ui.clip_rect_restore(clip);
                }
            }

            if window.resizable {
                resized = resize_handle(ui, body, &window);
            } else {
                let weak = ui.style().visuals.weak_text;
                let text = format!("{} × {} — fixed size", window.width, window.height);
                let galley = ui.layout_text(&text, &TextStyle::proportional(12.0), None);
                let pos = point(body.max.x - galley.size.x, body.max.y + 8.0);
                ui.painter().galley(pos, galley, weak);
            }
            out
        })
        .inner;
    if let Some(pos) = moved {
        let name = app.project.views[current].name.clone();
        app.window_positions.insert(name, pos);
    }
    if let (Some(payload), Some(target)) = (dragged, target)
        && ui.input().pointer.primary_released()
    {
        ui.ctx().take_dnd_payload::<DragPayload>();
        app.drop_payload(&payload, current, target.parent, target.index);
        return;
    }
    if let Some(id) = out.clicked {
        app.selected = Some(id);
    } else if out.background_clicked || select_window {
        // Nothing selected = the window itself (its properties are shown).
        app.selected = None;
    }
    if let Some((width, height)) = resized {
        let window = WindowSettings {
            width,
            height,
            ..window
        };
        app.edit_tagged(Some("window"), |view| view.set_window(window));
    }
}
