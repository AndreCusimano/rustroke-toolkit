//! Modal dialogs: new view, rename, delete, unsaved changes, stale files.

use rtk_document::WindowSettings;
use rtk_document::names::check_snake_case;
use rustroke::{Align, Frame, Label, Layout, Modal, PropertyGrid, TextEdit, Ui};

use super::properties::window_size_rows;

use crate::app::{Dialog, Toolkit};
use crate::project_io::{self, Then};

/// What the user chose in a dialog.
enum Outcome {
    /// Keep the dialog open.
    Open,
    /// Close it, nothing else.
    Cancel,
    /// Close it and do this.
    Done(Box<dyn FnOnce(&mut Toolkit)>),
}

/// A name field with validation; returns whether the name can be used.
fn name_field(ui: &mut Ui<'_>, name: &mut String, taken: bool) -> bool {
    let response = ui.add(
        TextEdit::singleline(name)
            .desired_width(280.0)
            .select_all_on_focus(true)
            .accessible_label("Name"),
    );
    // Typing goes to the name as soon as the dialog opens.
    if ui.ctx().focused().is_none() {
        ui.ctx().request_focus(response.id);
    }
    let error = match check_snake_case(name) {
        Err(err) => Some(err.to_string()),
        Ok(()) if taken => Some("a view with this name exists".to_owned()),
        Ok(()) => None,
    };
    let weak = ui.style().visuals.weak_text;
    match &error {
        Some(err) => {
            let red = ui.style().visuals.error;
            ui.add(Label::new(err.clone()).color(red));
        }
        None => {
            ui.add(Label::new(format!("File: {name}.rs")).color(weak));
        }
    }
    error.is_none()
}

/// "Cancel" and the main button, right-aligned; returns which was clicked.
fn buttons(ui: &mut Ui<'_>, ok: &str, ok_enabled: bool) -> (bool, bool) {
    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
        let ok = ui
            .add_enabled(ok_enabled, rustroke::Button::new(ok))
            .clicked();
        let cancel = ui.button("Cancel").clicked();
        (cancel, ok)
    })
    .inner
}

pub fn show(frame: &mut Frame<'_>, app: &mut Toolkit) {
    let Some(dialog) = app.dialog.as_mut() else {
        return;
    };
    let views: Vec<String> = app.project.views.iter().map(|v| v.name.clone()).collect();
    let enter = frame.ctx().input().key_pressed(rustroke::Key::Enter);
    let response =
        match dialog {
            Dialog::NewView {
                name,
                window,
                title_edited,
            } => Modal::new("new view")
                .title("New window")
                .show(frame, |ui| {
                    ui.label("Every view is a window: name it and choose its size.");
                    ui.add_space(4.0);
                    let mut ok = false;
                    PropertyGrid::new("new view").show(ui, |grid| {
                        ok = grid.row("Name", |ui| {
                            ui.vertical(|ui| name_field(ui, name, views.contains(name)))
                                .inner
                        });
                        if !*title_edited {
                            *window = WindowSettings {
                                title: WindowSettings::for_view(name).title,
                                ..window.clone()
                            };
                        }
                        let title = grid.row("Title", |ui| {
                            ui.add(
                                TextEdit::singleline(&mut window.title)
                                    .desired_width(280.0)
                                    .accessible_label("Title"),
                            )
                        });
                        *title_edited |= title.changed();
                        window_size_rows(grid, window);
                    });
                    let (cancel, clicked) = buttons(ui, "Create", ok);
                    if cancel {
                        Outcome::Cancel
                    } else if ok && (clicked || enter) {
                        let (name, window) = (name.clone(), window.clone());
                        Outcome::Done(Box::new(move |app: &mut Toolkit| {
                            if app
                                .edit_project(|p| p.add_view_with_window(&name, window))
                                .is_some()
                            {
                                let index = app.project.views.len() - 1;
                                app.open_view(index);
                            }
                        }))
                    } else {
                        Outcome::Open
                    }
                }),
            Dialog::RenameView { old, name } => Modal::new("rename view")
                .title("Rename view")
                .show(frame, |ui| {
                    ui.label(format!("New name of {old}.rs:"));
                    let ok = name_field(ui, name, name != old && views.contains(name));
                    let (cancel, clicked) = buttons(ui, "Rename", ok);
                    if cancel {
                        Outcome::Cancel
                    } else if ok && (clicked || enter) {
                        let (old, name) = (old.clone(), name.clone());
                        Outcome::Done(Box::new(move |app: &mut Toolkit| {
                            if app.edit_project(|p| p.rename_view(&old, &name)).is_some() {
                                if let Some(pos) = app.window_positions.remove(&old) {
                                    app.window_positions.insert(name.clone(), pos);
                                }
                                if let Some(state) = app.canvases.remove(&old) {
                                    app.canvases.insert(name, state);
                                }
                            }
                        }))
                    } else {
                        Outcome::Open
                    }
                }),
            Dialog::DeleteView { name } => {
                Modal::new("delete view")
                    .title("Delete view")
                    .show(frame, |ui| {
                        ui.label(format!(
                    "Delete the view {name}? Its file is deleted at the next save, after asking."
                ));
                        let (cancel, ok) = buttons(ui, "Delete", true);
                        if cancel {
                            Outcome::Cancel
                        } else if ok {
                            let name = name.clone();
                            Outcome::Done(Box::new(move |app: &mut Toolkit| {
                                app.edit_project(|p| p.remove_view(&name));
                                app.selected = None;
                            }))
                        } else {
                            Outcome::Open
                        }
                    })
            }
            Dialog::UnsavedChanges { then } => {
                Modal::new("unsaved")
                    .title("Save changes?")
                    .show(frame, |ui| {
                        ui.label("The project has changes that aren't saved.");
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let then = then.clone();
                            if ui.button("Save").clicked() {
                                return Outcome::Done(Box::new(move |app: &mut Toolkit| {
                                    app.after_save = Some(then);
                                    project_io::save(app);
                                }));
                            }
                            if ui.button("Cancel").clicked() {
                                return Outcome::Cancel;
                            }
                            if ui.button("Don't save").clicked() {
                                return Outcome::Done(Box::new(move |app: &mut Toolkit| {
                                    if then == Then::Close {
                                        app.dirty = false;
                                    }
                                    project_io::run(app, then);
                                }));
                            }
                            Outcome::Open
                        })
                        .inner
                    })
            }
            Dialog::StaleFiles { dir, files } => Modal::new("stale files")
                .title("Old view files")
                .show(frame, |ui| {
                    ui.label("These files belong to views that were renamed or deleted:");
                    for file in files.iter() {
                        let name = file.file_name().map(|n| n.to_string_lossy().into_owned());
                        ui.label(format!("  {}", name.unwrap_or_default()));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let (dir, files) = (dir.clone(), files.clone());
                        if ui.button("Delete them").clicked() {
                            return Outcome::Done(Box::new(move |app: &mut Toolkit| {
                                project_io::delete_stale_and_write(app, &dir, &files);
                            }));
                        }
                        if ui.button("Keep them").clicked() {
                            return Outcome::Done(Box::new(move |app: &mut Toolkit| {
                                project_io::write(app, &dir);
                            }));
                        }
                        if ui.button("Cancel").clicked() {
                            return Outcome::Cancel;
                        }
                        Outcome::Open
                    })
                    .inner
                }),
        };
    let outcome = if response.should_close() {
        Outcome::Cancel
    } else {
        response.inner
    };
    match outcome {
        Outcome::Open => {}
        Outcome::Cancel => {
            app.dialog = None;
            app.after_save = None;
        }
        Outcome::Done(action) => {
            app.dialog = None;
            action(app);
        }
    }
}
