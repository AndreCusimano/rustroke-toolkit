//! Left panel: the views of the project, each with its component tree.

use std::collections::HashMap;

use rtk_document::{NodeId, Project, WidgetKind};
use rustroke::{
    Button, Icon, IconId, Id, Point, Rect, Response, Sense, Stack, StackAlign, Stroke, Tree, Ui,
    point,
};

use crate::app::{Dialog, DragPayload, Toolkit};

/// A row of the tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Row {
    /// A view, by index.
    View(usize),
    /// A node of a view.
    Node(usize, NodeId),
}

/// Where a payload dropped on the tree goes.
#[derive(Clone, Copy, Debug, PartialEq)]
struct TreeDrop {
    view: usize,
    parent: Option<NodeId>,
    index: usize,
    /// The row, and whether the node goes inside it (else on the line
    /// above or below it).
    marker: Marker,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Marker {
    Inside(Rect),
    Line(Point, Point),
}

/// Whether node `id` of `view` is `ancestor` or inside it.
fn within(project: &Project, view: usize, id: Option<NodeId>, ancestor: NodeId) -> bool {
    let Some(view) = project.views.get(view) else {
        return false;
    };
    let (Some(id), Some(outer)) = (id, view.path(ancestor)) else {
        return false;
    };
    view.path(id).is_some_and(|path| path.starts_with(&outer))
}

/// The drop target under `pointer`: the upper or lower part of a row puts
/// the node before or after it, the middle of a container row inside it,
/// and a view row at the end of that view.
fn tree_drop(
    project: &Project,
    rows: &[(Row, Rect)],
    pointer: Point,
    kind: WidgetKind,
    moving: Option<(usize, NodeId)>,
) -> Option<TreeDrop> {
    let &(row, rect) = rows
        .iter()
        .find(|(_, r)| r.min.y <= pointer.y && pointer.y < r.max.y)?;
    let frac = (pointer.y - rect.min.y) / rect.height().max(1.0);
    let target = match row {
        Row::View(v) => {
            let view = project.views.get(v)?;
            view.accepts_child(None, kind).then_some(TreeDrop {
                view: v,
                parent: None,
                index: view.nodes.len(),
                marker: Marker::Inside(rect),
            })?
        }
        Row::Node(v, id) => {
            let view = project.views.get(v)?;
            let node = view.node(id)?;
            let into = node.kind.is_container() && view.accepts_child(Some(id), kind);
            let parent = view.parent(id)?;
            let index = view.children(parent)?.iter().position(|n| n.id == id)?;
            let beside = view.accepts_child(parent, kind);
            let inside = TreeDrop {
                view: v,
                parent: Some(id),
                index: node.children.len(),
                marker: Marker::Inside(rect),
            };
            let line = |after: bool| {
                let y = if after { rect.max.y } else { rect.min.y };
                TreeDrop {
                    view: v,
                    parent,
                    index: index + usize::from(after),
                    marker: Marker::Line(point(rect.min.x, y), point(rect.max.x, y)),
                }
            };
            match (into, beside) {
                (true, true) if (0.25..0.75).contains(&frac) => inside,
                (true, false) => inside,
                (_, true) => line(frac >= 0.5),
                (false, false) => return None,
            }
        }
    };
    // A node can't go inside itself.
    if let Some((view, id)) = moving
        && view == target.view
        && within(project, view, target.parent, id)
    {
        return None;
    }
    Some(target)
}

/// A context-menu item with an icon.
fn item(ui: &mut Ui<'_>, icon: IconId, text: &str) -> bool {
    ui.add(Button::new(text).icon(icon)).clicked()
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    let icons = app.icons();
    Stack::row().align(StackAlign::Center).show(ui, |ui| {
        ui.add(Icon::new(icons.panel_views).size(20.0));
        ui.heading("Views");
        let new_view = Button::icon_only(icons.view_new).accessible_label("New view");
        if ui.add(new_view).on_hover_text(ui, "New view").clicked() {
            app.dialog = Some(Dialog::new_view(&app.project));
        }
    });
    ui.separator();

    let roots: Vec<Row> = (0..app.project.views.len()).map(Row::View).collect();
    let mut selection = Some(match app.selected {
        Some(id) => Row::Node(app.current, id),
        None => Row::View(app.current),
    });
    let before = selection;
    let project = &app.project;
    let children = |row: Row| -> Vec<Row> {
        let (view, nodes) = match row {
            Row::View(v) => (v, project.views.get(v).map(|view| view.nodes.as_slice())),
            Row::Node(v, id) => (
                v,
                project
                    .views
                    .get(v)
                    .and_then(|view| view.node(id))
                    .map(|n| n.children.as_slice()),
            ),
        };
        nodes
            .unwrap_or(&[])
            .iter()
            .map(|n| Row::Node(view, n.id))
            .collect()
    };
    let label = |row: Row| -> String {
        match row {
            Row::View(v) => project
                .views
                .get(v)
                .map_or_else(String::new, |view| format!("{}.rs", view.name)),
            Row::Node(v, id) => project
                .views
                .get(v)
                .and_then(|view| view.node(id))
                .map_or_else(String::new, |n| n.summary()),
        }
    };
    // The tree scrolls by itself. Inside another scroll area, rustroke
    // 0.17.1 panics in debug builds (overflow in `ScrollArea::show_rows`
    // with an unbounded viewport).
    // Nodes are drag sources; the source takes the pointer, so each row
    // also gets its own click area (selection, context menu).
    let mut row_clicks: HashMap<Row, Response> = HashMap::new();
    let response = Tree::new("views").default_open_depth(16).show(
        ui,
        &roots,
        children,
        &mut selection,
        |ui, row| {
            let content = |ui: &mut Ui<'_>| {
                let icon = match row {
                    Row::View(_) => Some(icons.view_window),
                    Row::Node(v, id) => project
                        .views
                        .get(v)
                        .and_then(|view| view.node(id))
                        .map(|n| icons.node(n)),
                };
                if let Some(icon) = icon {
                    ui.icon(icon);
                }
                ui.label(label(row))
            };
            match row {
                Row::View(_) => {
                    content(ui);
                }
                Row::Node(v, id) => {
                    let drag = ui.dnd_drag_source(
                        Id::new(("tree", v, id)),
                        DragPayload::Node { view: v, id },
                        |ui| {
                            let label = content(ui);
                            ui.interact(Id::new(("tree row", v, id)), label.rect, Sense::CLICK)
                        },
                    );
                    row_clicks.insert(row, drag.inner);
                }
            }
        },
    );
    // A release at the end of a drag lands on the dragged row: not a click.
    let dragging = ui.ctx().is_dnd_active();
    for (row, click) in &row_clicks {
        if click.clicked() && !dragging {
            selection = Some(*row);
        }
    }

    // Dropping on the tree.
    if let Some(payload) = ui.ctx().dnd_payload::<DragPayload>()
        && let Some(pointer) = ui.input().pointer.pos()
        && response.response.rect.contains(pointer)
        && let Some(kind) = app.payload_kind(&payload)
    {
        let rows: Vec<(Row, Rect)> = response
            .rows
            .iter()
            .map(|(r, resp)| (*r, resp.rect))
            .collect();
        let moving = match *payload {
            DragPayload::Node { view, id } => Some((view, id)),
            DragPayload::New(_) => None,
        };
        if let Some(target) = tree_drop(&app.project, &rows, pointer, kind, moving) {
            let accent = ui.style().visuals.accent;
            let painter = ui.painter();
            match target.marker {
                Marker::Inside(rect) => painter.rect_stroke(rect, 3.0, Stroke::new(2.0, accent)),
                Marker::Line(a, b) => painter.line(a, b, Stroke::new(2.0, accent)),
            }
            if ui.input().pointer.primary_released() {
                ui.ctx().take_dnd_payload::<DragPayload>();
                app.drop_payload(&payload, target.view, target.parent, target.index);
                return;
            }
        }
    }

    if selection != before {
        match selection {
            Some(Row::View(v)) => {
                app.open_view(v);
                app.selected = None;
            }
            Some(Row::Node(v, id)) => {
                app.open_view(v);
                app.selected = Some(id);
            }
            None => app.selected = None,
        }
    }
    if let Some(Row::View(v)) = response.double_clicked
        && let Some(view) = app.project.views.get(v)
    {
        app.dialog = Some(Dialog::RenameView {
            old: view.name.clone(),
            name: view.name.clone(),
        });
    }

    for (row, row_response) in &response.rows {
        let row_response = row_clicks.get(row).unwrap_or(row_response);
        row_response.context_menu(ui, |ui| match *row {
            Row::View(v) => {
                let Some(name) = app.project.views.get(v).map(|view| view.name.clone()) else {
                    return;
                };
                if item(ui, icons.view_rename, "Rename…") {
                    app.dialog = Some(Dialog::RenameView {
                        old: name.clone(),
                        name: name.clone(),
                    });
                }
                if item(ui, icons.edit_duplicate, "Duplicate") {
                    let new = app.project.suggest_view_name(&format!("{name}_copy"));
                    app.edit_project(|p| p.duplicate_view(&name, &new));
                }
                if item(ui, icons.edit_delete, "Delete…") {
                    app.dialog = Some(Dialog::DeleteView { name });
                }
                ui.separator();
                let paste = Button::new("Paste").icon(icons.edit_paste);
                if ui.add_enabled(app.clipboard.is_some(), paste).clicked() {
                    // At the end of this view.
                    app.open_view(v);
                    app.selected = None;
                    app.paste();
                }
            }
            Row::Node(v, id) => {
                app.open_view(v);
                app.selected = Some(id);
                if item(ui, icons.edit_cut, "Cut") {
                    app.cut_selected();
                }
                if item(ui, icons.edit_copy, "Copy") {
                    app.copy_selected();
                }
                let paste = Button::new("Paste").icon(icons.edit_paste);
                if ui.add_enabled(app.clipboard.is_some(), paste).clicked() {
                    app.paste();
                }
                ui.separator();
                if item(ui, icons.edit_duplicate, "Duplicate") {
                    app.duplicate_selected();
                }
                if item(ui, icons.edit_move_up, "Move up") {
                    app.move_selected(-1);
                }
                if item(ui, icons.edit_move_down, "Move down") {
                    app.move_selected(1);
                }
                ui.separator();
                if item(ui, icons.edit_delete, "Delete") {
                    app.delete_selected();
                }
            }
        });
    }
}
