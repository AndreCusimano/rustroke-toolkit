//! New, open and save, with native folder dialogs (rfd), and the guard
//! against losing unsaved changes.

use std::path::{Path, PathBuf};

use rtk_document::project::{load_project, save_project, stale_files};
use rtk_document::{Project, View};
use rustroke::Toast;

use crate::app::{Dialog, Toolkit};

/// What to do once the unsaved changes are saved or discarded.
#[derive(Clone, Debug, PartialEq)]
pub enum Then {
    /// Close the window.
    Close,
    /// Start a new project.
    NewProject,
    /// Pick a folder to open.
    OpenDialog,
    /// Open this folder.
    Open(PathBuf),
}

/// How many recent projects the menu lists.
const MAX_RECENT: usize = 8;

/// Runs `then` now, or asks first if there are unsaved changes.
pub fn guard(app: &mut Toolkit, then: Then) {
    if app.dirty {
        app.dialog = Some(Dialog::UnsavedChanges { then });
    } else {
        run(app, then);
    }
}

/// Runs `then` without asking.
pub fn run(app: &mut Toolkit, then: Then) {
    match then {
        Then::Close => app.close = true,
        Then::NewProject => replace(app, new_empty_project(), None),
        Then::OpenDialog => {
            if let Some(dir) = rfd::FileDialog::new()
                .set_title("Open a Rustroke Toolkit project")
                .pick_folder()
            {
                open(app, &dir);
            }
        }
        Then::Open(dir) => open(app, &dir),
    }
}

fn new_empty_project() -> Project {
    Project {
        name: "my_views".to_owned(),
        views: vec![View::new("main_view")],
    }
}

/// Puts `project` in the editor.
fn replace(app: &mut Toolkit, project: Project, path: Option<PathBuf>) {
    app.project = project;
    app.path = path;
    app.current = 0;
    app.selected = None;
    app.canvases.clear();
    app.window_positions.clear();
    app.history.clear();
    app.code.reset();
    app.revision += 1;
    app.dirty = false;
}

/// "New project" from the menu.
pub fn new_project(app: &mut Toolkit) {
    guard(app, Then::NewProject);
}

/// "Open…" from the menu.
pub fn open_dialog(app: &mut Toolkit) {
    guard(app, Then::OpenDialog);
}

/// Opens the project in `dir`.
pub fn open(app: &mut Toolkit, dir: &Path) {
    match load_project(dir) {
        Ok(project) => {
            replace(app, project, Some(dir.to_owned()));
            remember(app, dir);
            app.status = format!("Opened {}", dir.display());
        }
        Err(err) => {
            app.status = format!("Can't open: {err}");
            app.toasts
                .push(Toast::error(format!("Can't open the project: {err}")));
        }
    }
}

/// Moves `dir` to the top of the recent projects.
fn remember(app: &mut Toolkit, dir: &Path) {
    app.recent.retain(|p| p != dir);
    app.recent.insert(0, dir.to_owned());
    app.recent.truncate(MAX_RECENT);
    app.recent_changed = true;
}

/// "Save": to the project's folder, or asks for one.
pub fn save(app: &mut Toolkit) {
    let Some(dir) = app.path.clone() else {
        save_as(app);
        return;
    };
    let stale = stale_files(&app.project, &dir);
    if stale.is_empty() {
        write(app, &dir);
    } else {
        app.dialog = Some(Dialog::StaleFiles { dir, files: stale });
    }
}

/// "Save as…": asks for the folder (its name becomes the project's name).
pub fn save_as(app: &mut Toolkit) {
    let Some(dir) = rfd::FileDialog::new()
        .set_title("Save the project folder")
        .set_file_name(&app.project.name)
        .save_file()
    else {
        return;
    };
    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
        app.project.name = name.to_owned();
    }
    app.path = Some(dir);
    save(app);
}

/// Writes the files; on success runs the action waiting for the save.
pub fn write(app: &mut Toolkit, dir: &Path) {
    match save_project(&app.project, dir) {
        Ok(()) => {
            app.dirty = false;
            remember(app, dir);
            app.status = format!("Saved to {}", dir.display());
            app.toasts.push(Toast::success("Project saved"));
            if let Some(then) = app.after_save.take() {
                run(app, then);
            }
        }
        Err(err) => {
            app.after_save = None;
            app.status = format!("Can't save: {err}");
            app.toasts
                .push(Toast::error(format!("Can't save the project: {err}")));
        }
    }
}

/// Deletes the files of removed or renamed views, then saves.
pub fn delete_stale_and_write(app: &mut Toolkit, dir: &Path, files: &[PathBuf]) {
    for file in files {
        if let Err(err) = std::fs::remove_file(file) {
            app.toasts.push(Toast::error(format!(
                "Can't delete {}: {err}",
                file.display()
            )));
        }
    }
    write(app, dir);
}
