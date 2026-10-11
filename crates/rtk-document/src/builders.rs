//! Builder methods (`.hint_text("…")`, `.fill(color)`…) of each kind,
//! shared by the code generator and the parser so both agree on them.

use crate::kind::WidgetKind;

/// How a builder method's argument is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Arg {
    /// A string literal.
    Str,
    /// A float literal.
    Float,
    /// An integer literal.
    Int,
    /// `true` / `false`; written only when it differs from the property's
    /// initial value.
    Bool,
    /// No argument: sets the property to `true` (`.no_wrap()`).
    Flag,
    /// `min..=max`
    Range,
    /// `Type::Variant`
    Variant(&'static str),
    /// No argument: sets the property to this text (`.vertical()`).
    Set(&'static str),
    /// `Color::from_srgb8(r, g, b)` / `Color::from_srgba8(r, g, b, a)`
    Color,
    /// `vec2(x, y)`, where an unlimited side is `f32::INFINITY`
    Size,
    /// `Stroke::new(width, color)`
    Stroke,
    /// `TextStyle::proportional(size).weight(w).bold()`
    Font,
    /// `Shadow { offset: vec2(x, y), blur, spread, color }`
    Shadow,
    /// Read but not kept (derived from other properties).
    Ignore,
}

/// Builder methods each kind accepts, in the order the generator writes
/// them: method, property key, argument.
pub(crate) fn builder_methods(kind: WidgetKind) -> &'static [(&'static str, &'static str, Arg)] {
    use Arg::*;
    const FRAME: [(&str, &str, Arg); 4] = [
        ("fill", "fill", Color),
        ("stroke", "stroke", Stroke),
        ("corner_radius", "corner_radius", Float),
        ("min_size", "min_size", Size),
    ];
    const TEXT: [(&str, &str, Arg); 2] =
        [("text_color", "text_color", Color), ("font", "font", Font)];
    match kind {
        WidgetKind::Label => &[
            ("no_wrap", "no_wrap", Flag),
            ("style", "font", Font),
            ("color", "color", Color),
        ],
        // `.style(ui.style().heading.clone())` is read by the parser itself.
        WidgetKind::Heading => &[("color", "color", Color)],
        WidgetKind::Hyperlink | WidgetKind::Checkbox => &TEXT,
        // Written as `RadioButton::new(..)` / `SelectableLabel::new(..)`.
        WidgetKind::RadioValue | WidgetKind::SelectableValue => &TEXT,
        WidgetKind::Separator => &[
            ("horizontal", "orientation", Set("horizontal")),
            ("vertical", "orientation", Set("vertical")),
            ("stroke", "stroke", Stroke),
            ("spacing", "spacing", Float),
        ],
        WidgetKind::Button => &[
            ("selected", "selected", Bool),
            ("shortcut_text", "shortcut_text", Str),
            ("frame", "frame", Bool),
            TEXT[0],
            TEXT[1],
            FRAME[0],
            FRAME[1],
            FRAME[2],
            FRAME[3],
        ],
        WidgetKind::Slider => &[
            ("text", "text", Str),
            ("step", "step", Float),
            ("show_value", "show_value", Bool),
            ("color", "bar_color", Color),
        ],
        WidgetKind::DragValue => &[
            ("speed", "speed", Float),
            ("range", "range", Range),
            ("prefix", "prefix", Str),
            ("suffix", "suffix", Str),
            ("decimals", "decimals", Int),
        ],
        WidgetKind::ProgressBar => &[
            ("text", "text", Str),
            ("show_percentage", "show_percentage", Flag),
            ("animate", "animate", Bool),
            ("fill", "bar_color", Color),
            ("desired_width", "desired_width", Float),
        ],
        WidgetKind::Spinner => &[("size", "size", Float), ("color", "color", Color)],
        WidgetKind::TextEdit => &[
            ("hint_text", "hint_text", Str),
            ("password", "password", Bool),
            ("code_editor", "code_editor", Flag),
            ("wrap", "wrap", Bool),
            ("line_numbers", "line_numbers", Bool),
            ("select_all_on_focus", "select_all_on_focus", Bool),
            ("font", "font", Font),
            ("margin", "margin", Size),
            ("desired_width", "desired_width", Float),
            ("desired_rows", "desired_rows", Int),
            FRAME[0],
            FRAME[1],
            FRAME[2],
            FRAME[3],
        ],
        WidgetKind::SearchField => &[
            ("hint_text", "hint_text", Str),
            ("desired_width", "desired_width", Float),
            FRAME[0],
            FRAME[1],
            FRAME[2],
            FRAME[3],
        ],
        WidgetKind::ComboBox => &[
            ("width", "width", Float),
            FRAME[0],
            FRAME[1],
            FRAME[2],
            FRAME[3],
            ("selected_text", "", Ignore),
        ],
        WidgetKind::Grid => &[("striped", "striped", Bool), ("spacing", "spacing", Size)],
        WidgetKind::ColorPicker => &[("alpha", "alpha", Bool)],
        WidgetKind::DatePicker => &[("sunday_first", "sunday_first", Bool)],
        WidgetKind::Collapsing => &[
            ("default_open", "default_open", Bool),
            ("text_color", "header_color", Color),
            ("font", "header_font", Font),
        ],
        WidgetKind::Container => &[
            FRAME[0],
            FRAME[1],
            FRAME[2],
            ("shadow", "shadow", Shadow),
            ("inner_margin", "inner_margin", Size),
            ("outer_margin", "outer_margin", Size),
            FRAME[3],
            ("max_size", "max_size", Size),
            ("fill_width", "fill_width", Bool),
            ("fill_height", "fill_height", Bool),
        ],
        WidgetKind::Panel => &[
            ("resizable", "resizable", Bool),
            ("default_size", "default_size", Float),
            ("min_size", "min", Float),
            ("max_size", "max", Float),
            ("margin", "margin", Float),
        ],
        WidgetKind::CentralPanel => &[("margin", "margin", Float)],
        WidgetKind::ScrollArea => &[
            ("max_height", "max_height", Float),
            ("max_width", "max_width", Float),
            ("auto_shrink", "auto_shrink", Bool),
        ],
        WidgetKind::Flex => &[
            ("wrap", "wrap", Bool),
            ("gap", "gap", Float),
            ("justify", "justify", Variant("FlexJustify")),
            ("align", "align", Variant("FlexAlign")),
            ("fill_width", "fill_width", Bool),
            ("fill_height", "fill_height", Bool),
        ],
        WidgetKind::FlexItem => &[
            ("grow", "grow", Float),
            ("shrink", "shrink", Float),
            ("basis", "basis", Float),
            ("align", "align", Variant("FlexAlign")),
            ("min_size", "min_size", Size),
            ("max_size", "max_size", Size),
        ],
        WidgetKind::FlexGrid => &[("gap", "gap", Float)],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::{Init, PropKind};

    /// Every builder sets a property of its kind, with a matching type.
    #[test]
    fn builders_match_the_specs() {
        for kind in WidgetKind::ALL {
            for &(method, key, arg) in builder_methods(kind) {
                if arg == Arg::Ignore {
                    continue;
                }
                let spec = kind
                    .prop(key)
                    .unwrap_or_else(|| panic!("{kind:?}.{method}: no property {key}"));
                let ok = match arg {
                    Arg::Str => spec.kind == PropKind::Text,
                    Arg::Float => matches!(spec.kind, PropKind::Float { .. }),
                    Arg::Int => matches!(spec.kind, PropKind::Int { .. }),
                    Arg::Bool => spec.kind == PropKind::Bool && spec.init != Init::None,
                    Arg::Flag => spec.kind == PropKind::Bool,
                    Arg::Range => spec.kind == PropKind::Range,
                    Arg::Variant(_) => matches!(spec.kind, PropKind::Choice(_)),
                    Arg::Set(text) => {
                        matches!(spec.kind, PropKind::Choice(c) if c.contains(&text))
                    }
                    Arg::Color => spec.kind == PropKind::Color,
                    Arg::Size => matches!(spec.kind, PropKind::Size | PropKind::Limit),
                    Arg::Shadow => spec.kind == PropKind::Shadow,
                    Arg::Stroke => spec.kind == PropKind::Stroke,
                    Arg::Font => spec.kind == PropKind::Font,
                    Arg::Ignore => true,
                };
                assert!(ok, "{kind:?}.{method}: {arg:?} for {:?}", spec.kind);
            }
        }
    }
}
