//! The code generated for the example project must match the files in
//! `rtk-codegen-check/src/views/`, which the workspace build compiles (and
//! clippy checks). After an intended change to the generator:
//! `UPDATE_SNAPSHOTS=1 cargo test -p rtk-document --test codegen_snapshots`.

use std::path::PathBuf;

use rtk_document::codegen::{mod_code, view_code};
use rtk_document::sample_project;

#[test]
fn generated_code_matches_the_compiled_snapshots() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../rtk-codegen-check/src/views");
    let project = sample_project();
    let mut files = vec![("mod.rs".to_owned(), mod_code(&project))];
    for view in &project.views {
        files.push((format!("{}.rs", view.name), view_code(view)));
    }
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut stale = Vec::new();
    for (name, code) in files {
        let path = dir.join(&name);
        if update {
            std::fs::write(&path, &code).unwrap();
        } else if std::fs::read_to_string(&path).ok().as_deref() != Some(code.as_str()) {
            stale.push(name);
        }
    }
    assert!(
        stale.is_empty(),
        "generated code differs from the snapshots: {stale:?}; \
         rerun with UPDATE_SNAPSHOTS=1, then build the workspace"
    );
}
