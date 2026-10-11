//! Status bar: last message, code errors, project folder.

use rustroke::{Align, Label, Layout, Ui};

use crate::app::Toolkit;

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    ui.horizontal(|ui| {
        if let Some(err) = &app.code.error {
            let red = ui.style().visuals.error;
            ui.add(
                Label::new(format!("Code error at {err}"))
                    .color(red)
                    .no_wrap(),
            );
        } else {
            ui.add(Label::new(&app.status).no_wrap());
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let weak = ui.style().visuals.weak_text;
            let path = app
                .path
                .as_ref()
                .map_or("Not saved yet".to_owned(), |p| p.display().to_string());
            ui.add(Label::new(path).color(weak).no_wrap());
        });
    });
}
