//! Right panel: the code of the current view, editable.
//!
//! Sync rules: while the editor has the keyboard, the code is never
//! regenerated (the cursor would jump); edits are parsed after a short
//! pause and replace the view. Otherwise, whenever the project changed, the
//! code is regenerated from the model.

use rtk_document::codegen::view_code;
use rtk_document::parser::{ParseError, parse_view};
use rustroke::{Label, LineHighlight, ScrollArea, TextEdit, TextStyle, Ui};

use crate::app::Toolkit;
use crate::highlight::{Palette, highlight};

/// Seconds without typing before the code is parsed.
const PARSE_DELAY: f64 = 0.15;

/// State of the code editor.
#[derive(Debug, Default)]
pub struct CodePanel {
    /// The text in the editor.
    pub text: String,
    /// The view the text belongs to.
    view: String,
    /// The project revision the text matches.
    revision: u64,
    /// When the text was last edited and not parsed yet.
    edited_at: Option<f64>,
    /// The last parse error, while the text doesn't parse.
    pub error: Option<ParseError>,
}

impl CodePanel {
    /// Forgets the text, so it is regenerated next frame (after opening
    /// another project).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Parses the edited text and puts it in the view it belongs to (which may
/// no longer be the current one).
fn apply_text(app: &mut Toolkit) {
    let Some(index) = app
        .project
        .views
        .iter()
        .position(|v| v.name == app.code.view)
    else {
        return;
    };
    let view = &app.project.views[index];
    match parse_view(&view.name, &app.code.text) {
        Ok(parsed) => {
            if index == app.current {
                // Keep the selection on the node at the same place.
                let path = app.selected.and_then(|id| view.path(id));
                app.selected = path.and_then(|p| parsed.node_at_path(&p)).map(|n| n.id);
            }
            if parsed != *view {
                // Editing the code in a row (pauses under a second) is one
                // undo step.
                let before = app.snapshot();
                app.history.record(before, Some("code"), app.now);
                app.project.views[index] = parsed;
                app.touch();
                app.status = "Code applied".to_owned();
            }
            // The text is the source: don't regenerate it.
            app.code.revision = app.revision;
            app.code.error = None;
        }
        Err(err) => app.code.error = Some(err),
    }
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit, now: f64) {
    let Some(view) = app.project.views.get(app.current) else {
        return;
    };
    let view_name = view.name.clone();

    // Parse after a pause in typing, or right away when another view was
    // opened meanwhile.
    if let Some(at) = app.code.edited_at {
        if now - at >= PARSE_DELAY || app.code.view != view_name {
            app.code.edited_at = None;
            apply_text(app);
        } else {
            ui.ctx().request_repaint_after(PARSE_DELAY - (now - at));
        }
    }

    ui.horizontal(|ui| {
        ui.heading(format!("{view_name}.rs"));
    });
    if let Some(err) = &app.code.error {
        let red = ui.style().visuals.error;
        ui.add(Label::new(format!("{err}")).color(red));
    }
    ui.separator();

    let editor_id = ui.id().with("code editor");
    // Never regenerate under the user's cursor, or with edits not parsed yet.
    let typing = app.code.edited_at.is_some() || ui.ctx().focused() == Some(editor_id);
    let other_view = app.code.view != view_name;
    if other_view || (app.code.revision != app.revision && !typing) {
        if let Some(view) = app.project.views.get(app.current) {
            app.code.text = view_code(view);
        }
        app.code.view = view_name;
        app.code.revision = app.revision;
        app.code.error = None;
    }

    // Measured outside the scroll area, whose size is unbounded.
    let rows = ((ui.available_rect().height() / 18.0) as usize).clamp(10, 1000);
    let width = ui.available_width() - ui.style().spacing.scrollbar_width;
    let visuals = &ui.style().visuals;
    let palette = Palette::new(visuals.dark_mode);
    let error_color = visuals.error;
    let error_line = app.code.error.as_ref().map(|e| e.line.saturating_sub(1));
    ScrollArea::both().auto_shrink(false).show(ui, |ui| {
        let mut editor = TextEdit::multiline(&mut app.code.text)
            .id(editor_id)
            .font(TextStyle::monospace(13.0))
            .code_editor()
            .wrap(false)
            .line_numbers(true)
            .layouter(move |text| highlight(text, palette))
            .desired_width(width)
            .desired_rows(rows)
            .accessible_label("View code");
        if let Some(line) = error_line {
            editor = editor.highlight_line(line, error_color, LineHighlight::Underline);
        }
        let response = ui.add(editor);
        if response.changed() {
            app.code.edited_at = Some(now);
            ui.ctx().request_repaint_after(PARSE_DELAY);
        }
    });
}
