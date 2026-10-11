//! Menu bar: project and edit menus, and the design/preview and theme
//! switches. Widgets are added from the palette (`palette.rs`).

use rtk_preview::Mode;
use rustroke::{Align, Button, IconId, Layout, Ui};

use crate::app::{COPY, CUT, DELETE, DUPLICATE, Dialog, PASTE, REDO, SAVE, Toolkit, UNDO};
use crate::project_io::{self, Then};

/// A menu item with an icon.
fn item(ui: &mut Ui<'_>, icon: IconId, text: &str) -> bool {
    ui.add(Button::new(text).icon(icon)).clicked()
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    let icons = app.icons();
    ui.horizontal(|ui| {
        ui.menu_button_with_icon(icons.project, "Project", |ui| {
            if item(ui, icons.project_new, "New project") {
                project_io::new_project(app);
            }
            if item(ui, icons.project_open, "Open…") {
                project_io::open_dialog(app);
            }
            let recent = app.recent.clone();
            ui.menu_button_with_icon(icons.project_open_recent, "Open recent", |ui| {
                if recent.is_empty() {
                    ui.add_enabled(false, Button::new("No recent projects"));
                }
                for dir in recent {
                    if item(ui, icons.project_open_recent, &dir.display().to_string()) {
                        project_io::guard(app, Then::Open(dir));
                    }
                }
            });
            let save = Button::new("Save")
                .icon(icons.project_save)
                .shortcut_text(SAVE.format());
            if ui.add(save).clicked() {
                project_io::save(app);
            }
            if item(ui, icons.project_save_as, "Save as…") {
                project_io::save_as(app);
            }
            ui.separator();
            if item(ui, icons.view_new, "New view…") {
                app.dialog = Some(Dialog::new_view(&app.project));
            }
        });
        ui.menu_button_with_icon(icons.menu_edit, "Edit", |ui| {
            let selected = app.selected.is_some();
            let undo = Button::new("Undo")
                .icon(icons.edit_undo)
                .shortcut_text(UNDO.format());
            if ui.add_enabled(app.history.can_undo(), undo).clicked() {
                app.undo();
            }
            let redo = Button::new("Redo")
                .icon(icons.edit_redo)
                .shortcut_text(REDO.format());
            if ui.add_enabled(app.history.can_redo(), redo).clicked() {
                app.redo();
            }
            ui.separator();
            let cut = Button::new("Cut")
                .icon(icons.edit_cut)
                .shortcut_text(CUT.format());
            if ui.add_enabled(selected, cut).clicked() {
                app.cut_selected();
            }
            let copy = Button::new("Copy")
                .icon(icons.edit_copy)
                .shortcut_text(COPY.format());
            if ui.add_enabled(selected, copy).clicked() {
                app.copy_selected();
            }
            let paste = Button::new("Paste")
                .icon(icons.edit_paste)
                .shortcut_text(PASTE.format());
            if ui.add_enabled(app.clipboard.is_some(), paste).clicked() {
                app.paste();
            }
            ui.separator();
            let duplicate = Button::new("Duplicate")
                .icon(icons.edit_duplicate)
                .shortcut_text(DUPLICATE.format());
            if ui.add_enabled(selected, duplicate).clicked() {
                app.duplicate_selected();
            }
            let delete = Button::new("Delete")
                .icon(icons.edit_delete)
                .shortcut_text(DELETE.format());
            if ui.add_enabled(selected, delete).clicked() {
                app.delete_selected();
            }
            ui.separator();
            let up = Button::new("Move up").icon(icons.edit_move_up);
            if ui.add_enabled(selected, up).clicked() {
                app.move_selected(-1);
            }
            let down = Button::new("Move down").icon(icons.edit_move_down);
            if ui.add_enabled(selected, down).clicked() {
                app.move_selected(1);
            }
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (icon, text) = if app.dark {
                (icons.theme_light, "Light theme")
            } else {
                (icons.theme_dark, "Dark theme")
            };
            if ui.add(Button::new(text).icon(icon)).clicked() {
                app.dark = !app.dark;
            }
            ui.separator();
            for (mode, icon, text) in [
                (Mode::Preview, icons.mode_preview, "Preview"),
                (Mode::Design, icons.mode_design, "Design"),
            ] {
                let button = Button::new(text)
                    .icon(icon)
                    .frame(false)
                    .selected(app.mode == mode);
                if ui.add(button).clicked() {
                    app.mode = mode;
                }
            }
        });
    });
}
