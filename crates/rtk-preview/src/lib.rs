//! Draws a Rustroke Toolkit view with rustroke: the design canvas and the
//! live preview.
//!
//! [`show_view`] walks the view's nodes and calls the same rustroke
//! functions the generated code calls, with the values of the bindings kept
//! in [`Bindings`].
//!
//! In [`Mode::Design`] every node gets a transparent layer over it that
//! takes the clicks, so clicking selects instead of using the widget, and
//! the selected and hovered nodes are outlined. In [`Mode::Preview`] the
//! widgets work for real.

use std::collections::HashMap;

use std::collections::BTreeMap;

use rtk_document::{
    CONTENT_STYLE, FieldType, Font, FontFamily, Node, NodeId, Track, Value, View, WidgetKind,
    parse_tracks,
};
use rustroke::{
    Button, CentralPanel, Checkbox, CollapsingHeader, Color, ColorPicker, ComboBox, Container,
    Date, DatePicker, DragValue, Flex, FlexAlign, FlexGrid, FlexItem, FlexJustify, Grid, GridCell,
    Hyperlink, Id, Label, Panel, ProgressBar, RadioButton, Rect, Response, ScrollArea, SearchField,
    SelectableLabel, Sense, Separator, Shadow, Slider, Spinner, Stroke, TextEdit, TextStyle,
    TimePicker, Ui, Vec2, Widget, vec2,
};

/// How the canvas behaves.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Mode {
    /// Clicks select nodes; widgets don't react.
    #[default]
    Design,
    /// The widgets work, with the values in [`Bindings`].
    Preview,
}

/// The value of one view field.
#[derive(Clone, Debug, PartialEq)]
pub enum BindingValue {
    /// `bool`
    Bool(bool),
    /// `String`
    String(String),
    /// `f32`
    F32(f32),
    /// `usize`
    Usize(usize),
    /// `(u32, u32)`
    Time((u32, u32)),
    /// `Color`
    Color(Color),
    /// `Date`
    Date(Date),
}

/// The values of the view's fields, as the preview edits them. A field
/// missing (or of another type) starts at its type's default, like the
/// generated `#[derive(Default)]` struct.
#[derive(Clone, Debug, Default)]
pub struct Bindings {
    values: HashMap<String, BindingValue>,
}

macro_rules! binding_accessor {
    ($name:ident, $variant:ident, $ty:ty) => {
        /// The field's value, created with the default if missing.
        pub fn $name(&mut self, name: &str) -> &mut $ty {
            let entry = self
                .values
                .entry(name.to_owned())
                .or_insert_with(|| BindingValue::$variant(Default::default()));
            if !matches!(entry, BindingValue::$variant(_)) {
                *entry = BindingValue::$variant(Default::default());
            }
            match entry {
                BindingValue::$variant(value) => value,
                _ => unreachable!(),
            }
        }
    };
}

impl Bindings {
    binding_accessor!(bool_mut, Bool, bool);
    binding_accessor!(string_mut, String, String);
    binding_accessor!(f32_mut, F32, f32);
    binding_accessor!(usize_mut, Usize, usize);
    binding_accessor!(time_mut, Time, (u32, u32));
    binding_accessor!(color_mut, Color, Color);
    binding_accessor!(date_mut, Date, Date);

    /// The value of a field, if it was set.
    pub fn get(&self, name: &str) -> Option<&BindingValue> {
        self.values.get(name)
    }

    /// Forgets every value (back to the defaults).
    pub fn clear(&mut self) {
        self.values.clear();
    }

    /// Sets a field to its type's default value.
    pub fn reset(&mut self, name: &str, ty: FieldType) {
        let value = match ty {
            FieldType::Bool => BindingValue::Bool(false),
            FieldType::String => BindingValue::String(String::new()),
            FieldType::F32 => BindingValue::F32(0.0),
            FieldType::Usize => BindingValue::Usize(0),
            FieldType::Time => BindingValue::Time((0, 0)),
            FieldType::Color => BindingValue::Color(Color::default()),
            FieldType::Date => BindingValue::Date(Date::default()),
        };
        self.values.insert(name.to_owned(), value);
    }
}

/// What the canvas keeps between frames.
#[derive(Debug, Default)]
pub struct CanvasState {
    /// Values of the view's fields.
    pub bindings: Bindings,
    /// The area of every node drawn last frame.
    rects: HashMap<NodeId, Rect>,
}

impl CanvasState {
    /// The area a node covered last frame (for drop indicators, scrolling
    /// to the selection).
    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.rects.get(&id).copied()
    }
}

/// What happened on the canvas this frame (design mode).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CanvasOutput {
    /// The node that was clicked (to select it).
    pub clicked: Option<NodeId>,
    /// The node that was double-clicked (to edit its text, show its code).
    pub double_clicked: Option<NodeId>,
    /// The node that was right-clicked (context menu), with its response.
    pub secondary_clicked: Option<NodeId>,
    /// The node under the pointer.
    pub hovered: Option<NodeId>,
    /// Whether the empty part of the canvas was clicked (to deselect).
    pub background_clicked: bool,
    /// The node being pressed or dragged, with the response of its layer
    /// (to start dragging it with `Ui::dnd_set_payload`).
    pub pressed: Option<(NodeId, Response)>,
}

struct Painter<'s> {
    mode: Mode,
    bindings: &'s mut Bindings,
    prev_rects: &'s HashMap<NodeId, Rect>,
    rects: HashMap<NodeId, Rect>,
    out: CanvasOutput,
}

/// Whether `code` is only comments (drawn as nothing).
fn only_comments(code: &str) -> bool {
    code.lines().all(|l| {
        let l = l.trim_start();
        l.is_empty() || l.starts_with("//")
    })
}

fn f32_prop(node: &Node, key: &str) -> Option<f32> {
    node.prop(key).and_then(Value::as_f64).map(|v| v as f32)
}

fn usize_prop(node: &Node, key: &str) -> Option<usize> {
    node.prop(key)
        .and_then(Value::as_int)
        .map(|v| v.max(0) as usize)
}

/// A color value as rustroke's [`Color`].
pub fn color(c: [u8; 4]) -> Color {
    Color::from_srgba8(c[0], c[1], c[2], c[3])
}

/// A font value as rustroke's [`TextStyle`].
pub fn text_style(font: Font) -> TextStyle {
    let style = match font.family {
        FontFamily::Proportional => TextStyle::proportional(font.size as f32),
        FontFamily::Monospace => TextStyle::monospace(font.size as f32),
        FontFamily::System => TextStyle::system(font.size as f32),
    };
    let style = style.weight(font.weight);
    if font.bold { style.bold() } else { style }
}

fn color_prop(node: &Node, key: &str) -> Option<Color> {
    node.prop(key).and_then(Value::as_color).map(color)
}

fn size_prop(node: &Node, key: &str) -> Option<Vec2> {
    let (w, h) = node.prop(key).and_then(Value::as_size)?;
    Some(vec2(w as f32, h as f32))
}

fn stroke_prop(node: &Node, key: &str) -> Option<Stroke> {
    let (w, c) = node.prop(key).and_then(Value::as_stroke)?;
    Some(Stroke::new(w as f32, color(c)))
}

fn font_prop(node: &Node, key: &str) -> Option<TextStyle> {
    node.prop(key).and_then(Value::as_font).map(text_style)
}

fn shadow_prop(node: &Node, key: &str) -> Option<Shadow> {
    let s = node.prop(key).and_then(Value::as_shadow)?;
    Some(Shadow {
        offset: vec2(s.offset.0 as f32, s.offset.1 as f32),
        blur: s.blur as f32,
        spread: s.spread as f32,
        color: color(s.color),
    })
}

/// Applies the text properties (color and font, under `color_key` and
/// `font_key`) of `$node` to the builder `$widget`.
macro_rules! text {
    ($widget:expr, $node:expr) => {
        text!($widget, $node, "text_color", "font")
    };
    ($widget:expr, $node:expr, $color_key:expr, $font_key:expr) => {{
        let mut widget = $widget;
        if let Some(color) = color_prop($node, $color_key) {
            widget = widget.text_color(color);
        }
        if let Some(font) = font_prop($node, $font_key) {
            widget = widget.font(font);
        }
        widget
    }};
}

/// Applies the frame properties (background, border, corners, minimum
/// size) of `$node` to the builder `$widget`.
macro_rules! frame {
    ($widget:expr, $node:expr) => {{
        let mut widget = $widget;
        if let Some(fill) = color_prop($node, "fill") {
            widget = widget.fill(fill);
        }
        if let Some(stroke) = stroke_prop($node, "stroke") {
            widget = widget.stroke(stroke);
        }
        if let Some(radius) = f32_prop($node, "corner_radius") {
            widget = widget.corner_radius(radius);
        }
        if let Some(size) = size_prop($node, "min_size") {
            widget = widget.min_size(size);
        }
        widget
    }};
}

/// Adds `widget`, stretched to the node's fixed size if it has one.
fn add(ui: &mut Ui<'_>, node: &Node, widget: impl Widget) -> Response {
    match size_prop(node, "fixed_size") {
        Some(size) => ui.add_sized(size, widget),
        None => ui.add(widget),
    }
}

/// Sets the content style properties (text color, spacing…) on `ui`, as
/// the generated `ui.style_mut()` lines do.
pub fn apply_content_style(ui: &mut Ui<'_>, props: &BTreeMap<&'static str, Value>) {
    for spec in CONTENT_STYLE {
        let Some(value) = props.get(spec.key) else {
            continue;
        };
        let style = ui.style_mut();
        match (spec.key, value) {
            ("text_color", Value::Color(c)) => style.visuals.text = color(*c),
            ("accent", Value::Color(c)) => style.visuals.accent = color(*c),
            ("widget_fill", Value::Color(c)) => style.visuals.inactive.bg_fill = color(*c),
            ("field_fill", Value::Color(c)) => style.visuals.text_field_fill = color(*c),
            ("item_spacing", Value::Size(w, h)) => {
                style.spacing.item_spacing = vec2(*w as f32, *h as f32);
            }
            _ => {}
        }
    }
}

fn justify(name: &str) -> FlexJustify {
    match name {
        "End" => FlexJustify::End,
        "Center" => FlexJustify::Center,
        "SpaceBetween" => FlexJustify::SpaceBetween,
        "SpaceAround" => FlexJustify::SpaceAround,
        "SpaceEvenly" => FlexJustify::SpaceEvenly,
        _ => FlexJustify::Start,
    }
}

fn align(name: &str) -> FlexAlign {
    match name {
        "End" => FlexAlign::End,
        "Center" => FlexAlign::Center,
        "Stretch" => FlexAlign::Stretch,
        _ => FlexAlign::Start,
    }
}

impl Painter<'_> {
    fn design(&self) -> bool {
        self.mode == Mode::Design
    }

    /// The transparent layer that takes the clicks over a node.
    fn overlay(&mut self, ui: &mut Ui<'_>, id: NodeId, rect: Rect, salt: &str) {
        if !self.design() || (rect.width() <= 0.0 || rect.height() <= 0.0) {
            return;
        }
        let response = ui.interact(
            Id::new(("rtk-overlay", id.0, salt)),
            rect,
            Sense::POINTER_DRAG,
        );
        self.report(id, &response);
    }

    fn report(&mut self, id: NodeId, response: &Response) {
        if response.is_pressed() || response.dragged() || response.drag_started() {
            self.out.pressed = Some((id, response.clone()));
        }
        if response.hovered() {
            self.out.hovered = Some(id);
        }
        if response.clicked() {
            self.out.clicked = Some(id);
        }
        if response.double_clicked() {
            self.out.double_clicked = Some(id);
        }
        if response.secondary_clicked() {
            self.out.secondary_clicked = Some(id);
        }
    }

    /// A box shown in design mode in place of an empty container, so it can
    /// be seen and selected.
    fn placeholder(&mut self, ui: &mut Ui<'_>, text: &str) {
        if !self.design() {
            return;
        }
        let style = ui.style();
        let weak = style.visuals.weak_text;
        let galley = ui.layout_text(text, &TextStyle::proportional(12.0), None);
        let size = galley.size + vec2(16.0, 10.0);
        let rect = ui.allocate_rect(vec2(
            size.x.max(80.0),
            size.y.max(style.spacing.interact_height),
        ));
        let painter = ui.painter();
        painter.dashed_line(
            &[
                rect.left_top(),
                rect.right_top(),
                rect.right_bottom(),
                rect.left_bottom(),
                rect.left_top(),
            ],
            Stroke::new(1.0, weak.with_alpha(0.6)),
            4.0,
            3.0,
        );
        let pos = rect.center() - galley.size * 0.5;
        painter.galley(pos, galley, weak);
    }

    fn nodes(&mut self, ui: &mut Ui<'_>, nodes: &[Node]) {
        for node in nodes {
            ui.push_id(node.id.0, |ui| self.node(ui, node));
        }
    }

    /// Children of a container, or a placeholder if there are none, after
    /// the container's content style.
    fn children(&mut self, ui: &mut Ui<'_>, node: &Node) {
        apply_content_style(ui, &node.props);
        if node.children.is_empty() {
            self.placeholder(ui, &format!("Empty {}", node.kind.name().to_lowercase()));
        } else {
            self.nodes(ui, &node.children);
        }
    }

    /// Union of the node's own area and its children's.
    fn record(&mut self, node: &Node, rect: Rect) {
        let mut rect = rect;
        for child in &node.children {
            if let Some(child_rect) = self.rects.get(&child.id) {
                rect = rect.union(*child_rect);
            }
        }
        self.rects.insert(node.id, rect);
    }

    fn node(&mut self, ui: &mut Ui<'_>, node: &Node) {
        let id = node.id;
        if node.kind.is_container() {
            // Registered before the children, so they get the clicks first.
            if let Some(rect) = self.prev_rects.get(&id).copied() {
                self.overlay(ui, id, rect, "area");
            }
        }
        let text = node.text("text").to_owned();
        let binding = node
            .binding()
            .map(|(name, _)| name.to_owned())
            .unwrap_or_default();
        let rect = match node.kind {
            WidgetKind::Label | WidgetKind::Heading => {
                let mut label = Label::new(text);
                if node.kind == WidgetKind::Heading {
                    label = label.style(ui.style().heading.clone());
                }
                if node.flag("no_wrap") {
                    label = label.no_wrap();
                }
                if let Some(font) = font_prop(node, "font") {
                    label = label.style(font);
                }
                if let Some(color) = color_prop(node, "color") {
                    label = label.color(color);
                }
                add(ui, node, label).rect
            }
            WidgetKind::Hyperlink => {
                let url = node.text("url").to_owned();
                let link = if text.is_empty() {
                    Hyperlink::new(url)
                } else {
                    Hyperlink::from_label_and_url(text, url)
                };
                add(ui, node, text!(link, node)).rect
            }
            WidgetKind::Separator => {
                let mut separator = Separator::new();
                match node.text("orientation") {
                    "horizontal" => separator = separator.horizontal(),
                    "vertical" => separator = separator.vertical(),
                    _ => {}
                }
                if let Some(stroke) = stroke_prop(node, "stroke") {
                    separator = separator.stroke(stroke);
                }
                if let Some(spacing) = f32_prop(node, "spacing") {
                    separator = separator.spacing(spacing);
                }
                ui.add(separator).rect
            }
            WidgetKind::Space => {
                let amount = f32_prop(node, "amount").unwrap_or(0.0);
                let before = ui.min_rect();
                ui.add_space(amount);
                let after = ui.min_rect();
                // The gap itself, at least a few points thick to be selectable.
                let horizontal = ui.layout().is_horizontal();
                if horizontal {
                    Rect::from_min_max(
                        rustroke::point(before.max.x, after.min.y),
                        rustroke::point(after.max.x.max(before.max.x + 4.0), after.max.y),
                    )
                } else {
                    Rect::from_min_max(
                        rustroke::point(after.min.x, before.max.y),
                        rustroke::point(after.max.x, after.max.y.max(before.max.y + 4.0)),
                    )
                }
            }
            WidgetKind::Button => {
                let mut button = Button::new(text);
                if node.flag("selected") {
                    button = button.selected(true);
                }
                if let Some(shortcut) = node.prop("shortcut_text").and_then(Value::as_text) {
                    button = button.shortcut_text(shortcut);
                }
                if node.prop("frame") == Some(&Value::Bool(false)) {
                    button = button.frame(false);
                }
                add(ui, node, frame!(text!(button, node), node)).rect
            }
            WidgetKind::Checkbox => {
                let checkbox = Checkbox::new(self.bindings.bool_mut(&binding), text);
                add(ui, node, text!(checkbox, node)).rect
            }
            WidgetKind::RadioValue | WidgetKind::SelectableValue => {
                // What `radio_value` / `selectable_value` do, with the
                // text properties.
                let value = usize_prop(node, "value").unwrap_or(0);
                let current = self.bindings.usize_mut(&binding);
                let selected = *current == value;
                let response = if node.kind == WidgetKind::RadioValue {
                    ui.add(text!(RadioButton::new(selected, text), node))
                } else {
                    ui.add(text!(SelectableLabel::new(selected, text), node))
                };
                if response.clicked() {
                    *current = value;
                }
                response.rect
            }
            WidgetKind::Slider => {
                let (min, max) = node
                    .prop("range")
                    .and_then(Value::as_range)
                    .unwrap_or((0.0, 1.0));
                let mut slider =
                    Slider::new(self.bindings.f32_mut(&binding), min as f32..=max as f32);
                if let Some(text) = node.prop("text").and_then(Value::as_text) {
                    slider = slider.text(text);
                }
                if let Some(step) = node.prop("step").and_then(Value::as_f64) {
                    slider = slider.step(step);
                }
                if node.prop("show_value") == Some(&Value::Bool(false)) {
                    slider = slider.show_value(false);
                }
                if let Some(color) = color_prop(node, "bar_color") {
                    slider = slider.color(color);
                }
                add(ui, node, slider).rect
            }
            WidgetKind::DragValue => {
                let mut drag = DragValue::new(self.bindings.f32_mut(&binding));
                if let Some(speed) = node.prop("speed").and_then(Value::as_f64) {
                    drag = drag.speed(speed);
                }
                if let Some((min, max)) = node.prop("range").and_then(Value::as_range) {
                    drag = drag.range(min as f32..=max as f32);
                }
                if let Some(prefix) = node.prop("prefix").and_then(Value::as_text) {
                    drag = drag.prefix(prefix);
                }
                if let Some(suffix) = node.prop("suffix").and_then(Value::as_text) {
                    drag = drag.suffix(suffix);
                }
                if let Some(decimals) = usize_prop(node, "decimals") {
                    drag = drag.decimals(decimals);
                }
                add(ui, node, drag).rect
            }
            WidgetKind::ProgressBar => {
                let progress = *self.bindings.f32_mut(&binding);
                let mut bar = ProgressBar::new(progress);
                if let Some(text) = node.prop("text").and_then(Value::as_text) {
                    bar = bar.text(text);
                }
                if node.flag("show_percentage") {
                    bar = bar.show_percentage();
                }
                if node.flag("animate") {
                    bar = bar.animate(true);
                }
                if let Some(color) = color_prop(node, "bar_color") {
                    bar = bar.fill(color);
                }
                if let Some(width) = f32_prop(node, "desired_width") {
                    bar = bar.desired_width(width);
                }
                add(ui, node, bar).rect
            }
            WidgetKind::Spinner => {
                let mut spinner = Spinner::new();
                if let Some(size) = f32_prop(node, "size") {
                    spinner = spinner.size(size);
                }
                if let Some(color) = color_prop(node, "color") {
                    spinner = spinner.color(color);
                }
                ui.add(spinner).rect
            }
            WidgetKind::TimePicker => {
                let (hour, minute) = self.bindings.time_mut(&binding);
                add(ui, node, TimePicker::new(hour, minute)).rect
            }
            WidgetKind::ColorPicker => {
                let picker =
                    ColorPicker::new(self.bindings.color_mut(&binding)).alpha(node.flag("alpha"));
                add(ui, node, picker).rect
            }
            WidgetKind::DatePicker => {
                let picker = DatePicker::new(self.bindings.date_mut(&binding))
                    .sunday_first(node.flag("sunday_first"));
                add(ui, node, picker).rect
            }
            WidgetKind::TextEdit => {
                let value = self.bindings.string_mut(&binding);
                let mut edit = if node.flag("multiline") {
                    TextEdit::multiline(value)
                } else {
                    TextEdit::singleline(value)
                };
                if let Some(hint) = node.prop("hint_text").and_then(Value::as_text) {
                    edit = edit.hint_text(hint);
                }
                if node.flag("password") {
                    edit = edit.password(true);
                }
                if let Some(width) = f32_prop(node, "desired_width") {
                    edit = edit.desired_width(width);
                }
                if let Some(rows) = usize_prop(node, "desired_rows") {
                    edit = edit.desired_rows(rows);
                }
                if node.flag("code_editor") {
                    edit = edit.code_editor();
                }
                if node.prop("wrap") == Some(&Value::Bool(false)) {
                    edit = edit.wrap(false);
                }
                if node.flag("line_numbers") {
                    edit = edit.line_numbers(true);
                }
                if node.flag("select_all_on_focus") {
                    edit = edit.select_all_on_focus(true);
                }
                if let Some(font) = font_prop(node, "font") {
                    edit = edit.font(font);
                }
                if let Some(margin) = size_prop(node, "margin") {
                    edit = edit.margin(margin);
                }
                add(ui, node, frame!(edit, node)).rect
            }
            WidgetKind::SearchField => {
                let mut field = SearchField::new(self.bindings.string_mut(&binding));
                if let Some(hint) = node.prop("hint_text").and_then(Value::as_text) {
                    field = field.hint_text(hint);
                }
                if let Some(width) = f32_prop(node, "desired_width") {
                    field = field.desired_width(width);
                }
                add(ui, node, frame!(field, node)).rect
            }
            WidgetKind::ComboBox => {
                let items = node.prop("items").and_then(Value::as_items).unwrap_or(&[]);
                let selected = self.bindings.usize_mut(&binding);
                let mut combo = frame!(ComboBox::from_label(node.text("label")), node);
                if let Some(width) = f32_prop(node, "width") {
                    combo = combo.width(width);
                }
                let current = if items.is_empty() {
                    selected.to_string()
                } else {
                    items[(*selected).min(items.len() - 1)].clone()
                };
                combo
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        for (i, item) in items.iter().enumerate() {
                            ui.selectable_value(selected, i, item);
                        }
                    })
                    .response
                    .rect
            }
            WidgetKind::Vertical => ui.vertical(|ui| self.children(ui, node)).response.rect,
            WidgetKind::Container => {
                let mut container = frame!(Container::new(), node);
                if let Some(shadow) = shadow_prop(node, "shadow") {
                    container = container.shadow(shadow);
                }
                if let Some(margin) = size_prop(node, "inner_margin") {
                    container = container.inner_margin(margin);
                }
                if let Some(margin) = size_prop(node, "outer_margin") {
                    container = container.outer_margin(margin);
                }
                if let Some(size) = size_prop(node, "max_size") {
                    container = container.max_size(size);
                }
                container
                    .fill_width(node.flag("fill_width"))
                    .fill_height(node.flag("fill_height"))
                    .show(ui, |ui| self.children(ui, node))
                    .response
                    .rect
            }
            WidgetKind::Panel => {
                let id_salt = node.text("id");
                let mut panel = match node.text("side") {
                    "right" => Panel::right(id_salt),
                    "top" => Panel::top(id_salt),
                    "bottom" => Panel::bottom(id_salt),
                    _ => Panel::left(id_salt),
                };
                if node.prop("resizable") == Some(&Value::Bool(false)) {
                    panel = panel.resizable(false);
                }
                if let Some(size) = f32_prop(node, "default_size") {
                    panel = panel.default_size(size);
                }
                if let Some(size) = f32_prop(node, "min") {
                    panel = panel.min_size(size);
                }
                if let Some(size) = f32_prop(node, "max") {
                    panel = panel.max_size(size);
                }
                if let Some(margin) = f32_prop(node, "margin") {
                    panel = panel.margin(margin);
                }
                panel
                    .show_inside(ui, |ui| self.children(ui, node))
                    .response
                    .rect
            }
            WidgetKind::CentralPanel => {
                let mut panel = CentralPanel::default();
                if let Some(margin) = f32_prop(node, "margin") {
                    panel = panel.margin(margin);
                }
                let mut rect = ui.available_rect();
                panel.show_inside(ui, |ui| {
                    self.children(ui, node);
                    rect = rect.intersect(ui.clip_rect());
                });
                rect
            }
            WidgetKind::Horizontal => {
                if node.flag("wrapped") {
                    ui.horizontal_wrapped(|ui| self.children(ui, node))
                        .response
                        .rect
                } else {
                    ui.horizontal(|ui| self.children(ui, node)).response.rect
                }
            }
            WidgetKind::VerticalCentered => {
                ui.vertical_centered(|ui| self.children(ui, node))
                    .response
                    .rect
            }
            WidgetKind::Indent => {
                let before = ui.min_rect();
                ui.indent(|ui| self.children(ui, node));
                let after = ui.min_rect();
                Rect::from_min_max(rustroke::point(after.min.x, before.max.y), after.max)
            }
            WidgetKind::Grid => {
                let mut grid = Grid::new(node.text("id"));
                if node.flag("striped") {
                    grid = grid.striped(true);
                }
                if let Some(spacing) = size_prop(node, "spacing") {
                    grid = grid.spacing(spacing);
                }
                grid.show(ui, |ui| self.children(ui, node)).response.rect
            }
            WidgetKind::EndRow => {
                ui.end_row();
                Rect::NOTHING
            }
            WidgetKind::Collapsing => {
                let mut header = text!(
                    CollapsingHeader::new(text).default_open(node.flag("default_open")),
                    node,
                    "header_color",
                    "header_font"
                );
                if self.design() {
                    // Always open while designing, so the content can be edited.
                    header = header.open(Some(true));
                }
                let response = header.show(ui, |ui| self.children(ui, node));
                // The header reacts to clicks itself: cover it too.
                self.overlay(ui, id, response.header_response.rect, "header");
                response.header_response.rect
            }
            WidgetKind::ScrollArea => {
                let mut area = match node.text("direction") {
                    "horizontal" => ScrollArea::horizontal(),
                    "both" => ScrollArea::both(),
                    _ => ScrollArea::vertical(),
                };
                if let Some(height) = f32_prop(node, "max_height") {
                    area = area.max_height(height);
                }
                if let Some(width) = f32_prop(node, "max_width") {
                    area = area.max_width(width);
                }
                if node.prop("auto_shrink") == Some(&Value::Bool(false)) {
                    area = area.auto_shrink(false);
                }
                area.show(ui, |ui| self.children(ui, node)).response.rect
            }
            WidgetKind::Flex => {
                let mut flex = if node.text("direction") == "column" {
                    Flex::column(node.text("id"))
                } else {
                    Flex::row(node.text("id"))
                };
                if node.flag("wrap") {
                    flex = flex.wrap(true);
                }
                if let Some(gap) = f32_prop(node, "gap") {
                    flex = flex.gap(gap);
                }
                if let Some(name) = node.prop("justify").and_then(Value::as_text) {
                    flex = flex.justify(justify(name));
                }
                if let Some(name) = node.prop("align").and_then(Value::as_text) {
                    flex = flex.align(align(name));
                }
                if node.flag("fill_width") {
                    flex = flex.fill_width(true);
                }
                if node.flag("fill_height") {
                    flex = flex.fill_height(true);
                }
                flex.show(ui, |flex| {
                    if node.children.is_empty() {
                        flex.add(FlexItem::new(), |ui| self.placeholder(ui, "Empty flex"));
                    }
                    for item in &node.children {
                        let mut flex_item = FlexItem::new();
                        if let Some(v) = f32_prop(item, "grow") {
                            flex_item = flex_item.grow(v);
                        }
                        if let Some(v) = f32_prop(item, "shrink") {
                            flex_item = flex_item.shrink(v);
                        }
                        if let Some(v) = f32_prop(item, "basis") {
                            flex_item = flex_item.basis(v);
                        }
                        if let Some(name) = item.prop("align").and_then(Value::as_text) {
                            flex_item = flex_item.align(align(name));
                        }
                        if let Some(size) = size_prop(item, "min_size") {
                            flex_item = flex_item.min_size(size);
                        }
                        if let Some(size) = size_prop(item, "max_size") {
                            flex_item = flex_item.max_size(size);
                        }
                        let item_id = item.id;
                        if let Some(rect) = self.prev_rects.get(&item_id).copied() {
                            self.overlay(flex.ui(), item_id, rect, "area");
                        }
                        let response = flex.add(flex_item, |ui| {
                            ui.push_id(item_id.0, |ui| self.children(ui, item));
                        });
                        self.record(item, response.response.rect);
                    }
                })
                .response
                .rect
            }
            WidgetKind::FlexGrid => {
                let columns: Vec<Track> =
                    parse_tracks(node.text("columns")).unwrap_or_else(|_| vec![Track::Auto]);
                let columns = columns
                    .into_iter()
                    .map(|t| match t {
                        Track::Points(v) => rustroke::Track::Points(v as f32),
                        Track::Fraction(v) => rustroke::Track::Fraction(v as f32),
                        Track::Auto => rustroke::Track::Auto,
                    })
                    .collect();
                let mut grid = FlexGrid::new(node.text("id"), columns);
                if let Some(gap) = f32_prop(node, "gap") {
                    grid = grid.gap(gap);
                }
                grid.show(ui, |grid| {
                    if node.children.is_empty() {
                        grid.add(GridCell::at(0, 0), |ui| {
                            self.placeholder(ui, "Empty flex grid");
                        });
                    }
                    for cell in &node.children {
                        let at = |key| usize_prop(cell, key).unwrap_or(0).min(999) as u16;
                        let place = GridCell::at(at("column"), at("row"))
                            .span(at("column_span").max(1), at("row_span").max(1));
                        let cell_id = cell.id;
                        if let Some(rect) = self.prev_rects.get(&cell_id).copied() {
                            self.overlay(grid.ui(), cell_id, rect, "area");
                        }
                        let response = grid.add(place, |ui| {
                            ui.push_id(cell_id.0, |ui| self.children(ui, cell));
                        });
                        self.record(cell, response.response.rect);
                    }
                })
                .response
                .rect
            }
            // Only reachable when a FlexItem or a GridCell is drawn outside
            // its container (an invalid tree): draw its content as a column.
            WidgetKind::FlexItem | WidgetKind::GridCell => {
                ui.vertical(|ui| self.children(ui, node)).response.rect
            }
            WidgetKind::RawCode => {
                let code = node.text("code");
                if only_comments(code) {
                    Rect::NOTHING
                } else if self.design() {
                    let first = code.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
                    let more = if code.trim().lines().count() > 1 {
                        " …"
                    } else {
                        ""
                    };
                    let weak = ui.style().visuals.weak_text;
                    let label = Label::new(format!("{{ {} {more}}}", first.trim()))
                        .style(TextStyle::monospace(12.0))
                        .color(weak)
                        .no_wrap();
                    ui.add(label).rect
                } else {
                    Rect::NOTHING
                }
            }
        };
        self.record(node, rect);
        if !node.kind.is_container() {
            self.overlay(ui, id, rect, "widget");
        }
    }
}

/// Draws `view` in `ui` and reports clicks on its nodes (design mode).
pub fn show_view(
    ui: &mut Ui<'_>,
    view: &View,
    state: &mut CanvasState,
    mode: Mode,
    selected: Option<NodeId>,
) -> CanvasOutput {
    let prev_rects = std::mem::take(&mut state.rects);
    let background = ui.available_rect();
    let mut painter = Painter {
        mode,
        bindings: &mut state.bindings,
        prev_rects: &prev_rects,
        rects: HashMap::new(),
        out: CanvasOutput::default(),
    };
    if mode == Mode::Design {
        // Clicks on the empty canvas deselect; registered first so every
        // node is above it.
        let response = ui.interact(
            Id::new("rtk-canvas-background"),
            background,
            Sense::POINTER_DRAG,
        );
        painter.out.background_clicked = response.clicked();
    }
    ui.push_id("rtk-view", |ui| {
        apply_content_style(ui, &view.style);
        if view.nodes.is_empty() {
            painter.placeholder(ui, "Empty view: add widgets from the menus above");
        }
        painter.nodes(ui, &view.nodes);
    });
    let Painter { rects, mut out, .. } = painter;
    if mode == Mode::Design {
        let accent = ui.style().visuals.accent;
        if let Some(rect) = out.hovered.and_then(|id| rects.get(&id))
            && out.hovered != selected
        {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                3.0,
                Stroke::new(1.0, accent.with_alpha(0.5)),
            );
        }
        if let Some(rect) = selected.and_then(|id| rects.get(&id)) {
            ui.painter()
                .rect_stroke(rect.expand(2.0), 3.0, Stroke::new(2.0, accent));
        }
        if out.clicked.is_some() {
            out.background_clicked = false;
        }
    }
    state.rects = rects;
    out
}

/// Where a dragged node would land on the canvas.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropTarget {
    /// The new parent (`None` = the view's top level).
    pub parent: Option<NodeId>,
    /// Index among the parent's children, counted before a moved node is
    /// taken out (as [`View::move_node`] expects).
    pub index: usize,
    /// What to draw to show it.
    pub marker: DropMarker,
}

/// How a drop target is shown.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DropMarker {
    /// A line between two widgets.
    Line(rustroke::Point, rustroke::Point),
    /// An empty container, outlined.
    Area(Rect),
}

/// Whether children of `kind` flow left to right (else top to bottom).
fn flows_horizontally(view: &View, parent: Option<NodeId>) -> bool {
    let kind = parent.and_then(|id| view.node(id));
    match kind {
        Some(node) => match node.kind {
            WidgetKind::Horizontal | WidgetKind::Grid | WidgetKind::FlexGrid => true,
            WidgetKind::Flex => node.text("direction") != "column",
            _ => false,
        },
        None => false,
    }
}

impl CanvasState {
    /// Where a node of `kind` dropped at `pointer` would go: inside the
    /// container under the pointer if it accepts it, else before or after
    /// the widget under the pointer (or in the nearest ancestor that
    /// accepts it). `moving` is the node being moved, if any: it and its
    /// children are not targets. `area` is the window's content area: the
    /// view's top level when the pointer is on no widget.
    pub fn drop_target(
        &self,
        view: &View,
        pointer: rustroke::Point,
        kind: WidgetKind,
        moving: Option<NodeId>,
        area: Rect,
    ) -> Option<DropTarget> {
        if !area.contains(pointer) {
            return None;
        }
        // The deepest node under the pointer (outside the moved one).
        fn deepest(
            state: &CanvasState,
            nodes: &[Node],
            pointer: rustroke::Point,
            moving: Option<NodeId>,
        ) -> Option<NodeId> {
            nodes.iter().rev().find_map(|node| {
                if Some(node.id) == moving {
                    return None;
                }
                let rect = state.rect(node.id)?;
                if !rect.contains(pointer) {
                    return None;
                }
                deepest(state, &node.children, pointer, moving).or(Some(node.id))
            })
        }
        let mut parent = match deepest(self, &view.nodes, pointer, moving) {
            Some(id) => {
                let node = view.node(id)?;
                if node.kind.is_container() && view.accepts_child(Some(id), kind) {
                    Some(id)
                } else {
                    view.parent(id)?
                }
            }
            None => None,
        };
        // Climb until a parent accepts the node.
        while !view.accepts_child(parent, kind) {
            parent = view.parent(parent?)?;
        }
        Some(self.place_among(view, parent, pointer, moving, area))
    }

    /// The index among `parent`'s children where `pointer` falls: in a
    /// column, before the first widget whose middle is below it; in a row,
    /// in reading order (top to bottom, then left to right).
    fn place_among(
        &self,
        view: &View,
        parent: Option<NodeId>,
        pointer: rustroke::Point,
        moving: Option<NodeId>,
        area: Rect,
    ) -> DropTarget {
        let children = view.children(parent).unwrap_or(&[]);
        let parent_rect = parent.and_then(|id| self.rect(id)).unwrap_or(area);
        let horizontal = flows_horizontally(view, parent);
        let placed: Vec<(usize, Rect)> = children
            .iter()
            .enumerate()
            .filter(|(_, n)| Some(n.id) != moving)
            .filter_map(|(i, n)| Some((i, self.rect(n.id)?)))
            .filter(|(_, r)| r.width() > 0.0 && r.height() > 0.0)
            .collect();
        let after = placed.iter().find(|(_, r)| {
            if horizontal {
                // Reading order: a later row, or further right in this one.
                r.min.y > pointer.y
                    || (r.min.y <= pointer.y && pointer.y <= r.max.y && r.center().x > pointer.x)
            } else {
                // Above the middle of a widget: before it.
                r.center().y > pointer.y
            }
        });
        let line = |rect: Rect, before: bool| {
            if horizontal {
                let x = if before {
                    rect.min.x - 2.0
                } else {
                    rect.max.x + 2.0
                };
                DropMarker::Line(
                    rustroke::point(x, rect.min.y),
                    rustroke::point(x, rect.max.y),
                )
            } else {
                let y = if before {
                    rect.min.y - 2.0
                } else {
                    rect.max.y + 2.0
                };
                DropMarker::Line(
                    rustroke::point(rect.min.x, y),
                    rustroke::point(parent_rect.max.x.max(rect.max.x), y),
                )
            }
        };
        match (after, placed.last()) {
            (Some(&(index, rect)), _) => DropTarget {
                parent,
                index,
                marker: line(rect, true),
            },
            (None, Some(&(_, rect))) => DropTarget {
                parent,
                index: children.len(),
                marker: line(rect, false),
            },
            (None, None) => DropTarget {
                parent,
                index: children.len(),
                marker: DropMarker::Area(parent_rect),
            },
        }
    }
}
