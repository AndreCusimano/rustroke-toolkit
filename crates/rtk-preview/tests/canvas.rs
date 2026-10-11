//! The canvas without a window: design-mode selection, preview-mode
//! interaction and a software-rendered picture of every sample view.

use rtk_document::{NodeId, View, WidgetKind, sample_project};
use rtk_preview::{BindingValue, CanvasOutput, CanvasState, Mode, show_view};
use rustroke::testing::Harness;
use rustroke::{App, Frame};

struct Canvas {
    view: View,
    state: CanvasState,
    mode: Mode,
    selected: Option<NodeId>,
    out: CanvasOutput,
}

impl Canvas {
    fn new(view: View, mode: Mode) -> Self {
        Self {
            view,
            state: CanvasState::default(),
            mode,
            selected: None,
            out: CanvasOutput::default(),
        }
    }

    fn find(&self, kind: WidgetKind, text: &str) -> NodeId {
        let mut found = None;
        self.view.walk(|n| {
            if n.kind == kind && n.text("text") == text {
                found = Some(n.id);
            }
        });
        found.unwrap_or_else(|| panic!("no {kind:?} {text:?}"))
    }
}

impl App for Canvas {
    fn update(&mut self, frame: &mut Frame) {
        frame.ui(|ui| {
            self.out = show_view(ui, &self.view, &mut self.state, self.mode, self.selected);
            if let Some(id) = self.out.clicked {
                self.selected = Some(id);
            } else if self.out.background_clicked {
                self.selected = None;
            }
        });
    }
}

fn login() -> View {
    sample_project().views.remove(0)
}

#[test]
fn design_mode_selects_instead_of_using_widgets() {
    let mut app = Canvas::new(login(), Mode::Design);
    let mut harness = Harness::with_size(800.0, 600.0);
    harness.run(&mut app);
    harness.run(&mut app);

    assert!(harness.click(&mut app, "Remember me"));
    let checkbox = app.find(WidgetKind::Checkbox, "Remember me");
    assert_eq!(app.selected, Some(checkbox));
    assert_eq!(
        app.state.bindings.get("remember"),
        Some(&BindingValue::Bool(false)),
        "not toggled"
    );

    assert!(harness.click(&mut app, "Cancel"));
    assert_eq!(app.selected, Some(app.find(WidgetKind::Button, "Cancel")));

    // Every visible node has an area.
    let mut missing = Vec::new();
    app.view.walk(|n| {
        if app.state.rect(n.id).is_none_or(|r| r.width() <= 0.0) {
            missing.push(n.summary());
        }
    });
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn clicking_a_container_selects_it_and_children_win() {
    let mut app = Canvas::new(login(), Mode::Design);
    let mut harness = Harness::with_size(800.0, 600.0);
    harness.run(&mut app);
    harness.run(&mut app);
    let row = app
        .view
        .nodes
        .iter()
        .find(|n| n.kind == WidgetKind::Stack && n.text("direction") == "row")
        .unwrap()
        .id;
    let rect = app.state.rect(row).unwrap();
    let cancel = app.find(WidgetKind::Button, "Cancel");
    let cancel_rect = app.state.rect(cancel).unwrap();
    // In the gap between the two buttons, inside the row.
    let pos = rustroke::point(cancel_rect.max.x + 1.0, cancel_rect.center().y);
    assert!(rect.contains(pos));
    click_at(&mut harness, &mut app, pos);
    assert_eq!(app.selected, Some(row));
    click_at(&mut harness, &mut app, cancel_rect.center());
    assert_eq!(
        app.selected,
        Some(cancel),
        "the child is above its container"
    );
    click_at(&mut harness, &mut app, rustroke::point(700.0, 500.0));
    assert_eq!(app.selected, None, "the empty canvas deselects");
}

fn click_at(harness: &mut Harness, app: &mut Canvas, pos: rustroke::Point) {
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
    harness.run(app);
}

#[test]
fn preview_mode_uses_the_widgets() {
    let mut app = Canvas::new(login(), Mode::Preview);
    let mut harness = Harness::with_size(800.0, 600.0);
    harness.run(&mut app);
    assert!(harness.click(&mut app, "Remember me"));
    assert_eq!(
        app.state.bindings.get("remember"),
        Some(&BindingValue::Bool(true))
    );
    assert_eq!(app.selected, None);
}

#[test]
fn renders_every_sample_view() {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"));
    for view in sample_project().views {
        let name = view.name.clone();
        let mut app = Canvas::new(view, Mode::Design);
        let mut harness = Harness::with_size(900.0, 900.0);
        for _ in 0..3 {
            harness.run(&mut app);
        }
        app.selected = app.view.nodes.first().map(|n| n.id);
        harness.run(&mut app);
        let shot = harness.render_software();
        shot.save_png(dir.join(format!("canvas-{name}.png")))
            .unwrap();
    }
}
