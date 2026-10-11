//! Projects on disk: a folder with one `<view>.rs` per view, a `mod.rs`
//! and a small manifest (`rustroke-toolkit.ron`) with the view order.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::codegen::{mod_code, view_code};
use crate::model::Project;
use crate::parser::{ParseError, parse_view};

/// File name of the manifest.
pub const MANIFEST: &str = "rustroke-toolkit.ron";

/// Version of the manifest format.
const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Serialize, Deserialize)]
struct Manifest {
    version: u32,
    name: String,
    views: Vec<String>,
}

/// Why a project couldn't be opened or saved.
#[derive(Debug)]
pub enum ProjectError {
    /// Reading or writing a file failed.
    Io(PathBuf, std::io::Error),
    /// The manifest is not valid.
    Manifest(String),
    /// A view file doesn't parse.
    Parse(PathBuf, ParseError),
}

impl std::fmt::Display for ProjectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, err) => write!(f, "{}: {err}", path.display()),
            Self::Manifest(err) => write!(f, "{MANIFEST}: {err}"),
            Self::Parse(path, err) => write!(f, "{}:{err}", path.display()),
        }
    }
}

impl std::error::Error for ProjectError {}

/// The files of a project: `(file name, content)`.
pub fn project_files(project: &Project) -> Vec<(String, String)> {
    let manifest = Manifest {
        version: FORMAT_VERSION,
        name: project.name.clone(),
        views: project.views.iter().map(|v| v.name.clone()).collect(),
    };
    let manifest = ron::ser::to_string_pretty(&manifest, ron::ser::PrettyConfig::default())
        .expect("the manifest serializes");
    let mut files = vec![
        (MANIFEST.to_owned(), manifest + "\n"),
        ("mod.rs".to_owned(), mod_code(project)),
    ];
    for view in &project.views {
        files.push((format!("{}.rs", view.name), view_code(view)));
    }
    files
}

/// View files in `dir` that `project` no longer has: files of views listed
/// in the folder's manifest (so written by the toolkit) that were renamed
/// or removed since. They are left alone unless the caller deletes them.
pub fn stale_files(project: &Project, dir: &Path) -> Vec<PathBuf> {
    let Ok(text) = std::fs::read_to_string(dir.join(MANIFEST)) else {
        return Vec::new();
    };
    let Ok(manifest) = ron::from_str::<Manifest>(&text) else {
        return Vec::new();
    };
    manifest
        .views
        .iter()
        .filter(|name| project.view(name).is_none())
        .map(|name| dir.join(format!("{name}.rs")))
        .filter(|path| path.is_file())
        .collect()
}

/// Writes the project into `dir` (created if missing). Other files in the
/// folder are not touched.
pub fn save_project(project: &Project, dir: &Path) -> Result<(), ProjectError> {
    std::fs::create_dir_all(dir).map_err(|e| ProjectError::Io(dir.to_owned(), e))?;
    for (name, content) in project_files(project) {
        let path = dir.join(name);
        // Leave files that didn't change alone (keeps their timestamps for
        // build tools watching the folder).
        if std::fs::read_to_string(&path).ok().as_deref() == Some(content.as_str()) {
            continue;
        }
        std::fs::write(&path, content).map_err(|e| ProjectError::Io(path, e))?;
    }
    Ok(())
}

/// Opens the project in `dir`. Without a manifest, every `.rs` file but
/// `mod.rs` is a view, in name order, and the project takes the folder's
/// name.
pub fn load_project(dir: &Path) -> Result<Project, ProjectError> {
    let manifest_path = dir.join(MANIFEST);
    let (name, views) = match std::fs::read_to_string(&manifest_path) {
        Ok(text) => {
            let manifest: Manifest =
                ron::from_str(&text).map_err(|e| ProjectError::Manifest(e.to_string()))?;
            if manifest.version > FORMAT_VERSION {
                return Err(ProjectError::Manifest(format!(
                    "made by a newer version of the toolkit (format {})",
                    manifest.version
                )));
            }
            (manifest.name, manifest.views)
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            let entries =
                std::fs::read_dir(dir).map_err(|e| ProjectError::Io(dir.to_owned(), e))?;
            let mut views: Vec<String> = entries
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    let stem = path.file_stem()?.to_str()?.to_owned();
                    (path.extension()? == "rs"
                        && stem != "mod"
                        && crate::names::check_snake_case(&stem).is_ok())
                    .then_some(stem)
                })
                .collect();
            views.sort();
            let name = dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project")
                .to_owned();
            (name, views)
        }
        Err(err) => return Err(ProjectError::Io(manifest_path, err)),
    };
    let mut project = Project {
        name,
        views: Vec::new(),
    };
    for view in views {
        let path = dir.join(format!("{view}.rs"));
        let source =
            std::fs::read_to_string(&path).map_err(|e| ProjectError::Io(path.clone(), e))?;
        let view = parse_view(&view, &source).map_err(|e| ProjectError::Parse(path, e))?;
        project.views.push(view);
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("rtk-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn save_and_open_again() {
        let dir = temp_dir("save");
        let project = crate::sample_project();
        save_project(&project, &dir).unwrap();
        let loaded = load_project(&dir).unwrap();
        assert_eq!(loaded.name, project.name);
        assert_eq!(
            loaded
                .views
                .iter()
                .map(|v| v.name.as_str())
                .collect::<Vec<_>>(),
            ["login", "settings", "dashboard", "empty"]
        );
        assert_eq!(project_files(&loaded), project_files(&project));

        // A renamed view leaves its old file, reported as stale.
        let mut renamed = loaded.clone();
        renamed.rename_view("login", "sign_in").unwrap();
        assert_eq!(stale_files(&renamed, &dir), [dir.join("login.rs")]);
        save_project(&renamed, &dir).unwrap();
        assert!(dir.join("sign_in.rs").is_file());
        assert!(
            stale_files(&renamed, &dir).is_empty(),
            "the manifest was rewritten"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn open_a_folder_without_manifest() {
        let dir = temp_dir("plain");
        std::fs::create_dir_all(&dir).unwrap();
        let view = crate::model::View::new("b_view");
        std::fs::write(dir.join("b_view.rs"), view_code(&view)).unwrap();
        std::fs::write(
            dir.join("a_view.rs"),
            view_code(&crate::model::View::new("a_view")),
        )
        .unwrap();
        std::fs::write(dir.join("mod.rs"), "pub mod a_view;\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a view").unwrap();
        let project = load_project(&dir).unwrap();
        let names: Vec<_> = project.views.iter().map(|v| v.name.as_str()).collect();
        assert_eq!(names, ["a_view", "b_view"]);

        std::fs::write(dir.join("a_view.rs"), "fn broken(").unwrap();
        assert!(matches!(load_project(&dir), Err(ProjectError::Parse(..))));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
