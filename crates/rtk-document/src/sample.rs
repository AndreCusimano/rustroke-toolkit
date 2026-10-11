//! An example project that uses every kind of node. It is the project the
//! toolkit opens on first start, and its generated code is compiled by the
//! `rtk-codegen-check` crate to prove the generator emits valid rustroke code.

use crate::kind::WidgetKind as K;
use crate::model::{Node, NodeId, Project, RawField, View, WindowSettings};
use crate::value::{Font, FontFamily, Shadow, Value};

fn text(s: &str) -> Value {
    Value::Text(s.to_owned())
}

/// Adds a node of `kind` under `parent` with the given properties.
fn add(
    view: &mut View,
    parent: Option<NodeId>,
    kind: K,
    props: &[(&'static str, Value)],
) -> NodeId {
    let mut node: Node = view.create(kind);
    for (key, value) in props {
        node.props.insert(key, value.clone());
    }
    view.insert(parent, usize::MAX, node)
        .expect("sample nodes are valid")
}

fn login() -> View {
    let mut v = View::new("login");
    v.window = WindowSettings {
        title: "Sign in".to_owned(),
        width: 360.0,
        height: 300.0,
        resizable: false,
        min_size: None,
        max_size: None,
    };
    v.style.insert("accent", Value::Color([46, 160, 103, 255]));
    v.style.insert("item_spacing", Value::Size(8.0, 10.0));
    add(
        &mut v,
        None,
        K::Heading,
        &[
            ("text", text("Sign in")),
            ("color", Value::Color([46, 160, 103, 255])),
        ],
    );
    add(
        &mut v,
        None,
        K::TextEdit,
        &[
            ("binding", text("username")),
            ("hint_text", text("Username")),
            ("corner_radius", Value::Float(6.0)),
            ("stroke", Value::Stroke(1.0, [46, 160, 103, 128])),
            ("desired_width", Value::Float(240.0)),
        ],
    );
    add(
        &mut v,
        None,
        K::TextEdit,
        &[
            ("binding", text("password")),
            ("hint_text", text("Password")),
            ("password", Value::Bool(true)),
        ],
    );
    add(
        &mut v,
        None,
        K::Checkbox,
        &[("binding", text("remember")), ("text", text("Remember me"))],
    );
    add(&mut v, None, K::Space, &[("amount", Value::Float(8.0))]);
    let row = add(&mut v, None, K::Horizontal, &[]);
    add(&mut v, Some(row), K::Button, &[("text", text("Cancel"))]);
    add(
        &mut v,
        Some(row),
        K::Button,
        &[
            ("text", text("Sign in")),
            (
                "on_click",
                text("// Free code is kept as written.\nself.attempts += 1;"),
            ),
            ("fill", Value::Color([46, 160, 103, 255])),
            ("corner_radius", Value::Float(6.0)),
            ("min_size", Value::Size(96.0, 32.0)),
        ],
    );
    add(&mut v, None, K::Separator, &[]);
    add(
        &mut v,
        None,
        K::Hyperlink,
        &[
            ("text", text("Forgot your password?")),
            ("url", text("https://example.com/reset")),
        ],
    );
    add(
        &mut v,
        None,
        K::Hyperlink,
        &[("text", text("")), ("url", text("https://example.com"))],
    );
    v.extra_fields.push(RawField {
        name: "attempts".to_owned(),
        code: "/// Failed sign-in attempts.\npub attempts: u32".to_owned(),
    });
    v
}

fn settings() -> View {
    let mut v = View::new("settings");
    v.window = WindowSettings {
        title: "Settings".to_owned(),
        width: 560.0,
        height: 760.0,
        resizable: true,
        min_size: Some((400.0, 480.0)),
        max_size: Some((1200.0, 1600.0)),
    };
    add(&mut v, None, K::Heading, &[("text", text("Settings"))]);
    add(
        &mut v,
        None,
        K::SearchField,
        &[
            ("binding", text("filter")),
            ("hint_text", text("Search settings")),
            ("desired_width", Value::Float(240.0)),
        ],
    );

    let audio = add(
        &mut v,
        None,
        K::Collapsing,
        &[("text", text("Audio")), ("default_open", Value::Bool(true))],
    );
    add(
        &mut v,
        Some(audio),
        K::Slider,
        &[
            ("binding", text("volume")),
            ("range", Value::Range(0.0, 100.0)),
            ("text", text("Volume")),
            ("step", Value::Float(5.0)),
        ],
    );
    add(
        &mut v,
        Some(audio),
        K::Slider,
        &[
            ("binding", text("balance")),
            ("range", Value::Range(-1.0, 1.0)),
            ("show_value", Value::Bool(false)),
        ],
    );
    add(
        &mut v,
        Some(audio),
        K::DragValue,
        &[
            ("binding", text("latency")),
            ("speed", Value::Float(0.5)),
            ("range", Value::Range(0.0, 500.0)),
            ("prefix", text("Latency: ")),
            ("suffix", text(" ms")),
            ("decimals", Value::Int(1)),
        ],
    );
    add(
        &mut v,
        Some(audio),
        K::DragValue,
        &[("binding", text("gain"))],
    );

    let display = add(&mut v, None, K::Collapsing, &[("text", text("Display"))]);
    add(
        &mut v,
        Some(display),
        K::ComboBox,
        &[
            ("binding", text("quality")),
            ("label", text("Quality")),
            (
                "items",
                Value::Items(vec!["Low".into(), "Medium".into(), "High".into()]),
            ),
            ("width", Value::Float(160.0)),
        ],
    );
    add(
        &mut v,
        Some(display),
        K::ComboBox,
        &[
            ("binding", text("empty_choice")),
            ("label", text("Nothing")),
            ("items", Value::Items(Vec::new())),
        ],
    );
    let modes = add(&mut v, Some(display), K::Horizontal, &[]);
    for (i, label) in ["Window", "Fullscreen", "Borderless"].iter().enumerate() {
        add(
            &mut v,
            Some(modes),
            K::RadioValue,
            &[
                ("binding", text("mode")),
                ("value", Value::Int(i as i64)),
                ("text", text(label)),
            ],
        );
    }
    let tabs = add(
        &mut v,
        Some(display),
        K::Horizontal,
        &[("wrapped", Value::Bool(true))],
    );
    for (i, label) in ["General", "Advanced"].iter().enumerate() {
        add(
            &mut v,
            Some(tabs),
            K::SelectableValue,
            &[
                ("binding", text("tab")),
                ("value", Value::Int(i as i64)),
                ("text", text(label)),
            ],
        );
    }

    let grid = add(
        &mut v,
        None,
        K::Grid,
        &[("id", text("summary")), ("striped", Value::Bool(true))],
    );
    add(&mut v, Some(grid), K::Label, &[("text", text("Version"))]);
    add(
        &mut v,
        Some(grid),
        K::Label,
        &[("text", text("0.1.0")), ("no_wrap", Value::Bool(true))],
    );
    add(&mut v, Some(grid), K::EndRow, &[]);
    add(&mut v, Some(grid), K::Label, &[("text", text("Status"))]);
    add(&mut v, Some(grid), K::Spinner, &[]);
    add(&mut v, Some(grid), K::EndRow, &[]);

    add(
        &mut v,
        None,
        K::ProgressBar,
        &[
            ("binding", text("progress")),
            ("text", text("Downloading")),
            ("show_percentage", Value::Bool(true)),
            ("desired_width", Value::Float(200.0)),
        ],
    );
    add(
        &mut v,
        None,
        K::ProgressBar,
        &[("binding", text("progress"))],
    );
    add(&mut v, None, K::Spinner, &[("size", Value::Float(24.0))]);

    let scroll = add(
        &mut v,
        None,
        K::ScrollArea,
        &[
            ("direction", text("both")),
            ("max_height", Value::Float(120.0)),
            ("max_width", Value::Float(400.0)),
            ("auto_shrink", Value::Bool(false)),
        ],
    );
    add(
        &mut v,
        Some(scroll),
        K::TextEdit,
        &[
            ("binding", text("notes")),
            ("multiline", Value::Bool(true)),
            ("desired_rows", Value::Int(4)),
            ("desired_width", Value::Float(360.0)),
        ],
    );
    add(
        &mut v,
        Some(scroll),
        K::TextEdit,
        &[("binding", text("log")), ("multiline", Value::Bool(true))],
    );
    add(&mut v, None, K::ScrollArea, &[]);

    let indent = add(&mut v, None, K::Indent, &[]);
    add(
        &mut v,
        Some(indent),
        K::Label,
        &[("text", text("Indented"))],
    );
    let centered = add(&mut v, None, K::VerticalCentered, &[]);
    add(
        &mut v,
        Some(centered),
        K::Label,
        &[("text", text("Centered"))],
    );
    add(&mut v, None, K::Vertical, &[]);

    let flex = add(
        &mut v,
        None,
        K::Flex,
        &[
            ("id", text("toolbar")),
            ("wrap", Value::Bool(true)),
            ("gap", Value::Float(8.0)),
            ("justify", text("SpaceBetween")),
            ("align", text("Center")),
        ],
    );
    add(&mut v, Some(flex), K::Button, &[("text", text("Apply"))]);
    let item = add(
        &mut v,
        Some(flex),
        K::FlexItem,
        &[
            ("grow", Value::Float(1.0)),
            ("shrink", Value::Float(0.0)),
            ("basis", Value::Float(120.0)),
            ("align", text("Stretch")),
        ],
    );
    add(
        &mut v,
        Some(item),
        K::TextEdit,
        &[("binding", text("name"))],
    );
    add(
        &mut v,
        None,
        K::Flex,
        &[("id", text("column")), ("direction", text("column"))],
    );

    appearance(&mut v);

    add(
        &mut v,
        None,
        K::RawCode,
        &[(
            "code",
            text(
                "// Free code is kept as written.\nif self.show_hint {\n    ui.add(Label::new(\"Press Enter to apply\").color(Color::from_srgb8(137, 180, 250)));\n}",
            ),
        )],
    );
    v.extra_fields.push(RawField {
        name: "show_hint".to_owned(),
        code: "pub show_hint: bool".to_owned(),
    });
    v.extra_methods.push(
        "/// Restores the defaults.\npub fn reset(&mut self) {\n    *self = Self::default();\n}"
            .to_owned(),
    );
    v.extra_items.push(
        "/// Volume of a new installation.\npub const DEFAULT_VOLUME: f32 = 50.0;".to_owned(),
    );
    v
}

/// Colors, borders, fonts and sizes.
fn appearance(v: &mut View) {
    let section = add(
        v,
        None,
        K::Collapsing,
        &[
            ("text", text("Appearance")),
            ("text_color", Value::Color([230, 200, 120, 255])),
            ("item_spacing", Value::Size(6.0, 6.0)),
        ],
    );
    let box_ = add(
        v,
        Some(section),
        K::Vertical,
        &[
            ("widget_fill", Value::Color([40, 44, 52, 255])),
            ("field_fill", Value::Color([20, 22, 26, 255])),
            ("accent", Value::Color([250, 120, 80, 255])),
        ],
    );
    add(
        v,
        Some(box_),
        K::Label,
        &[
            ("text", text("Styled text")),
            (
                "font",
                Value::Font(Font {
                    family: FontFamily::Proportional,
                    size: 18.0,
                    weight: 600,
                    bold: false,
                }),
            ),
            ("color", Value::Color([137, 180, 250, 255])),
        ],
    );
    add(
        v,
        Some(box_),
        K::Label,
        &[
            ("text", text("fn main() {}")),
            (
                "font",
                Value::Font(Font {
                    family: FontFamily::Monospace,
                    size: 13.0,
                    weight: Font::REGULAR,
                    bold: true,
                }),
            ),
            ("fixed_size", Value::Size(200.0, 24.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Heading,
        &[
            ("text", text("Sized heading")),
            ("fixed_size", Value::Size(300.0, 40.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Hyperlink,
        &[
            ("text", text("Docs")),
            ("url", text("https://docs.rs/rustroke")),
            ("fixed_size", Value::Size(80.0, 24.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Hyperlink,
        &[
            ("text", text("")),
            ("url", text("https://crates.io")),
            ("fixed_size", Value::Size(120.0, 24.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Button,
        &[
            ("text", text("Danger")),
            ("selected", Value::Bool(true)),
            ("shortcut_text", text("⌘D")),
            ("fill", Value::Color([200, 60, 60, 230])),
            ("stroke", Value::Stroke(2.0, [255, 255, 255, 255])),
            ("corner_radius", Value::Float(12.0)),
            ("fixed_size", Value::Size(160.0, 36.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Button,
        &[("text", text("Flat")), ("frame", Value::Bool(false))],
    );
    add(
        v,
        Some(box_),
        K::Checkbox,
        &[
            ("binding", text("dark")),
            ("text", text("Dark")),
            ("fixed_size", Value::Size(120.0, 28.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Slider,
        &[
            ("binding", text("volume")),
            ("range", Value::Range(0.0, 100.0)),
            ("fixed_size", Value::Size(300.0, 28.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::DragValue,
        &[
            ("binding", text("gain")),
            ("fixed_size", Value::Size(80.0, 28.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::ProgressBar,
        &[
            ("binding", text("progress")),
            ("animate", Value::Bool(true)),
            ("fixed_size", Value::Size(200.0, 20.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::Spinner,
        &[("color", Value::Color([250, 120, 80, 255]))],
    );
    add(
        v,
        Some(box_),
        K::TextEdit,
        &[
            ("binding", text("script")),
            ("multiline", Value::Bool(true)),
            ("code_editor", Value::Bool(true)),
            ("wrap", Value::Bool(false)),
            ("line_numbers", Value::Bool(true)),
            ("select_all_on_focus", Value::Bool(true)),
            (
                "font",
                Value::Font(Font {
                    family: FontFamily::Monospace,
                    size: 13.0,
                    weight: Font::REGULAR,
                    bold: false,
                }),
            ),
            ("fill", Value::Color([16, 16, 16, 255])),
            ("margin", Value::Size(8.0, 6.0)),
            ("min_size", Value::Size(300.0, 80.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::TextEdit,
        &[
            ("binding", text("code")),
            ("fixed_size", Value::Size(200.0, 28.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::SearchField,
        &[
            ("binding", text("filter")),
            ("fill", Value::Color([30, 30, 30, 255])),
            ("stroke", Value::Stroke(1.0, [90, 90, 90, 255])),
            ("corner_radius", Value::Float(14.0)),
            ("min_size", Value::Size(200.0, 28.0)),
            ("fixed_size", Value::Size(220.0, 28.0)),
        ],
    );
    add(
        v,
        Some(box_),
        K::ComboBox,
        &[
            ("binding", text("quality")),
            ("label", text("Styled")),
            ("items", Value::Items(vec!["A".into(), "B".into()])),
            ("fill", Value::Color([40, 40, 60, 255])),
            ("stroke", Value::Stroke(1.0, [120, 120, 200, 255])),
            ("corner_radius", Value::Float(4.0)),
            ("min_size", Value::Size(120.0, 28.0)),
        ],
    );
    add(v, Some(box_), K::TimePicker, &[("binding", text("alarm"))]);
    add(
        v,
        Some(box_),
        K::TimePicker,
        &[
            ("binding", text("wake_up")),
            ("fixed_size", Value::Size(120.0, 28.0)),
        ],
    );

    let grid = add(
        v,
        Some(section),
        K::Grid,
        &[
            ("id", text("spaced")),
            ("spacing", Value::Size(24.0, 4.0)),
            ("text_color", Value::Color([200, 200, 200, 255])),
        ],
    );
    add(v, Some(grid), K::Label, &[("text", text("Spaced"))]);
    add(v, Some(grid), K::EndRow, &[]);

    let layout = add(
        v,
        Some(section),
        K::FlexGrid,
        &[
            ("id", text("layout")),
            ("columns", text("120, 1fr, auto")),
            ("gap", Value::Float(6.0)),
        ],
    );
    add(v, Some(layout), K::Label, &[("text", text("Name"))]);
    add(v, Some(layout), K::TextEdit, &[("binding", text("name"))]);
    let wide = add(
        v,
        Some(layout),
        K::GridCell,
        &[
            ("column", Value::Int(0)),
            ("row", Value::Int(1)),
            ("column_span", Value::Int(3)),
            ("row_span", Value::Int(1)),
            ("accent", Value::Color([90, 200, 250, 255])),
        ],
    );
    add(v, Some(wide), K::Button, &[("text", text("Across"))]);
    add(v, None, K::FlexGrid, &[("id", text("empty_grid"))]);
    let flex = add(v, Some(section), K::Flex, &[("id", text("styled"))]);
    add(
        v,
        Some(flex),
        K::FlexItem,
        &[("text_color", Value::Color([255, 0, 0, 255]))],
    );
    add(
        v,
        Some(section),
        K::ScrollArea,
        &[("item_spacing", Value::Size(2.0, 2.0))],
    );
    add(
        v,
        Some(section),
        K::Horizontal,
        &[("accent", Value::Color([255, 200, 0, 255]))],
    );
    add(
        v,
        Some(section),
        K::Indent,
        &[("text_color", Value::Color([255, 255, 255, 128]))],
    );
    add(
        v,
        Some(section),
        K::VerticalCentered,
        &[("widget_fill", Value::Color([0, 0, 0, 255]))],
    );
}

/// Zones: a sidebar as tall as the window with a width between limits,
/// a top bar, and the rest with cards.
fn dashboard() -> View {
    let mut v = View::new("dashboard");
    v.window = WindowSettings {
        title: "Dashboard".to_owned(),
        width: 900.0,
        height: 600.0,
        resizable: true,
        min_size: Some((600.0, 400.0)),
        max_size: None,
    };
    let sidebar = add(
        &mut v,
        None,
        K::Panel,
        &[
            ("id", text("sidebar")),
            ("default_size", Value::Float(200.0)),
            ("min", Value::Float(160.0)),
            ("max", Value::Float(320.0)),
            ("text_color", Value::Color([220, 225, 235, 255])),
        ],
    );
    add(&mut v, Some(sidebar), K::Heading, &[("text", text("Menu"))]);
    for (i, item) in ["Overview", "Reports", "Settings"].into_iter().enumerate() {
        let mut props = vec![
            ("binding", text("page")),
            ("value", Value::Int(i as i64)),
            ("text", text(item)),
        ];
        if i == 0 {
            props.push(("text_color", Value::Color([46, 160, 103, 255])));
            props.push((
                "font",
                Value::Font(Font {
                    bold: true,
                    ..Font::default()
                }),
            ));
        }
        add(&mut v, Some(sidebar), K::SelectableValue, &props);
    }
    add(
        &mut v,
        Some(sidebar),
        K::Separator,
        &[
            ("stroke", Value::Stroke(2.0, [46, 160, 103, 255])),
            ("spacing", Value::Float(12.0)),
        ],
    );
    add(
        &mut v,
        Some(sidebar),
        K::RadioValue,
        &[
            ("binding", text("density")),
            ("value", Value::Int(0)),
            ("text", text("Compact")),
            ("text_color", Value::Color([200, 120, 40, 255])),
        ],
    );
    add(
        &mut v,
        Some(sidebar),
        K::RadioValue,
        &[
            ("binding", text("density")),
            ("value", Value::Int(1)),
            ("text", text("Comfortable")),
        ],
    );
    let top = add(
        &mut v,
        None,
        K::Panel,
        &[
            ("id", text("toolbar")),
            ("side", text("top")),
            ("resizable", Value::Bool(false)),
            ("margin", Value::Float(6.0)),
        ],
    );
    let row = add(&mut v, Some(top), K::Horizontal, &[]);
    add(
        &mut v,
        Some(row),
        K::DatePicker,
        &[
            ("binding", text("day")),
            ("sunday_first", Value::Bool(true)),
        ],
    );
    add(
        &mut v,
        Some(row),
        K::Separator,
        &[("orientation", text("vertical"))],
    );
    add(
        &mut v,
        Some(row),
        K::ColorPicker,
        &[("binding", text("highlight")), ("alpha", Value::Bool(true))],
    );
    add(
        &mut v,
        Some(row),
        K::Hyperlink,
        &[
            ("text", text("Help")),
            ("url", text("https://example.com/help")),
            ("text_color", Value::Color([200, 120, 40, 255])),
        ],
    );
    let central = add(
        &mut v,
        None,
        K::CentralPanel,
        &[("margin", Value::Float(12.0))],
    );
    let flex = add(
        &mut v,
        Some(central),
        K::Flex,
        &[
            ("id", text("cards")),
            ("gap", Value::Float(12.0)),
            ("align", text("Stretch")),
            ("fill_height", Value::Bool(true)),
        ],
    );
    let card = |v: &mut View, title: &str, grow: f64, max: f64| {
        let item = add(
            v,
            Some(flex),
            K::FlexItem,
            &[
                ("grow", Value::Float(grow)),
                ("min_size", Value::Size(160.0, 0.0)),
                ("max_size", Value::Size(max, f64::INFINITY)),
            ],
        );
        let container = add(
            v,
            Some(item),
            K::Container,
            &[
                ("fill", Value::Color([40, 44, 52, 255])),
                ("stroke", Value::Stroke(1.0, [70, 76, 88, 255])),
                ("corner_radius", Value::Float(8.0)),
                (
                    "shadow",
                    Value::Shadow(Shadow {
                        offset: (0.0, 3.0),
                        blur: 10.0,
                        spread: 0.0,
                        color: [0, 0, 0, 90],
                    }),
                ),
                ("inner_margin", Value::Size(12.0, 10.0)),
                ("fill_width", Value::Bool(true)),
                ("fill_height", Value::Bool(true)),
                ("text_color", Value::Color([235, 238, 245, 255])),
            ],
        );
        add(v, Some(container), K::Heading, &[("text", text(title))]);
        container
    };
    let sales = card(&mut v, "Sales", 2.0, 600.0);
    add(
        &mut v,
        Some(sales),
        K::ProgressBar,
        &[
            ("binding", text("progress")),
            ("show_percentage", Value::Bool(true)),
            ("bar_color", Value::Color([46, 160, 103, 255])),
        ],
    );
    add(
        &mut v,
        Some(sales),
        K::Slider,
        &[
            ("binding", text("progress")),
            ("range", Value::Range(0.0, 1.0)),
            ("bar_color", Value::Color([200, 120, 40, 255])),
        ],
    );
    let notes = card(&mut v, "Notes", 1.0, 360.0);
    let section = add(
        &mut v,
        Some(notes),
        K::Collapsing,
        &[
            ("text", text("Details")),
            ("default_open", Value::Bool(true)),
            ("header_color", Value::Color([46, 160, 103, 255])),
        ],
    );
    add(
        &mut v,
        Some(section),
        K::Checkbox,
        &[
            ("binding", text("pinned")),
            ("text", text("Pinned")),
            ("text_color", Value::Color([200, 120, 40, 255])),
        ],
    );
    let limited = add(
        &mut v,
        Some(notes),
        K::Container,
        &[
            ("stroke", Value::Stroke(1.0, [120, 120, 120, 255])),
            ("outer_margin", Value::Size(0.0, 6.0)),
            ("min_size", Value::Size(0.0, 60.0)),
            ("max_size", Value::Size(f64::INFINITY, 120.0)),
        ],
    );
    add(
        &mut v,
        Some(limited),
        K::Label,
        &[("text", text("At least 60 and at most 120 points tall."))],
    );
    add(
        &mut v,
        Some(notes),
        K::Button,
        &[
            ("text", text("Archive")),
            ("text_color", Value::Color([255, 255, 255, 255])),
            ("fill", Value::Color([180, 60, 60, 255])),
        ],
    );
    v
}

/// The example project.
pub fn sample_project() -> Project {
    Project {
        name: "example".to_owned(),
        views: vec![login(), settings(), dashboard(), View::new("empty")],
    }
}
