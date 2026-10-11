//! Properties of the selected node (or of the view, with nothing selected).
//!
//! Text values are edited in a buffer: while a field has the keyboard it
//! keeps what the user types, even if invalid (a binding name being typed),
//! and only valid values are applied.

use std::collections::HashMap;

use rtk_document::names::check_snake_case;
use rtk_document::{
    CONTENT_STYLE, Font, FontFamily, NodeId, PropKind, PropSpec, Shadow, Value, WindowSettings,
    parse_tracks,
};
use rustroke::{
    Checkbox, ColorPicker, ComboBox, DragValue, Id, Label, PropertyGrid, PropertyGridUi,
    ScrollArea, TextEdit, TextStyle, Ui,
};

use crate::app::{Dialog, Toolkit};

/// Widest value editors.
const MAX_VALUE_WIDTH: f32 = 220.0;

/// Edit buffers of text fields, by node and property.
#[derive(Debug, Default)]
pub struct PropertiesPanel {
    buffers: HashMap<(NodeId, &'static str), String>,
}

/// How a property text field looks.
struct Field<'a> {
    /// Property label (for screen readers).
    label: &'a str,
    width: f32,
    multiline: bool,
}

/// A text field over a buffer; returns the new text when it changed and is
/// `valid`. Shows the error under the field while the text isn't valid.
fn text_field(
    ui: &mut Ui<'_>,
    field: Field<'_>,
    buffers: &mut HashMap<(NodeId, &'static str), String>,
    key: (NodeId, &'static str),
    current: &str,
    valid: impl Fn(&str) -> Result<(), String>,
) -> Option<String> {
    let Field {
        label,
        width,
        multiline,
    } = field;
    let buffer = buffers.entry(key).or_insert_with(|| current.to_owned());
    let edit = if multiline {
        TextEdit::multiline(buffer).desired_rows(4)
    } else {
        TextEdit::singleline(buffer)
    };
    // An id of its own per node and property: when the selection changes,
    // the keyboard focus doesn't move to the next node's field.
    let response = ui.add(
        edit.id(Id::new(("property field", key.0.0, key.1)))
            .desired_width(width)
            .accessible_label(format!("{label} property")),
    );
    let result = valid(buffer);
    let changed = response.changed() && result.is_ok() && buffer != current;
    let new = changed.then(|| buffer.clone());
    if !response.has_focus() && !response.changed() && buffer != current {
        // Not being edited: show the model's value.
        buffer.clear();
        buffer.push_str(current);
    }
    if let Err(message) = result
        && response.has_focus()
    {
        let red = ui.style().visuals.error;
        ui.add(Label::new(message).color(red));
    }
    new
}

/// A color picker over sRGBA components; returns the new ones when changed.
fn color_editor(ui: &mut Ui<'_>, rgba: [u8; 4]) -> Option<[u8; 4]> {
    let mut color = rtk_preview::color(rgba);
    let changed = ui.add(ColorPicker::new(&mut color).alpha(true)).changed();
    let new = color.to_srgba8();
    (changed && new != rgba).then_some(new)
}

/// A checkbox that sets or unsets an optional property, before its editor.
/// Returns `Some(new value)` when toggled (`set` is the value to start
/// from).
fn set_toggle(
    ui: &mut Ui<'_>,
    spec: &PropSpec,
    current: Option<&Value>,
    set: impl FnOnce() -> Value,
) -> Option<Option<Value>> {
    let mut on = current.is_some();
    let toggle = Checkbox::new(&mut on, "").accessible_label(format!("Set {}", spec.label));
    (spec.optional() && ui.add(toggle).changed()).then(|| on.then(set))
}

fn size_drag(ui: &mut Ui<'_>, value: &mut f64) -> bool {
    ui.add(
        DragValue::new(value)
            .speed(0.5)
            .range(0.0..=10_000.0)
            .decimals(0),
    )
    .changed()
}

/// One side of a size limit: a checkbox to limit it, then its value, or
/// "∞" without a limit (`f64::INFINITY`).
fn limit_side(ui: &mut Ui<'_>, value: &mut f64, label: &str) -> bool {
    let mut limited = value.is_finite();
    let toggle = Checkbox::new(&mut limited, "").accessible_label(format!("Limit {label}"));
    if ui.add(toggle).changed() {
        *value = if limited { 300.0 } else { f64::INFINITY };
        return true;
    }
    if limited {
        size_drag(ui, value)
    } else {
        ui.label("∞");
        false
    }
}

fn offset_drag(ui: &mut Ui<'_>, value: &mut f64) -> bool {
    ui.add(
        DragValue::new(value)
            .speed(0.2)
            .range(-1000.0..=1000.0)
            .decimals(1),
    )
    .changed()
}

/// The editor of one property; returns the value to set (`Some(None)` to
/// unset an optional property). `id` identifies the node (0 for the view)
/// for the text buffers.
fn prop_editor(
    ui: &mut Ui<'_>,
    width: f32,
    buffers: &mut HashMap<(NodeId, &'static str), String>,
    id: NodeId,
    current: Option<&Value>,
    spec: &PropSpec,
) -> Option<Option<Value>> {
    let key = (id, spec.key);
    let optional = spec.optional();
    let line = |width| Field {
        label: spec.label,
        width,
        multiline: false,
    };
    let lines = Field {
        label: spec.label,
        width,
        multiline: true,
    };
    match spec.kind {
        PropKind::Text | PropKind::IdSalt => {
            let text = current.and_then(Value::as_text).unwrap_or("");
            let new = text_field(ui, line(width), buffers, key, text, |_| Ok(()))?;
            // An optional text left empty is unset.
            Some((!(optional && new.is_empty())).then_some(Value::Text(new)))
        }
        PropKind::Binding(ty) => {
            let text = current.and_then(Value::as_text).unwrap_or("");
            let new = ui
                .horizontal(|ui| {
                    // Room for the type after the field.
                    let width = (width - 50.0).max(60.0);
                    let new = text_field(ui, line(width), buffers, key, text, |t| {
                        check_snake_case(t).map_err(|e| e.to_string())
                    });
                    let weak = ui.style().visuals.weak_text;
                    ui.add(Label::new(ty.rust_name()).color(weak));
                    new
                })
                .inner?;
            Some(Some(Value::Text(new)))
        }
        PropKind::Code => {
            let text = current.and_then(Value::as_text).unwrap_or("");
            ui.style_mut().body = TextStyle::monospace(12.0);
            let new = text_field(ui, lines, buffers, key, text, |_| Ok(()))?;
            Some(Some(Value::Text(new)))
        }
        PropKind::Items => {
            let items = current.and_then(Value::as_items).unwrap_or(&[]);
            let text = items.join("\n");
            let new = text_field(ui, lines, buffers, key, &text, |_| Ok(()))?;
            let items = new.lines().map(str::to_owned).collect();
            Some(Some(Value::Items(items)))
        }
        PropKind::Bool => {
            let mut value = current.and_then(Value::as_bool).unwrap_or(false);
            ui.add(Checkbox::new(&mut value, ""))
                .changed()
                .then_some(Some(Value::Bool(value)))
        }
        PropKind::Float { min, max } => {
            ui.horizontal(|ui| {
                let mut set = current.is_some();
                if optional && ui.add(Checkbox::new(&mut set, "")).changed() {
                    return Some(set.then_some(Value::Float(0.0f64.clamp(min, max))));
                }
                if !set {
                    return None;
                }
                let mut value = current.and_then(Value::as_f64).unwrap_or(0.0);
                ui.add(DragValue::new(&mut value).speed(0.5).range(min..=max))
                    .changed()
                    .then_some(Some(Value::Float(value)))
            })
            .inner
        }
        PropKind::Int { min, max } => {
            ui.horizontal(|ui| {
                let mut set = current.is_some();
                if optional && ui.add(Checkbox::new(&mut set, "")).changed() {
                    return Some(set.then_some(Value::Int(min.max(0).min(max))));
                }
                if !set {
                    return None;
                }
                let mut value = current.and_then(Value::as_int).unwrap_or(0);
                ui.add(DragValue::new(&mut value).speed(0.1).range(min..=max))
                    .changed()
                    .then_some(Some(Value::Int(value)))
            })
            .inner
        }
        PropKind::Range => {
            ui.horizontal(|ui| {
                let mut set = current.is_some();
                if optional && ui.add(Checkbox::new(&mut set, "")).changed() {
                    return Some(set.then_some(Value::Range(0.0, 100.0)));
                }
                let (mut lo, mut hi) = current.and_then(Value::as_range)?;
                let a = ui.add(DragValue::new(&mut lo).speed(0.5)).changed();
                ui.label("..=");
                let b = ui.add(DragValue::new(&mut hi).speed(0.5)).changed();
                (a || b).then_some(Some(Value::Range(lo, hi)))
            })
            .inner
        }
        PropKind::Choice(choices) => {
            let text = current.and_then(Value::as_text);
            let mut choice = text.map(str::to_owned);
            ComboBox::from_id_salt(("choice", id.0, spec.key))
                .selected_text(text.unwrap_or("(default)"))
                .width(width)
                .show_ui(ui, |ui| {
                    if optional {
                        ui.selectable_value(&mut choice, None, "(default)");
                    }
                    for c in choices {
                        ui.selectable_value(&mut choice, Some((*c).to_owned()), *c);
                    }
                });
            (choice.as_deref() != text).then(|| choice.map(Value::Text))
        }
        PropKind::Tracks => {
            let text = current.and_then(Value::as_text).unwrap_or("");
            let new = text_field(ui, line(width), buffers, key, text, |t| {
                parse_tracks(t).map(|_| ())
            })?;
            Some(Some(Value::Text(new)))
        }
        PropKind::Color => {
            ui.horizontal(|ui| {
                if let Some(toggled) =
                    set_toggle(ui, spec, current, || Value::Color([128, 128, 128, 255]))
                {
                    return Some(toggled);
                }
                let rgba = current.and_then(Value::as_color)?;
                let new = color_editor(ui, rgba)?;
                Some(Some(Value::Color(new)))
            })
            .inner
        }
        PropKind::Size => {
            ui.horizontal(|ui| {
                if let Some(toggled) = set_toggle(ui, spec, current, || Value::Size(100.0, 28.0)) {
                    return Some(toggled);
                }
                let (mut w, mut h) = current.and_then(Value::as_size)?;
                let a = size_drag(ui, &mut w);
                ui.label("×");
                let b = size_drag(ui, &mut h);
                (a || b).then_some(Some(Value::Size(w, h)))
            })
            .inner
        }
        PropKind::Limit => {
            ui.horizontal(|ui| {
                if let Some(toggled) =
                    set_toggle(ui, spec, current, || Value::Size(300.0, f64::INFINITY))
                {
                    return Some(toggled);
                }
                let (mut w, mut h) = current.and_then(Value::as_size)?;
                let a = limit_side(ui, &mut w, "width");
                ui.label("×");
                let b = limit_side(ui, &mut h, "height");
                (a || b).then_some(Some(Value::Size(w, h)))
            })
            .inner
        }
        PropKind::Shadow => {
            ui.vertical(|ui| {
                let toggled = ui
                    .horizontal(|ui| {
                        let toggled =
                            set_toggle(ui, spec, current, || Value::Shadow(Shadow::default()));
                        let shadow = current.and_then(Value::as_shadow);
                        let color = shadow.and_then(|s| color_editor(ui, s.color));
                        match (toggled, shadow, color) {
                            (Some(t), _, _) => Some(t),
                            (None, Some(s), Some(color)) => {
                                Some(Some(Value::Shadow(Shadow { color, ..s })))
                            }
                            _ => None,
                        }
                    })
                    .inner;
                if toggled.is_some() {
                    return toggled;
                }
                let mut s = current.and_then(Value::as_shadow)?;
                let offset = ui
                    .horizontal(|ui| {
                        ui.label("Offset");
                        let a = offset_drag(ui, &mut s.offset.0);
                        let b = offset_drag(ui, &mut s.offset.1);
                        a || b
                    })
                    .inner;
                let sizes = ui
                    .horizontal(|ui| {
                        ui.label("Blur");
                        let a = size_drag(ui, &mut s.blur);
                        ui.label("Spread");
                        let b = offset_drag(ui, &mut s.spread);
                        a || b
                    })
                    .inner;
                (offset || sizes).then_some(Some(Value::Shadow(s)))
            })
            .inner
        }
        PropKind::Stroke => {
            ui.horizontal(|ui| {
                if let Some(toggled) = set_toggle(ui, spec, current, || {
                    Value::Stroke(1.0, [128, 128, 128, 255])
                }) {
                    return Some(toggled);
                }
                let (mut w, rgba) = current.and_then(Value::as_stroke)?;
                let a = ui
                    .add(
                        DragValue::new(&mut w)
                            .speed(0.1)
                            .range(0.0..=100.0)
                            .suffix(" pt"),
                    )
                    .changed();
                let color = color_editor(ui, rgba);
                (a || color.is_some()).then(|| Some(Value::Stroke(w, color.unwrap_or(rgba))))
            })
            .inner
        }
        PropKind::Font => {
            ui.vertical(|ui| {
                let toggled = ui
                    .horizontal(|ui| {
                        let toggled =
                            set_toggle(ui, spec, current, || Value::Font(Font::default()));
                        if current.is_none() {
                            let weak = ui.style().visuals.weak_text;
                            ui.add(Label::new("(theme)").color(weak));
                        }
                        toggled
                    })
                    .inner;
                if toggled.is_some() {
                    return toggled;
                }
                let font = current.and_then(Value::as_font)?;
                let mut new = font;
                ui.horizontal(|ui| {
                    ComboBox::from_id_salt(("font family", id.0, spec.key))
                        .selected_text(font.family.ctor())
                        .width(110.0)
                        .show_ui(ui, |ui| {
                            for family in FontFamily::ALL {
                                ui.selectable_value(&mut new.family, family, family.ctor());
                            }
                        });
                    ui.add(
                        DragValue::new(&mut new.size)
                            .speed(0.2)
                            .range(1.0..=200.0)
                            .suffix(" pt"),
                    );
                });
                ui.horizontal(|ui| {
                    ui.add(
                        DragValue::new(&mut new.weight)
                            .speed(2.0)
                            .range(100..=900)
                            .prefix("weight "),
                    );
                    ui.add(Checkbox::new(&mut new.bold, "Bold"));
                });
                (new != font).then_some(Some(Value::Font(new)))
            })
            .inner
        }
    }
}

/// Rows for `props`, the unnamed group first, then one section per group.
fn prop_rows(
    grid: &mut PropertyGridUi<'_, '_>,
    buffers: &mut HashMap<(NodeId, &'static str), String>,
    id: NodeId,
    props: &[PropSpec],
    current: impl Fn(&str) -> Option<Value>,
) -> Option<(&'static str, Option<Value>)> {
    let width = grid.value_width().min(MAX_VALUE_WIDTH);
    let mut change = None;
    let mut groups: Vec<&str> = Vec::new();
    for spec in props {
        if !groups.contains(&spec.group) {
            groups.push(spec.group);
        }
    }
    for group in groups {
        let mut rows = |grid: &mut PropertyGridUi<'_, '_>| {
            for spec in props.iter().filter(|s| s.group == group) {
                let value = current(spec.key);
                if let Some(new) = grid.row(spec.label, |ui| {
                    ui.push_id(spec.key, |ui| {
                        prop_editor(ui, width, buffers, id, value.as_ref(), spec)
                    })
                }) {
                    change = Some((spec.key, new));
                }
            }
        };
        if group.is_empty() {
            rows(grid);
        } else {
            grid.section(group, true, rows);
        }
    }
    change
}

fn node_properties(ui: &mut Ui<'_>, app: &mut Toolkit, id: NodeId) {
    let Some(node) = app.view().and_then(|v| v.node(id)).cloned() else {
        return;
    };
    let icon = app.icons().node(&node);
    let buffers = &mut app.properties.buffers;
    let change = PropertyGrid::new(("node", id.0))
        .header(Some(icon), node.kind.name())
        .show(ui, |grid| {
            prop_rows(grid, buffers, id, node.kind.spec().props, |key| {
                node.prop(key).cloned()
            })
        });
    if let Some((key, value)) = change {
        let tag = format!("prop {} {key}", id.0);
        app.edit_tagged(Some(&tag), |view| view.set_prop(id, key, value));
    }
    if node.kind.spec().props.is_empty() {
        let weak = ui.style().visuals.weak_text;
        ui.add(Label::new("No properties.").color(weak));
    }
}

/// Width, height and resizable rows of a window; returns whether one changed.
pub fn window_size_rows(grid: &mut PropertyGridUi<'_, '_>, window: &mut WindowSettings) -> bool {
    let range = WindowSettings::MIN_SIZE..=WindowSettings::MAX_SIZE;
    let mut changed = false;
    changed |= grid.row("Width", |ui| {
        ui.add(
            DragValue::new(&mut window.width)
                .speed(1.0)
                .range(range.clone())
                .suffix(" pt"),
        )
        .changed()
    });
    changed |= grid.row("Height", |ui| {
        ui.add(
            DragValue::new(&mut window.height)
                .speed(1.0)
                .range(range.clone())
                .suffix(" pt"),
        )
        .changed()
    });
    changed |= grid.row("Resizable", |ui| {
        ui.add(Checkbox::new(&mut window.resizable, "")).changed()
    });
    changed
}

/// An optional size limit of the window (min or max): a checkbox, then
/// width × height.
fn size_limit_row(
    grid: &mut PropertyGridUi<'_, '_>,
    name: &str,
    limit: &mut Option<(f32, f32)>,
    start: (f32, f32),
) -> bool {
    grid.row(name, |ui| {
        let mut on = limit.is_some();
        if ui.add(Checkbox::new(&mut on, "")).changed() {
            *limit = on.then_some(start);
            return true;
        }
        let Some((w, h)) = limit else {
            return false;
        };
        let range = WindowSettings::MIN_SIZE..=WindowSettings::MAX_SIZE;
        let a = ui
            .add(DragValue::new(w).speed(1.0).range(range.clone()))
            .changed();
        ui.label("×");
        let b = ui.add(DragValue::new(h).speed(1.0).range(range)).changed();
        a || b
    })
}

fn view_properties(ui: &mut Ui<'_>, app: &mut Toolkit) {
    let Some(view) = app.view().cloned() else {
        return;
    };
    let (fields, conflicts) = view.fields();
    let mut window_change = None;
    let mut style_change = None;
    PropertyGrid::new("view")
        .header(None, format!("Window of {}", view.name))
        .show(ui, |grid| {
            grid.row("File", |ui| ui.label(format!("{}.rs", view.name)));
            grid.row("Struct", |ui| ui.label(view.struct_name()));
            grid.row("Name", |ui| {
                if ui.button("Rename…").clicked() {
                    app.dialog = Some(Dialog::RenameView {
                        old: view.name.clone(),
                        name: view.name.clone(),
                    });
                }
            });
            grid.section("Window", true, |grid| {
                let width = grid.value_width().min(MAX_VALUE_WIDTH);
                let mut window = view.window.clone();
                let buffers = &mut app.properties.buffers;
                let title = grid.row("Title", |ui| {
                    let field = Field {
                        label: "Title",
                        width,
                        multiline: false,
                    };
                    text_field(
                        ui,
                        field,
                        buffers,
                        (NodeId(0), "title"),
                        &window.title,
                        |_| Ok(()),
                    )
                });
                let mut changed = false;
                if let Some(title) = title {
                    window.title = title;
                    changed = true;
                }
                changed |= window_size_rows(grid, &mut window);
                if window.resizable {
                    let size = (window.width, window.height);
                    changed |= size_limit_row(grid, "Min size", &mut window.min_size, size);
                    changed |= size_limit_row(grid, "Max size", &mut window.max_size, size);
                }
                if changed {
                    window_change = Some(window);
                }
            });
            let buffers = &mut app.properties.buffers;
            style_change = prop_rows(grid, buffers, NodeId(0), CONTENT_STYLE, |key| {
                view.style.get(key).cloned()
            });
            grid.section("Fields", true, |grid| {
                if fields.is_empty() && view.extra_fields.is_empty() {
                    grid.row("", |ui| ui.label("No fields."));
                }
                for field in &fields {
                    grid.row(&field.name, |ui| ui.label(field.ty.rust_name()));
                }
                for field in &view.extra_fields {
                    let weak = grid.ui().style().visuals.weak_text;
                    grid.row(&field.name, |ui| {
                        ui.add(Label::new("written in code").color(weak))
                    });
                }
            });
        });
    if let Some(window) = window_change {
        app.edit_tagged(Some("window"), |view| view.set_window(window));
    }
    if let Some((key, value)) = style_change {
        let tag = format!("view style {key}");
        app.edit_tagged(Some(&tag), |view| view.set_style(key, value));
    }
    if !conflicts.is_empty() {
        let red = ui.style().visuals.error;
        for c in conflicts {
            let text = match c.other {
                Some(other) => format!(
                    "`{}` is bound as {} and as {}",
                    c.name,
                    c.first.rust_name(),
                    other.rust_name()
                ),
                None => format!("`{}` is also a field written in code", c.name),
            };
            ui.add(Label::new(text).color(red));
        }
    }
}

pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit) {
    ui.heading("Properties");
    ui.separator();
    ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| match app.selected {
            Some(id) if app.view().is_some_and(|v| v.node(id).is_some()) => {
                node_properties(ui, app, id);
            }
            _ => view_properties(ui, app),
        });
}
