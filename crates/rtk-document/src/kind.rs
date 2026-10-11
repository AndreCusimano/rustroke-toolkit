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
    /// `ui.vertical(|ui| …)`
    Vertical,
    /// `Container::new().show(ui, |ui| …)`: background, border, margins,
    /// size limits
    Container,
    /// `Panel::left(id).show_inside(ui, |ui| …)`: a bar along one side of
    /// the space left
    Panel,
    /// `CentralPanel::default().show_inside(ui, |ui| …)`: all the space
    /// left after the panels
    CentralPanel,
    /// `ui.horizontal(|ui| …)` / `horizontal_wrapped`
    Horizontal,
    /// `ui.vertical_centered(|ui| …)`
    VerticalCentered,
    /// `ui.indent(|ui| …)`
    Indent,
    /// `Grid::new(id).show(ui, |ui| …)`
    Grid,
    /// `ui.end_row()`, only inside a [`WidgetKind::Grid`]
    EndRow,
    /// `CollapsingHeader::new(text).show(ui, |ui| …)`
    Collapsing,
    /// `ScrollArea::vertical().show(ui, |ui| …)`
    ScrollArea,
    /// `Flex::row(id).show(ui, |flex| …)`; children are [`WidgetKind::FlexItem`]s
    Flex,
    /// `flex.add(FlexItem::new(), |ui| …)`, only inside a [`WidgetKind::Flex`]
    FlexItem,
    /// `FlexGrid::new(id, columns).show(ui, |grid| …)`; children are
    /// [`WidgetKind::GridCell`]s
    FlexGrid,
    /// `grid.add(GridCell::at(column, row), |ui| …)`, only inside a
    /// [`WidgetKind::FlexGrid`]
    GridCell,
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
    /// Free code.
    Code,
}

impl Category {
    /// Every category, in menu order.
    pub const ALL: [Self; 6] = [
        Self::Text,
        Self::Buttons,
        Self::Values,
        Self::Input,
        Self::Containers,
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

const STYLED: &[PropSpec] = container!();
const CONTAINER: &[PropSpec] = container!(
    FILL,
    STROKE,
    CORNER_RADIUS,
    g(STYLE, "shadow", "Shadow", PropKind::Shadow),
    g(LAYOUT, "inner_margin", "Inner margin", PropKind::Size),
    g(LAYOUT, "outer_margin", "Outer margin", PropKind::Size),
    MIN_SIZE,
    MAX_SIZE,
    flag(LAYOUT, "fill_width", "Fill width"),
    flag(LAYOUT, "fill_height", "Fill height"),
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
const HORIZONTAL: &[PropSpec] = container!(p("wrapped", "Wrapped", BOOL, Init::Bool(false)));
const GRID: &[PropSpec] = container!(
    p("id", "Id", PropKind::IdSalt, Init::Text("grid")),
    p("striped", "Striped", BOOL, Init::Bool(false)),
    g(LAYOUT, "spacing", "Cell spacing", PropKind::Size),
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
/// Directions of a flex container.
pub const FLEX_DIRECTIONS: &[&str] = &["row", "column"];
/// `FlexJustify` variants.
pub const FLEX_JUSTIFY: &[&str] = &[
    "Start",
    "End",
    "Center",
    "SpaceBetween",
    "SpaceAround",
    "SpaceEvenly",
];
/// `FlexAlign` variants.
pub const FLEX_ALIGN: &[&str] = &["Start", "End", "Center", "Stretch"];
const FLEX: &[PropSpec] = &[
    p("id", "Id", PropKind::IdSalt, Init::Text("flex")),
    p(
        "direction",
        "Direction",
        PropKind::Choice(FLEX_DIRECTIONS),
        Init::Text("row"),
    ),
    p("wrap", "Wrap", BOOL, Init::Bool(false)),
    p("gap", "Gap", SIZE, Init::None),
    p(
        "justify",
        "Justify",
        PropKind::Choice(FLEX_JUSTIFY),
        Init::None,
    ),
    p("align", "Align", PropKind::Choice(FLEX_ALIGN), Init::None),
    flag(LAYOUT, "fill_width", "Fill width"),
    flag(LAYOUT, "fill_height", "Fill height"),
];
const FLEX_ITEM: &[PropSpec] = container!(
    p("grow", "Grow", SIZE, Init::None),
    p("shrink", "Shrink", SIZE, Init::None),
    p("basis", "Basis", ANY_FLOAT, Init::None),
    p("align", "Align", PropKind::Choice(FLEX_ALIGN), Init::None),
    MIN_SIZE,
    MAX_SIZE,
);
const FLEX_GRID: &[PropSpec] = &[
    p("id", "Id", PropKind::IdSalt, Init::Text("flex_grid")),
    p(
        "columns",
        "Columns",
        PropKind::Tracks,
        Init::Text("120, 1fr"),
    ),
    p("gap", "Gap", SIZE, Init::None),
];
const CELL_INDEX: PropKind = PropKind::Int { min: 0, max: 999 };
const CELL_SPAN: PropKind = PropKind::Int { min: 1, max: 999 };
const GRID_CELL: &[PropSpec] = container!(
    p("column", "Column", CELL_INDEX, Init::Int(0)),
    p("row", "Row", CELL_INDEX, Init::Int(0)),
    p("column_span", "Columns spanned", CELL_SPAN, Init::Int(1)),
    p("row_span", "Rows spanned", CELL_SPAN, Init::Int(1)),
);
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
        Self::Vertical,
        Self::Horizontal,
        Self::VerticalCentered,
        Self::Indent,
        Self::Container,
        Self::Panel,
        Self::CentralPanel,
        Self::Grid,
        Self::EndRow,
        Self::Collapsing,
        Self::ScrollArea,
        Self::Flex,
        Self::FlexItem,
        Self::FlexGrid,
        Self::GridCell,
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
            Self::Vertical => spec!(
                "Vertical",
                Containers,
                true,
                STYLED,
                "Stacks its widgets top to bottom."
            ),
            Self::Horizontal => spec!(
                "Horizontal",
                Containers,
                true,
                HORIZONTAL,
                "Puts its widgets side by side."
            ),
            Self::VerticalCentered => spec!(
                "Vertical centered",
                Containers,
                true,
                STYLED,
                "Stacks its widgets, centered."
            ),
            Self::Indent => spec!(
                "Indent",
                Containers,
                true,
                STYLED,
                "Shifts its widgets to the right."
            ),
            Self::Container => spec!(
                "Container",
                Containers,
                true,
                CONTAINER,
                "Background, border, margins and size limits."
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
                "Aligns widgets in columns and rows."
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
            Self::Flex => spec!(
                "Flex",
                Containers,
                true,
                FLEX,
                "Flexbox: rows or columns that wrap and grow."
            ),
            Self::FlexItem => spec!(
                "Flex item",
                Containers,
                true,
                FLEX_ITEM,
                "One item of a flex, with its own grow."
            ),
            Self::FlexGrid => spec!(
                "Flex grid",
                Containers,
                true,
                FLEX_GRID,
                "Columns of fixed or shared widths, cells that span."
            ),
            Self::GridCell => spec!(
                "Grid cell",
                Containers,
                true,
                GRID_CELL,
                "One cell of a flex grid: column, row, span."
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
            (Some(Self::Flex), child) => child == Self::FlexItem,
            (Some(Self::FlexGrid), child) => child == Self::GridCell,
            (Some(parent), _) if !parent.is_container() => false,
            (Some(Self::Grid), _) => !matches!(child, Self::FlexItem | Self::GridCell),
            (_, Self::EndRow | Self::FlexItem | Self::GridCell) => false,
            _ => true,
        }
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
        assert!(!K::accepts(None, K::FlexItem));
        assert!(K::accepts(Some(K::Grid), K::EndRow));
        assert!(!K::accepts(Some(K::Vertical), K::EndRow));
        assert!(K::accepts(Some(K::Flex), K::FlexItem));
        assert!(!K::accepts(Some(K::Flex), K::Label));
        assert!(K::accepts(Some(K::FlexItem), K::Label));
        assert!(!K::accepts(Some(K::Label), K::Label));
        assert!(K::accepts(Some(K::FlexGrid), K::GridCell));
        assert!(!K::accepts(Some(K::FlexGrid), K::Label));
        assert!(!K::accepts(None, K::GridCell));
        assert!(K::accepts(Some(K::GridCell), K::Label));
    }
}
