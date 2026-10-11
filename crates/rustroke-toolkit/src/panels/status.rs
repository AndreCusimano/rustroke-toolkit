//! Status bar: last message, code errors, project folder.

use rustroke::{Justify, Label, Stack, StackAlign, Ui};

use crate::app::Toolkit;

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    // The message at the start, the project folder at the end.
    Stack::row()
        .fill_width()
        .justify(Justify::SpaceBetween)
        .align(StackAlign::Center)
        .show(ui, |ui| {
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
            let weak = ui.style().visuals.weak_text;
            let path = app
                .path
                .as_ref()
                .map_or("Not saved yet".to_owned(), |p| p.display().to_string());
            ui.add(Label::new(path).color(weak).no_wrap());
        });
}
