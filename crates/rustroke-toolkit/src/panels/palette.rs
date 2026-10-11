//! The widget palette (leftmost panel): every kind of node, by category in
//! collapsible sections, each with its icon and a short explanation. A
//! search field filters them into one flat list. Click an item to add it
//! after the selection, or drag it onto the component tree or the canvas.

use rtk_document::{Category, WidgetKind};
use rustroke::{
    Align, CollapsingHeader, Icon, Id, Label, Layout, ScrollArea, SearchField, Sense, TextStyle, Ui,
};

use crate::app::{DragPayload, Toolkit};
use crate::icons::Icons;

/// Search text of the palette.
#[derive(Debug, Default)]
pub struct PalettePanel {
    filter: String,
}

/// Whether `kind` matches the search text (name, description or category).
fn matches(kind: WidgetKind, filter: &str) -> bool {
    let spec = kind.spec();
    let filter = filter.to_lowercase();
    filter.split_whitespace().all(|word| {
        spec.name.to_lowercase().contains(word)
            || spec.description.to_lowercase().contains(word)
            || spec.category.name().to_lowercase().contains(word)
    })
}

/// One palette item: draggable; a click adds the widget.
fn item(ui: &mut Ui<'_>, app: &mut Toolkit, icons: &Icons, kind: WidgetKind) {
    let spec = kind.spec();
    let style = ui.style();
    let weak = style.visuals.weak_text;
    let width = ui.available_width();
    let mut clicked = false;
    ui.dnd_drag_source(Id::new(("palette", kind)), DragPayload::New(kind), |ui| {
        let row = ui.with_layout(Layout::left_to_right(Align::Min), |ui| {
            ui.add(Icon::new(icons.kind(kind)).size(20.0));
            ui.vertical(|ui| {
                ui.style_mut().spacing.item_spacing.y = 1.0;
                let name = style.body.clone().weight(600);
                ui.add(Label::new(spec.name).style(name));
                let small = TextStyle::proportional(12.0);
                ui.add(Label::new(spec.description).style(small).color(weak));
            });
        });
        // The whole row reacts to clicks (the drag source only drags).
        let rect = row.response.rect;
        let rect = rustroke::Rect::from_min_size(rect.min, rustroke::vec2(width, rect.height()));
        let response = ui.interact(Id::new(("palette item", kind)), rect, Sense::CLICK);
        if response.hovered() {
            let accent = ui.style().visuals.accent.with_alpha(0.6);
            let stroke = rustroke::Stroke::new(1.0, accent);
            ui.painter().rect_stroke(rect.expand(2.0), 4.0, stroke);
        }
        // While dragged the content follows the pointer, so the release
        // lands on it: that is a drop, not a click.
        clicked = response.clicked() && !ui.ctx().is_dnd_active();
        ui.describe(
            &response,
            rustroke::WidgetInfo::new(rustroke::WidgetRole::Button, spec.name),
        );
    });
    if clicked {
        if app.can_add(kind) {
            app.add_widget(kind);
        } else {
            app.status = format!("{} can't go here: drag it where it belongs", spec.name);
        }
    }
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    let icons = app.icons();
    ui.heading("Widgets");
    let mut filter = std::mem::take(&mut app.palette.filter);
    ui.add(
        SearchField::new(&mut filter)
            .hint_text("Search widgets")
            .accessible_label("Search widgets")
            .desired_width(ui.available_width()),
    );
    ui.separator();
    ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
        if filter.trim().is_empty() {
            for category in Category::ALL {
                CollapsingHeader::new(category.name())
                    .icon(icons.category(category))
                    .default_open(true)
                    .show(ui, |ui| {
                        for kind in WidgetKind::ALL
                            .into_iter()
                            .filter(|k| k.spec().category == category)
                        {
                            item(ui, app, &icons, kind);
                        }
                    });
            }
        } else {
            let found: Vec<_> = WidgetKind::ALL
                .into_iter()
                .filter(|k| matches(*k, &filter))
                .collect();
            if found.is_empty() {
                let weak = ui.style().visuals.weak_text;
                ui.add(Label::new("No widgets match.").color(weak));
            }
            for kind in found {
                item(ui, app, &icons, kind);
            }
        }
    });
    app.palette.filter = filter;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_matches_names_descriptions_and_categories() {
        assert!(matches(WidgetKind::Slider, "slid"));
        assert!(matches(WidgetKind::Checkbox, "BOOL"));
        assert!(matches(WidgetKind::Grid, "containers columns"));
        assert!(!matches(WidgetKind::Label, "slider"));
    }
}
