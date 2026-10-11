//! The whole app without a window: menus, canvas, tree, properties, code.

use rtk_document::{WidgetKind, sample_project};
use rustroke::testing::Harness;
use rustroke_toolkit::Toolkit;

fn harness() -> Harness {
    Harness::with_size(1500.0, 900.0)
}

fn settle(harness: &mut Harness, app: &mut Toolkit) {
    for _ in 0..3 {
        harness.run(app);
    }
}

#[test]
fn screenshot() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "Remember me"));
    settle(&mut harness, &mut app);
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    harness
        .render_software()
        .save_png(dir.join("toolkit.png"))
        .unwrap();

    // A styled button: its Style and Size sections.
    let button = app.project.views[0].nodes[5].children[1].id;
    app.selected = Some(button);
    settle(&mut harness, &mut app);
    let shot = harness.render_software();
    shot.save_png(dir.join("toolkit-style.png")).unwrap();

    // A resizable window, selected (nothing inside it selected).
    app.open_view(1);
    settle(&mut harness, &mut app);
    let shot = harness.render_software();
    shot.save_png(dir.join("toolkit-resizable.png")).unwrap();

    // Panels inside a view, stacks, charts; the chart card selected.
    app.open_view(2);
    settle(&mut harness, &mut app);
    let card = app.project.views[2].nodes[2].children[0].children[0].id;
    app.selected = Some(card);
    settle(&mut harness, &mut app);
    let shot = harness.render_software();
    shot.save_png(dir.join("toolkit-zones.png")).unwrap();

    assert!(harness.click(&mut app, "New view"));
    settle(&mut harness, &mut app);
    let shot = harness.render_software();
    shot.save_png(dir.join("toolkit-new-view.png")).unwrap();

    // The palette, filtered.
    harness.key(&mut app, rustroke::Key::Escape, rustroke::Modifiers::NONE);
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "Search widgets"));
    harness.type_text(&mut app, "text");
    settle(&mut harness, &mut app);
    let shot = harness.render_software();
    shot.save_png(dir.join("toolkit-search.png")).unwrap();
}

#[test]
fn add_a_widget_from_the_menu() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let before = app.project.views[0].nodes.len();
    assert!(harness.click(&mut app, "Label"), "the palette item");
    settle(&mut harness, &mut app);
    let view = &app.project.views[0];
    assert_eq!(view.nodes.len(), before + 1);
    assert_eq!(view.nodes.last().unwrap().kind, WidgetKind::Label);
    assert_eq!(app.selected, Some(view.nodes.last().unwrap().id));
    assert!(
        app.code.text.contains("ui.label(\"Label\");"),
        "{}",
        app.code.text
    );
    assert!(app.dirty);
}

#[test]
fn typing_code_updates_the_canvas() {
    use rustroke::{Key, Modifiers};
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    // Near the top: the editor is taller than the window.
    let editor = harness.find("View code").expect("code editor").rect;
    click_at(
        &mut harness,
        &mut app,
        editor.min + rustroke::vec2(40.0, 20.0),
    );
    harness.key(&mut app, Key::A, Modifiers::COMMAND);
    let code = "use rustroke::Ui;\n\n#[derive(Debug, Default)]\npub struct Login {\n    pub ok: bool,\n}\n\nimpl Login {\n    pub fn show(&mut self, ui: &mut Ui<'_>) {\n        ui.heading(\"Typed\");\n        ui.checkbox(&mut self.ok, \"Ok\");\n    }\n}\n";
    harness.type_text(&mut app, code);
    // Not parsed while typing…
    assert_ne!(app.project.views[0].nodes.len(), 2);
    // …but after a short pause.
    harness.advance_time(0.3);
    settle(&mut harness, &mut app);
    let view = &app.project.views[0];
    assert_eq!(view.nodes.len(), 2, "{:?}", app.code.error);
    assert_eq!(view.nodes[0].text("text"), "Typed");
    assert!(
        harness.find("Ok").is_some(),
        "the canvas shows the new checkbox"
    );

    // A syntax error keeps the last good view and shows where it is.
    harness.type_text(&mut app, "fn broken(");
    harness.advance_time(0.3);
    settle(&mut harness, &mut app);
    assert!(app.code.error.is_some());
    assert_eq!(app.project.views[0].nodes.len(), 2);
}

#[test]
fn editing_a_property_updates_the_code() {
    use rustroke::{Key, Modifiers};
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "Remember me"));
    assert!(harness.click(&mut app, "Text property"));
    harness.key(&mut app, Key::A, Modifiers::COMMAND);
    harness.type_text(&mut app, "Keep me signed in");
    settle(&mut harness, &mut app);
    let checkbox = &app.project.views[0].nodes[3];
    assert_eq!(checkbox.text("text"), "Keep me signed in");
    // Only the code editor's own focus holds back regeneration.
    assert!(
        app.code.text.contains("\"Keep me signed in\""),
        "{}",
        app.code.text
    );
}

#[test]
fn style_properties() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let cancel = on_canvas(&harness, "Cancel").center();
    click_at(&mut harness, &mut app, cancel);
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "Set Background"));
    settle(&mut harness, &mut app);
    assert!(
        app.code
            .text
            .contains("Button::new(\"Cancel\").fill(Color::from_srgb8(128, 128, 128))"),
        "{}",
        app.code.text
    );
    assert!(harness.click(&mut app, "Set Fixed size"));
    settle(&mut harness, &mut app);
    // Formatted over several lines.
    let compact: String = app.code.text.split_whitespace().collect();
    assert!(
        compact.contains("ui.add_sized(vec2(100.0,28.0),Button::new(\"Cancel\")"),
        "{}",
        app.code.text
    );
    // Unset again: back to the shorthand.
    assert!(harness.click(&mut app, "Set Background"));
    assert!(harness.click(&mut app, "Set Fixed size"));
    settle(&mut harness, &mut app);
    assert!(
        app.code.text.contains("ui.button(\"Cancel\");"),
        "{}",
        app.code.text
    );
}

#[test]
fn new_view_dialog_and_delete_key() {
    use rustroke::{Key, Modifiers};
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "New view"));
    settle(&mut harness, &mut app);
    // The name is focused and selected: typing replaces it.
    harness.type_text(&mut app, "Bad Name");
    settle(&mut harness, &mut app);
    assert!(
        harness.find("Create").is_some_and(|w| !w.enabled),
        "invalid names can't be created"
    );
    harness.key(&mut app, Key::A, Modifiers::COMMAND);
    harness.type_text(&mut app, "profile");
    settle(&mut harness, &mut app);
    assert!(harness.click(&mut app, "Create"));
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views.last().unwrap().name, "profile");
    assert_eq!(app.view().unwrap().name, "profile");

    // Add a button, then delete it with the Delete key.
    assert!(harness.click(&mut app, "Button"), "the palette item");
    settle(&mut harness, &mut app);
    assert_eq!(app.view().unwrap().nodes.len(), 1);
    harness.key(&mut app, Key::Delete, Modifiers::NONE);
    settle(&mut harness, &mut app);
    assert!(app.view().unwrap().nodes.is_empty());
}

/// Presses at `from`, moves to `to` in steps (one frame each), releases.
fn drag(harness: &mut Harness, app: &mut Toolkit, from: rustroke::Point, to: rustroke::Point) {
    use rustroke::{Event, Modifiers, PointerButton};
    let button = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.run_with_events(app, vec![Event::PointerMoved(from), button(from, true)]);
    for i in 1..=8 {
        let t = i as f32 / 8.0;
        let pos = from + (to - from) * t;
        harness.run_with_events(app, vec![Event::PointerMoved(pos)]);
    }
    harness.run_with_events(app, vec![Event::PointerMoved(to)]);
    harness.run_with_events(app, vec![button(to, false)]);
    settle(harness, app);
}

fn center(harness: &Harness, label: &str) -> rustroke::Point {
    harness
        .find(label)
        .unwrap_or_else(|| panic!("no widget {label:?}"))
        .rect
        .center()
}

/// The component-tree row with this label (palette items have the same
/// names).
fn in_tree(harness: &Harness, label: &str) -> rustroke::Rect {
    // The tree's column starts at its "Views" title.
    let views_x = harness.find("Views").expect("views panel").rect.min.x - 30.0;
    harness
        .widgets()
        .iter()
        .filter(|w| w.info.label == label && w.rect.min.x > views_x)
        .min_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x))
        .unwrap_or_else(|| panic!("no tree row {label:?}"))
        .rect
}

/// The canvas widget with this label (rightmost of the duplicates).
fn on_canvas(harness: &Harness, label: &str) -> rustroke::Rect {
    harness
        .widgets()
        .iter()
        .filter(|w| w.info.label == label)
        .max_by(|a, b| a.rect.min.x.total_cmp(&b.rect.min.x))
        .unwrap_or_else(|| panic!("no widget {label:?}"))
        .rect
}

#[test]
fn drag_from_the_palette_to_the_canvas() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let before = app.project.views[0].nodes.len();
    // Just below the last link of the sign-in window: the end of the view.
    let link = on_canvas(&harness, "https://example.com");
    let to = rustroke::point(link.min.x + 10.0, link.max.y + 10.0);
    let from = center(&harness, "Checkbox");
    drag(&mut harness, &mut app, from, to);
    let view = &app.project.views[0];
    assert_eq!(view.nodes.len(), before + 1, "{}", app.status);
    assert_eq!(view.nodes.last().unwrap().kind, WidgetKind::Checkbox);
    assert_eq!(app.selected, Some(view.nodes.last().unwrap().id));

    // Onto the left half of "Cancel": before it, in the row.
    let cancel = on_canvas(&harness, "Cancel");
    let to = rustroke::point(cancel.min.x + 3.0, cancel.center().y);
    let from = center(&harness, "Slider");
    drag(&mut harness, &mut app, from, to);
    let row = app.project.views[0]
        .nodes
        .iter()
        .find(|n| n.kind == WidgetKind::Stack && n.text("direction") == "row")
        .unwrap();
    let kinds: Vec<_> = row.children.iter().map(|n| n.kind).collect();
    assert_eq!(
        kinds,
        [WidgetKind::Slider, WidgetKind::Button, WidgetKind::Button]
    );
}

#[test]
fn drag_on_the_canvas() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let kinds = |app: &Toolkit| -> Vec<WidgetKind> {
        app.project.views[0].nodes.iter().map(|n| n.kind).collect()
    };
    let before = kinds(&app);
    // The checkbox, onto the top half of the heading: before it.
    let heading = on_canvas(&harness, "Sign in");
    let heading = harness
        .widgets()
        .iter()
        .filter(|w| w.info.label == "Sign in" && w.rect.min.x > heading.min.x - 200.0)
        .min_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y))
        .unwrap()
        .rect;
    let from = on_canvas(&harness, "Remember me").center();
    let to = rustroke::point(heading.center().x, heading.min.y + 2.0);
    drag(&mut harness, &mut app, from, to);
    let after = kinds(&app);
    assert_eq!(after.len(), before.len(), "moved, not copied");
    assert_eq!(after[0], WidgetKind::Checkbox, "{}: {after:?}", app.status);
    assert_eq!(app.selected, Some(app.project.views[0].nodes[0].id));

    // A click still selects.
    let cancel = on_canvas(&harness, "Cancel").center();
    click_at(&mut harness, &mut app, cancel);
    settle(&mut harness, &mut app);
    let row = &app.project.views[0].nodes[5];
    assert_eq!(app.selected, Some(row.children[0].id));
    assert_eq!(kinds(&app), after, "nothing moved");
}

#[test]
fn drag_in_and_onto_the_tree() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    // Onto the middle of the "Horizontal" row: inside the row, at the end.
    let row = in_tree(&harness, "Stack (row)").center();
    let from = center(&harness, "Spinner");
    drag(&mut harness, &mut app, from, row);
    let horizontal = app.project.views[0]
        .nodes
        .iter()
        .find(|n| n.kind == WidgetKind::Stack && n.text("direction") == "row")
        .unwrap();
    assert_eq!(
        horizontal.children.last().unwrap().kind,
        WidgetKind::Spinner,
        "{}",
        app.status
    );

    // Drag the separator above the heading: first in the view.
    let heading = harness.find("Heading \"Sign in\"").unwrap().rect;
    let to = rustroke::point(heading.center().x, heading.min.y + 2.0);
    let from = in_tree(&harness, "Separator").center();
    drag(&mut harness, &mut app, from, to);
    let first = &app.project.views[0].nodes[0];
    assert_eq!(first.kind, WidgetKind::Separator, "{}", app.status);
    // A click on a row still selects it.
    let space = in_tree(&harness, "Space").center();
    click_at(&mut harness, &mut app, space);
    assert_eq!(
        app.selected
            .and_then(|id| app.project.views[0].node(id))
            .map(|n| n.kind),
        Some(WidgetKind::Space)
    );
}

#[test]
fn palette_search_and_window_drag() {
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    assert!(
        harness.find("Containers").is_some(),
        "sections while not searching"
    );
    assert!(harness.click(&mut app, "Search widgets"));
    harness.type_text(&mut app, "slider");
    settle(&mut harness, &mut app);
    assert!(
        harness.find("Containers").is_none(),
        "a flat list while searching"
    );
    assert!(harness.find("Slider").is_some());
    assert!(harness.find("Checkbox").is_none());

    // Drag the window by its title bar.
    // The heading is the topmost "Sign in" (the button has the same text).
    let heading = harness
        .widgets()
        .iter()
        .filter(|w| w.info.label == "Sign in")
        .min_by(|a, b| a.rect.min.y.total_cmp(&b.rect.min.y))
        .unwrap()
        .rect;
    let margin = harness.ctx().style().spacing.window_margin;
    let title = rustroke::point(heading.min.x + 120.0, heading.min.y - margin - 15.0);
    let to = title + rustroke::vec2(60.0, 40.0);
    drag(&mut harness, &mut app, title, to);
    let pos = app.window_positions.get("login").copied().expect("moved");
    assert!(pos.x > 48.0 && pos.y > 48.0, "{pos:?}");
}

fn click_at(harness: &mut Harness, app: &mut Toolkit, pos: rustroke::Point) {
    use rustroke::{Event, Modifiers, PointerButton};
    let button = |pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    harness.run_with_events(
        app,
        vec![Event::PointerMoved(pos), button(true), button(false)],
    );
    settle(harness, app);
}

#[test]
fn undo_redo() {
    use rustroke::{Key, Modifiers};
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let original = app.project.views[0].clone();
    assert!(harness.click(&mut app, "Label"), "palette item");
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views[0].nodes.len(), original.nodes.len() + 1);

    harness.key(&mut app, Key::Z, Modifiers::COMMAND);
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views[0], original, "undone");
    harness.key(&mut app, Key::Z, Modifiers::COMMAND.plus(Modifiers::SHIFT));
    settle(&mut harness, &mut app);
    assert_eq!(
        app.project.views[0].nodes.len(),
        original.nodes.len() + 1,
        "redone"
    );

    // Typing in a property is one step.
    assert!(harness.click(&mut app, "Remember me"));
    assert!(harness.click(&mut app, "Text property"));
    harness.key(&mut app, Key::A, Modifiers::COMMAND);
    for word in ["Keep", " me", " in"] {
        harness.type_text(&mut app, word);
    }
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views[0].nodes[3].text("text"), "Keep me in");
    // Selecting another widget takes the keyboard from the field.
    assert!(harness.click(&mut app, "Cancel"));
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views[0].nodes[3].text("text"), "Keep me in");
    harness.key(&mut app, Key::Z, Modifiers::COMMAND);
    settle(&mut harness, &mut app);
    assert_eq!(
        app.project.views[0].nodes[3].text("text"),
        "Remember me",
        "one step"
    );
}

#[test]
fn copy_cut_paste() {
    use rtk_document::WidgetKind as K;
    use rustroke::{Key, Modifiers};
    let mut app = Toolkit::new(sample_project());
    let mut harness = harness();
    settle(&mut harness, &mut app);
    let count = |app: &Toolkit| app.project.views[0].nodes.len();
    let before = count(&app);

    assert!(harness.click(&mut app, "Remember me"));
    harness.key(&mut app, Key::C, Modifiers::COMMAND);
    harness.key(&mut app, Key::V, Modifiers::COMMAND);
    settle(&mut harness, &mut app);
    let view = &app.project.views[0];
    assert_eq!(count(&app), before + 1, "{}", app.status);
    assert_eq!(view.nodes[4].kind, K::Checkbox, "pasted after the original");
    assert_eq!(view.nodes[4].text("binding"), "remember_2", "its own field");
    assert_eq!(app.selected, Some(view.nodes[4].id));

    // Cut the copy, paste it into another view.
    harness.key(&mut app, Key::X, Modifiers::COMMAND);
    settle(&mut harness, &mut app);
    assert_eq!(count(&app), before);
    let empty = app
        .project
        .views
        .iter()
        .position(|v| v.name == "empty")
        .expect("the sample's empty view");
    app.open_view(empty);
    harness.key(&mut app, Key::V, Modifiers::COMMAND);
    settle(&mut harness, &mut app);
    assert_eq!(app.project.views[empty].nodes.len(), 1);
    assert_eq!(app.project.views[empty].nodes[0].kind, K::Checkbox);
}
