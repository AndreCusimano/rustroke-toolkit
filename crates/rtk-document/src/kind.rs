//! The kinds of nodes a view can contain and the properties of each.
//!
//! Everything else (palette, properties panel, code generator, parser)
//! is driven by these specs, so adding a widget starts here.

use crate::value::{FieldType, Init, PropKind, PropSpec};

/// What a node is: a rustroke widget, a layout container or free code.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WidgetKind {
    /// `ui.label(text)`
    Label,
    /// `ui.heading(text)`
    Heading,
    /// `ui.hyperlink_to(text, url)`
    Hyperlink,
    /// `ui.separator()` / `Separator::new()`
    Separator,
    /// `ui.add_space(amount)`
    Space,
    /// `ui.button(text)`, optionally `if … .clicked() { … }`
    Button,
    /// `ui.checkbox(&mut self.x, text)`
    Checkbox,
    /// `ui.radio_value(&mut self.x, value, text)`
    RadioValue,
    /// `ui.selectable_value(&mut self.x, value, text)`
    SelectableValue,
    /// `Slider::new(&mut self.x, range)`
    Slider,
    /// `DragValue::new(&mut self.x)`
    DragValue,
    /// `ProgressBar::new(self.x)`
    ProgressBar,
    /// `ui.spinner()`
    Spinner,
    /// `TextEdit::singleline(&mut self.x)` / `multiline`
    TextEdit,
    /// `SearchField::new(&mut self.x)`
    SearchField,
    /// `TimePicker::new(&mut self.x.0, &mut self.x.1)`
    TimePicker,
    /// `ColorPicker::new(&mut self.x)`
    ColorPicker,
    /// `DatePicker::new(&mut self.x)`
    DatePicker,
    /// `ComboBox::from_label(label)` with one `selectable_value` per item
    ComboBox,
    /// `Stack::column().show(ui, |ui| …)`: a column, a row or an overlay
    /// in a box (background, border, padding, size); without options,
    /// the short forms `ui.vertical(|ui| …)` / `ui.horizontal(|ui| …)`
    Stack,
    /// `ui.stack_item(StackItem::new(), |ui| …)`, only inside a
    /// [`WidgetKind::Stack`]: grow, size limits, alignment
    StackItem,
    /// `Panel::left(id).show_inside(ui, |ui| …)`: a bar along one side of
    /// the space left
    Panel,
    /// `CentralPanel::default().show_inside(ui, |ui| …)`: all the space
    /// left after the panels
    CentralPanel,
    /// `Grid::new(id).show(ui, |ui| …)`
    Grid,
    /// `ui.end_row()`, only inside a [`WidgetKind::Grid`]
    EndRow,
    /// `ui.grid_cell(GridCell::at(column, row), |ui| …)` or
    /// `ui.grid_span(columns, rows, |ui| …)`, only inside a
    /// [`WidgetKind::Grid`]
    GridCell,
    /// `CollapsingHeader::new(text).show(ui, |ui| …)`
    Collapsing,
    /// `ScrollArea::vertical().show(ui, |ui| …)`
    ScrollArea,
    /// `Plot::new(id).show(ui, |plot| …)`; children are series (and free
    /// code)
    Plot,
    /// `plot.line(Line::new(&self.x))`, only inside a [`WidgetKind::Plot`]
    PlotLine,
    /// `plot.points(Points::new(&self.x))`, only inside a [`WidgetKind::Plot`]
    PlotPoints,
    /// `plot.hline(HLine::new(y))`, only inside a [`WidgetKind::Plot`]
    PlotHLine,
    /// `plot.vline(VLine::new(x))`, only inside a [`WidgetKind::Plot`]
    PlotVLine,
    /// `PieChart::new(self.x.clone()).show(ui)`
    PieChart,
    /// Code the toolkit doesn't model, kept as written
    RawCode,
}

/// Groups of the widget menus.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    /// Labels, links, separators.
    Text,
    /// Buttons and choices.
    Buttons,
    /// Numbers and progress.
    Values,
    /// Text fields.
    Input,
    /// Layout containers.
    Containers,
    /// Plots and pie charts.
    Charts,
    /// Free code.
    Code,
}

impl Category {
    /// Every category, in menu order.
    pub const ALL: [Self; 7] = [
        Self::Text,
        Self::Buttons,
        Self::Values,
        Self::Input,
        Self::Containers,
        Self::Charts,
        Self::Code,
    ];

    /// Menu title.
    pub fn name(self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Buttons => "Buttons",
            Self::Values => "Values",
            Self::Input => "Input",
            Self::Containers => "Containers",
            Self::Charts => "Charts",
            Self::Code => "Code",
        }
    }
}

/// Static description of a [`WidgetKind`].
#[derive(Debug)]
pub struct KindSpec {
    /// Name shown in menus and in the component tree.
    pub name: &'static str,
    /// Menu the kind appears in.
    pub category: Category,
    /// Whether the node has children.
    pub container: bool,
    /// Properties, in the order shown in the properties panel.
    pub props: &'static [PropSpec],
    /// One-line explanation shown in the widget palette.
    pub description: &'static str,
}

const fn p(key: &'static str, label: &'static str, kind: PropKind, init: Init) -> PropSpec {
    PropSpec {
        key,
        label,
        kind,
        group: "",
        init,
    }
}

/// An optional property shown in the section `group` of the properties
/// panel.
const fn g(
    group: &'static str,
    key: &'static str,
    label: &'static str,
    kind: PropKind,
) -> PropSpec {
    PropSpec {
        key,
        label,
        kind,
        group,
        init: Init::None,
    }
}

const TEXT: PropKind = PropKind::Text;
const BOOL: PropKind = PropKind::Bool;
const ANY_FLOAT: PropKind = PropKind::Float {
    min: f64::NEG_INFINITY,
    max: f64::INFINITY,
};
const SIZE: PropKind = PropKind::Float {
    min: 0.0,
    max: f64::INFINITY,
};

/// Section of the properties panel for colors, borders and fonts.
pub const STYLE: &str = "Style";
/// Section of the properties panel for sizes.
pub const LAYOUT: &str = "Size";

const FONT: PropSpec = g(STYLE, "font", "Font", PropKind::Font);
const COLOR: PropSpec = g(STYLE, "color", "Color", PropKind::Color);
const FILL: PropSpec = g(STYLE, "fill", "Background", PropKind::Color);
const STROKE: PropSpec = g(STYLE, "stroke", "Border", PropKind::Stroke);
const CORNER_RADIUS: PropSpec = g(STYLE, "corner_radius", "Corner radius", SIZE);
const MIN_SIZE: PropSpec = g(LAYOUT, "min_size", "Min size", PropKind::Size);
const TEXT_COLOR: PropSpec = g(STYLE, "text_color", "Text color", PropKind::Color);
/// Color of the filled part of a slider or progress bar.
const BAR_COLOR: PropSpec = g(STYLE, "bar_color", "Bar color", PropKind::Color);
const MAX_SIZE: PropSpec = g(LAYOUT, "max_size", "Max size", PropKind::Limit);

/// A flag of the section `group`, false in a new node.
const fn flag(group: &'static str, key: &'static str, label: &'static str) -> PropSpec {
    PropSpec {
        group,
        ..p(key, label, BOOL, Init::Bool(false))
    }
}

/// `ui.add_sized(vec2(w, h), widget)`: the widget stretched to this size.
const FIXED_SIZE: PropSpec = g(LAYOUT, "fixed_size", "Fixed size", PropKind::Size);

/// Style of the widgets inside a container (or the whole view), set at the
/// start of its content with `ui.style_mut()`.
pub const CONTENT_STYLE: &[PropSpec] = &[
    g(STYLE, "text_color", "Text color", PropKind::Color),
    g(STYLE, "accent", "Accent", PropKind::Color),
    g(STYLE, "widget_fill", "Widget fill", PropKind::Color),
    g(STYLE, "field_fill", "Field fill", PropKind::Color),
    g(STYLE, "item_spacing", "Spacing", PropKind::Size),
];

/// The `ui.style_mut()` field each [`CONTENT_STYLE`] property sets.
pub fn content_style_target(key: &str) -> Option<&'static str> {
    Some(match key {
        "text_color" => "visuals.text",
        "accent" => "visuals.accent",
        "widget_fill" => "visuals.inactive.bg_fill",
        "field_fill" => "visuals.text_field_fill",
        "item_spacing" => "spacing.item_spacing",
        _ => return None,
    })
}

const LABEL: &[PropSpec] = &[
    p("text", "Text", TEXT, Init::Text("Label")),
    p("no_wrap", "No wrap", BOOL, Init::Bool(false)),
    FONT,
    COLOR,
    FIXED_SIZE,
];
const HEADING: &[PropSpec] = &[
    p("text", "Text", TEXT, Init::Text("Heading")),
    COLOR,
    FIXED_SIZE,
];
const HYPERLINK: &[PropSpec] = &[
    p("text", "Text", TEXT, Init::Text("Link")),
    p("url", "URL", TEXT, Init::Text("https://")),
    TEXT_COLOR,
    FONT,
    FIXED_SIZE,
];
/// Orientations of a separator (`auto` follows the layout).
pub const SEPARATOR_ORIENTATIONS: &[&str] = &["auto", "horizontal", "vertical"];
const SEPARATOR: &[PropSpec] = &[
    p(
        "orientation",
        "Orientation",
        PropKind::Choice(SEPARATOR_ORIENTATIONS),
        Init::Text("auto"),
    ),
    STROKE,
    g(LAYOUT, "spacing", "Spacing", SIZE),
];
const SPACE: &[PropSpec] = &[p("amount", "Amount", SIZE, Init::Float(8.0))];
const BUTTON: &[PropSpec] = &[
    p("text", "Text", TEXT, Init::Text("Button")),
    p("on_click", "On click", PropKind::Code, Init::Text("")),
    p("selected", "Selected", BOOL, Init::Bool(false)),
    p("shortcut_text", "Shortcut text", TEXT, Init::None),
    PropSpec {
        group: STYLE,
        ..p("frame", "Frame", BOOL, Init::Bool(true))
    },
    TEXT_COLOR,
    FONT,
    FILL,
    STROKE,
    CORNER_RADIUS,
    MIN_SIZE,
    FIXED_SIZE,
];
const CHECKBOX: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Bool),
        Init::Text("checked"),
    ),
    p("text", "Text", TEXT, Init::Text("Checkbox")),
    TEXT_COLOR,
    FONT,
    FIXED_SIZE,
];
const CHOICE: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Usize),
        Init::Text("choice"),
    ),
    p(
        "value",
        "Value",
        PropKind::Int {
            min: 0,
            max: i64::MAX,
        },
        Init::Int(0),
    ),
    p("text", "Text", TEXT, Init::Text("Option")),
    TEXT_COLOR,
    FONT,
];
const SLIDER: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::F32),
        Init::Text("value"),
    ),
    p("range", "Range", PropKind::Range, Init::Range(0.0, 100.0)),
    p("text", "Text", TEXT, Init::None),
    p("step", "Step", SIZE, Init::None),
    p("show_value", "Show value", BOOL, Init::Bool(true)),
    BAR_COLOR,
    FIXED_SIZE,
];
const DRAG_VALUE: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::F32),
        Init::Text("value"),
    ),
    p("speed", "Speed", SIZE, Init::None),
    p("range", "Range", PropKind::Range, Init::None),
    p("prefix", "Prefix", TEXT, Init::None),
    p("suffix", "Suffix", TEXT, Init::None),
    p(
        "decimals",
        "Decimals",
        PropKind::Int { min: 0, max: 15 },
        Init::None,
    ),
    FIXED_SIZE,
];
const PROGRESS_BAR: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::F32),
        Init::Text("progress"),
    ),
    p("text", "Text", TEXT, Init::None),
    p(
        "show_percentage",
        "Show percentage",
        BOOL,
        Init::Bool(false),
    ),
    p("animate", "Animate", BOOL, Init::Bool(false)),
    BAR_COLOR,
    g(LAYOUT, "desired_width", "Width", SIZE),
    FIXED_SIZE,
];
const SPINNER: &[PropSpec] = &[p("size", "Size", SIZE, Init::None), COLOR];
const TEXT_EDIT: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::String),
        Init::Text("text"),
    ),
    p("multiline", "Multiline", BOOL, Init::Bool(false)),
    p("hint_text", "Hint", TEXT, Init::None),
    p("password", "Password", BOOL, Init::Bool(false)),
    p(
        "code_editor",
        "Code editor (Tab indents)",
        BOOL,
        Init::Bool(false),
    ),
    p("wrap", "Wrap lines", BOOL, Init::Bool(true)),
    p("line_numbers", "Line numbers", BOOL, Init::Bool(false)),
    p(
        "select_all_on_focus",
        "Select all on focus",
        BOOL,
        Init::Bool(false),
    ),
    FONT,
    FILL,
    STROKE,
    CORNER_RADIUS,
    g(STYLE, "margin", "Inner margin", PropKind::Size),
    g(LAYOUT, "desired_width", "Width", SIZE),
    PropSpec {
        group: LAYOUT,
        ..p(
            "desired_rows",
            "Rows",
            PropKind::Int { min: 1, max: 1000 },
            Init::None,
        )
    },
    MIN_SIZE,
    FIXED_SIZE,
];
const SEARCH_FIELD: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::String),
        Init::Text("search"),
    ),
    p("hint_text", "Hint", TEXT, Init::None),
    FILL,
    STROKE,
    CORNER_RADIUS,
    g(LAYOUT, "desired_width", "Width", SIZE),
    MIN_SIZE,
    FIXED_SIZE,
];
const TIME_PICKER: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Time),
        Init::Text("time"),
    ),
    FIXED_SIZE,
];
const COLOR_PICKER: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Color),
        Init::Text("color"),
    ),
    p("alpha", "Alpha", BOOL, Init::Bool(false)),
    FIXED_SIZE,
];
const DATE_PICKER: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Date),
        Init::Text("date"),
    ),
    p(
        "sunday_first",
        "Week starts on Sunday",
        BOOL,
        Init::Bool(false),
    ),
    FIXED_SIZE,
];
const COMBO_BOX: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Usize),
        Init::Text("selected"),
    ),
    p("label", "Label", TEXT, Init::Text("Choice")),
    p(
        "items",
        "Items",
        PropKind::Items,
        Init::Items(&["First", "Second", "Third"]),
    ),
    FILL,
    STROKE,
    CORNER_RADIUS,
    g(LAYOUT, "width", "Width", SIZE),
    MIN_SIZE,
];

/// Container properties followed by [`CONTENT_STYLE`].
macro_rules! container {
    ($($prop:expr),* $(,)?) => {
        &[
            $($prop,)*
            CONTENT_STYLE[0],
            CONTENT_STYLE[1],
            CONTENT_STYLE[2],
            CONTENT_STYLE[3],
            CONTENT_STYLE[4],
        ]
    };
}

/// Directions of a stack.
pub const STACK_DIRECTIONS: &[&str] = &["column", "row", "overlay"];
/// `Justify` variants.
pub const JUSTIFY: &[&str] = &[
    "Start",
    "Center",
    "End",
    "SpaceBetween",
    "SpaceAround",
    "SpaceEvenly",
];
/// `StackAlign` variants.
pub const STACK_ALIGN: &[&str] = &["Start", "Center", "End", "Stretch"];

/// Box properties of stacks and grids, followed by [`CONTENT_STYLE`].
macro_rules! boxed {
    ($($prop:expr),* $(,)?) => {
        container!(
            $($prop,)*
            FILL,
            STROKE,
            CORNER_RADIUS,
            g(STYLE, "shadow", "Shadow", PropKind::Shadow),
            g(LAYOUT, "padding", "Padding", PropKind::Size),
            g(LAYOUT, "margin", "Margin", PropKind::Size),
            g(LAYOUT, "width", "Width", PropKind::Sizing),
            g(LAYOUT, "height", "Height", PropKind::Sizing),
            MIN_SIZE,
            MAX_SIZE,
            g(LAYOUT, "clip", "Clip", BOOL),
        )
    };
}

const STACK: &[PropSpec] = boxed!(
    p(
        "direction",
        "Direction",
        PropKind::Choice(STACK_DIRECTIONS),
        Init::Text("column"),
    ),
    p("gap", "Gap", SIZE, Init::None),
    p("wrap", "Wrap", BOOL, Init::Bool(false)),
    p("justify", "Justify", PropKind::Choice(JUSTIFY), Init::None),
    p("align", "Align", PropKind::Choice(STACK_ALIGN), Init::None),
);
const STACK_ITEM: &[PropSpec] = container!(
    p("grow", "Grow", SIZE, Init::None),
    p("shrink", "Shrink", SIZE, Init::None),
    p("basis", "Basis", SIZE, Init::None),
    p("align", "Align", PropKind::Choice(STACK_ALIGN), Init::None),
    MIN_SIZE,
    MAX_SIZE,
);
/// Sides a panel can be on.
pub const PANEL_SIDES: &[&str] = &["left", "right", "top", "bottom"];
const PANEL: &[PropSpec] = container!(
    p("id", "Id", PropKind::IdSalt, Init::Text("panel")),
    p(
        "side",
        "Side",
        PropKind::Choice(PANEL_SIDES),
        Init::Text("left")
    ),
    p("resizable", "Resizable (sides)", BOOL, Init::Bool(true)),
    g(LAYOUT, "default_size", "Size", SIZE),
    g(LAYOUT, "min", "Min size", SIZE),
    g(LAYOUT, "max", "Max size", SIZE),
    g(LAYOUT, "margin", "Margin", SIZE),
);
const CENTRAL_PANEL: &[PropSpec] = container!(g(LAYOUT, "margin", "Margin", SIZE));
const GRID: &[PropSpec] = boxed!(
    p("id", "Id", PropKind::IdSalt, Init::Text("grid")),
    p("columns", "Columns", PropKind::Tracks, Init::None),
    p("striped", "Striped", BOOL, Init::Bool(false)),
    g(LAYOUT, "spacing", "Cell spacing", PropKind::Size),
    p(
        "align_x",
        "Cell align x",
        PropKind::Choice(STACK_ALIGN),
        Init::None
    ),
    p(
        "align_y",
        "Cell align y",
        PropKind::Choice(STACK_ALIGN),
        Init::None
    ),
);
const CELL_INDEX: PropKind = PropKind::Int { min: 0, max: 999 };
const CELL_SPAN: PropKind = PropKind::Int { min: 1, max: 999 };
const GRID_CELL: &[PropSpec] = container!(
    p("column", "Column", CELL_INDEX, Init::None),
    p("row", "Row", CELL_INDEX, Init::None),
    p("column_span", "Columns spanned", CELL_SPAN, Init::Int(1)),
    p("row_span", "Rows spanned", CELL_SPAN, Init::Int(1)),
);
const COLLAPSING: &[PropSpec] = container!(
    p("text", "Text", TEXT, Init::Text("Section")),
    p("default_open", "Open by default", BOOL, Init::Bool(false)),
    g(STYLE, "header_color", "Header color", PropKind::Color),
    g(STYLE, "header_font", "Header font", PropKind::Font),
);
/// Directions of a scroll area.
pub const SCROLL_DIRECTIONS: &[&str] = &["vertical", "horizontal", "both"];
const SCROLL_AREA: &[PropSpec] = container!(
    p(
        "direction",
        "Direction",
        PropKind::Choice(SCROLL_DIRECTIONS),
        Init::Text("vertical"),
    ),
    p("auto_shrink", "Auto shrink", BOOL, Init::Bool(true)),
    g(LAYOUT, "max_height", "Max height", SIZE),
    g(LAYOUT, "max_width", "Max width", SIZE),
);
/// `Corner` variants (plot legend).
pub const CORNERS: &[&str] = &["TopLeft", "TopRight", "BottomLeft", "BottomRight"];
/// `MarkerShape` variants.
pub const MARKER_SHAPES: &[&str] = &["Circle", "Square", "Diamond", "Triangle", "Cross", "Plus"];
/// Line styles of chart lines (`LineStyle::dashed()`…).
pub const LINE_STYLES: &[&str] = &["solid", "dashed", "dotted"];
const PLOT: &[PropSpec] = &[
    p("id", "Id", PropKind::IdSalt, Init::Text("plot")),
    p("x_label", "X axis title", TEXT, Init::None),
    p("y_label", "Y axis title", TEXT, Init::None),
    p("legend", "Legend", PropKind::Choice(CORNERS), Init::None),
    p("crosshair", "Crosshair", BOOL, Init::Bool(true)),
    p("tooltip", "Tooltip", BOOL, Init::Bool(true)),
    p("allow_drag", "Drag to pan", BOOL, Init::Bool(true)),
    p("allow_zoom", "Zoom", BOOL, Init::Bool(true)),
    g(STYLE, "background", "Background", PropKind::Color),
    g(LAYOUT, "width", "Width", SIZE),
    g(LAYOUT, "height", "Height", SIZE),
];
const SERIES_NAME: PropSpec = p("name", "Name", TEXT, Init::None);
const LINE_STYLE: PropSpec = PropSpec {
    group: STYLE,
    ..p(
        "style",
        "Line style",
        PropKind::Choice(LINE_STYLES),
        Init::Text("solid"),
    )
};
const LINE_WIDTH: PropSpec = g(STYLE, "width", "Width", SIZE);
const PLOT_LINE: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Points),
        Init::Text("series"),
    ),
    SERIES_NAME,
    COLOR,
    LINE_WIDTH,
    LINE_STYLE,
    g(STYLE, "fill", "Fill down to", ANY_FLOAT),
    g(STYLE, "markers", "Markers", PropKind::Choice(MARKER_SHAPES)),
    flag(STYLE, "mark_min_max", "Mark min and max"),
];
const PLOT_POINTS: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Points),
        Init::Text("points"),
    ),
    SERIES_NAME,
    COLOR,
    g(STYLE, "radius", "Radius", SIZE),
    g(STYLE, "shape", "Shape", PropKind::Choice(MARKER_SHAPES)),
    PropSpec {
        group: STYLE,
        ..p("filled", "Filled", BOOL, Init::Bool(true))
    },
];
const PLOT_REFERENCE: &[PropSpec] = &[
    p("value", "Value", ANY_FLOAT, Init::Float(0.0)),
    SERIES_NAME,
    COLOR,
    LINE_WIDTH,
    LINE_STYLE,
];
const PIE_CHART: &[PropSpec] = &[
    p(
        "binding",
        "Binding",
        PropKind::Binding(FieldType::Slices),
        Init::Text("slices"),
    ),
    p(
        "donut",
        "Donut hole",
        PropKind::Float { min: 0.0, max: 0.9 },
        Init::None,
    ),
    p("percentages", "Percentages", BOOL, Init::Bool(true)),
    p("legend", "Legend", BOOL, Init::Bool(true)),
    p("start_angle", "Start angle", ANY_FLOAT, Init::None),
    g(LAYOUT, "diameter", "Diameter", SIZE),
];
const RAW_CODE: &[PropSpec] = &[p(
    "code",
    "Code",
    PropKind::Code,
    Init::Text("// your code"),
)];

impl WidgetKind {
    /// Every kind, in menu order.
    pub const ALL: [Self; 35] = [
        Self::Label,
        Self::Heading,
        Self::Hyperlink,
        Self::Separator,
        Self::Space,
        Self::Button,
        Self::Checkbox,
        Self::RadioValue,
        Self::SelectableValue,
        Self::ComboBox,
        Self::Slider,
        Self::DragValue,
        Self::ProgressBar,
        Self::Spinner,
        Self::TextEdit,
        Self::SearchField,
        Self::TimePicker,
        Self::ColorPicker,
        Self::DatePicker,
        Self::Stack,
        Self::StackItem,
        Self::Panel,
        Self::CentralPanel,
        Self::Grid,
        Self::EndRow,
        Self::GridCell,
        Self::Collapsing,
        Self::ScrollArea,
        Self::Plot,
        Self::PlotLine,
        Self::PlotPoints,
        Self::PlotHLine,
        Self::PlotVLine,
        Self::PieChart,
        Self::RawCode,
    ];

    /// The kind's static description.
    pub fn spec(self) -> &'static KindSpec {
        macro_rules! spec {
            ($name:literal, $cat:ident, $container:literal, $props:expr, $description:literal) => {{
                const SPEC: KindSpec = KindSpec {
                    name: $name,
                    category: Category::$cat,
                    container: $container,
                    props: $props,
                    description: $description,
                };
                &SPEC
            }};
        }
        match self {
            Self::Label => spec!(
                "Label",
                Text,
                false,
                LABEL,
                "Text, wrapped to the available width."
            ),
            Self::Heading => spec!(
                "Heading",
                Text,
                false,
                HEADING,
                "Large bold text, for titles."
            ),
            Self::Hyperlink => spec!(
                "Hyperlink",
                Text,
                false,
                HYPERLINK,
                "A link that opens a web address."
            ),
            Self::Separator => spec!(
                "Separator",
                Text,
                false,
                SEPARATOR,
                "A thin line between widgets."
            ),
            Self::Space => spec!("Space", Text, false, SPACE, "Empty space of a fixed size."),
            Self::Button => spec!(
                "Button",
                Buttons,
                false,
                BUTTON,
                "Runs your code when clicked."
            ),
            Self::Checkbox => spec!(
                "Checkbox",
                Buttons,
                false,
                CHECKBOX,
                "Turns a bool field on and off."
            ),
            Self::RadioValue => spec!(
                "Radio button",
                Buttons,
                false,
                CHOICE,
                "One choice of a group sharing a field."
            ),
            Self::SelectableValue => spec!(
                "Selectable label",
                Buttons,
                false,
                CHOICE,
                "Highlighted text that selects a value."
            ),
            Self::ComboBox => spec!(
                "Combo box",
                Buttons,
                false,
                COMBO_BOX,
                "Drop-down list to choose one item."
            ),
            Self::Slider => spec!(
                "Slider",
                Values,
                false,
                SLIDER,
                "Drag to set a number in a range."
            ),
            Self::DragValue => spec!(
                "Drag value",
                Values,
                false,
                DRAG_VALUE,
                "A number you can drag or type."
            ),
            Self::ProgressBar => spec!(
                "Progress bar",
                Values,
                false,
                PROGRESS_BAR,
                "Shows progress from 0 to 1."
            ),
            Self::Spinner => spec!(
                "Spinner",
                Values,
                false,
                SPINNER,
                "Shows that something is loading."
            ),
            Self::TextEdit => spec!(
                "Text field",
                Input,
                false,
                TEXT_EDIT,
                "Single- or multi-line text input."
            ),
            Self::SearchField => spec!(
                "Search field",
                Input,
                false,
                SEARCH_FIELD,
                "Text input with a search icon."
            ),
            Self::TimePicker => spec!(
                "Time picker",
                Input,
                false,
                TIME_PICKER,
                "Hour and minute, with a picker."
            ),
            Self::ColorPicker => spec!(
                "Color picker",
                Input,
                false,
                COLOR_PICKER,
                "A color swatch that opens a picker."
            ),
            Self::DatePicker => spec!(
                "Date picker",
                Input,
                false,
                DATE_PICKER,
                "A date, with a calendar."
            ),
            Self::Stack => spec!(
                "Stack",
                Containers,
                true,
                STACK,
                "Widgets in a column, a row or one over the other."
            ),
            Self::StackItem => spec!(
                "Stack item",
                Containers,
                true,
                STACK_ITEM,
                "One item of a stack: grow, size limits, alignment."
            ),
            Self::Panel => spec!(
                "Panel",
                Containers,
                true,
                PANEL,
                "A bar along one side, full height or width."
            ),
            Self::CentralPanel => spec!(
                "Central panel",
                Containers,
                true,
                CENTRAL_PANEL,
                "All the space left after the panels."
            ),
            Self::Grid => spec!(
                "Grid",
                Containers,
                true,
                GRID,
                "Columns and rows, in a box with a size."
            ),
            Self::EndRow => spec!(
                "End of row",
                Containers,
                false,
                &[],
                "Starts a new row of a grid."
            ),
            Self::Collapsing => spec!(
                "Collapsing header",
                Containers,
                true,
                COLLAPSING,
                "A section that opens and closes."
            ),
            Self::ScrollArea => spec!(
                "Scroll area",
                Containers,
                true,
                SCROLL_AREA,
                "Scrolls content larger than its area."
            ),
            Self::GridCell => spec!(
                "Grid cell",
                Containers,
                true,
                GRID_CELL,
                "A grid cell spanning columns and rows, or placed."
            ),
            Self::Plot => spec!(
                "Plot",
                Charts,
                true,
                PLOT,
                "A chart with axes; holds lines and points."
            ),
            Self::PlotLine => spec!(
                "Line series",
                Charts,
                false,
                PLOT_LINE,
                "A line through points, optionally filled."
            ),
            Self::PlotPoints => spec!(
                "Point series",
                Charts,
                false,
                PLOT_POINTS,
                "Points with markers (a scatter plot)."
            ),
            Self::PlotHLine => spec!(
                "Horizontal line",
                Charts,
                false,
                PLOT_REFERENCE,
                "A line across the plot at a y value."
            ),
            Self::PlotVLine => spec!(
                "Vertical line",
                Charts,
                false,
                PLOT_REFERENCE,
                "A line across the plot at an x value."
            ),
            Self::PieChart => spec!(
                "Pie chart",
                Charts,
                false,
                PIE_CHART,
                "Slices of a whole, as a pie or a donut."
            ),
            Self::RawCode => spec!(
                "Code",
                Code,
                false,
                RAW_CODE,
                "Your own Rust code, kept as written."
            ),
        }
    }

    /// Display name.
    pub fn name(self) -> &'static str {
        self.spec().name
    }

    /// Whether nodes of this kind have children.
    pub fn is_container(self) -> bool {
        self.spec().container
    }

    /// The spec of property `key`, if this kind has it.
    pub fn prop(self, key: &str) -> Option<&'static PropSpec> {
        self.spec().props.iter().find(|spec| spec.key == key)
    }

    /// Whether a node of kind `child` may be placed directly inside a node
    /// of this kind (`None` = the view's top level).
    pub fn accepts(parent: Option<Self>, child: Self) -> bool {
        match (parent, child) {
            (Some(Self::Plot), child) => child.is_series() || child == Self::RawCode,
            (_, child) if child.is_series() => false,
            (Some(parent), _) if !parent.is_container() => false,
            (Some(Self::Grid), Self::EndRow | Self::GridCell) => true,
            (Some(Self::Stack), Self::StackItem) => true,
            (_, Self::EndRow | Self::GridCell | Self::StackItem) => false,
            _ => true,
        }
    }

    /// Whether this is a series of a [`WidgetKind::Plot`].
    pub fn is_series(self) -> bool {
        matches!(
            self,
            Self::PlotLine | Self::PlotPoints | Self::PlotHLine | Self::PlotVLine
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs_are_consistent() {
        for kind in WidgetKind::ALL {
            let spec = kind.spec();
            let mut keys: Vec<_> = spec.props.iter().map(|p| p.key).collect();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(keys.len(), spec.props.len(), "{kind:?} has duplicate keys");
            assert!(
                spec.props
                    .iter()
                    .filter(|p| matches!(p.kind, PropKind::Binding(_)))
                    .count()
                    <= 1,
                "{kind:?} has more than one binding"
            );
            for prop in spec.props {
                if let Some(value) = prop.init.to_value() {
                    assert!(prop.kind.accepts(&value), "{kind:?}.{} init", prop.key);
                }
            }
        }
        assert_eq!(
            WidgetKind::ALL.len(),
            WidgetKind::ALL
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
        );
    }

    #[test]
    fn containment_rules() {
        use WidgetKind as K;
        assert!(K::accepts(None, K::Label));
        assert!(!K::accepts(None, K::EndRow));
        assert!(!K::accepts(None, K::StackItem));
        assert!(K::accepts(Some(K::Grid), K::EndRow));
        assert!(!K::accepts(Some(K::Stack), K::EndRow));
        assert!(K::accepts(Some(K::Stack), K::StackItem));
        assert!(K::accepts(Some(K::Stack), K::Label));
        assert!(!K::accepts(Some(K::Grid), K::StackItem));
        assert!(K::accepts(Some(K::StackItem), K::Label));
        assert!(!K::accepts(Some(K::Label), K::Label));
        assert!(K::accepts(Some(K::Grid), K::GridCell));
        assert!(K::accepts(Some(K::Grid), K::Label));
        assert!(!K::accepts(None, K::GridCell));
        assert!(K::accepts(Some(K::GridCell), K::Label));
        assert!(K::accepts(Some(K::Plot), K::PlotLine));
        assert!(!K::accepts(Some(K::Plot), K::Label));
        assert!(K::accepts(Some(K::Plot), K::RawCode));
        assert!(!K::accepts(None, K::PlotPoints));
        assert!(!K::accepts(Some(K::Stack), K::PlotHLine));
        assert!(K::accepts(None, K::PieChart));
    }
}
