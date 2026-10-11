//! Property values and their types.

/// The value of a node property.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Text, a binding name, a choice, an id or code.
    Text(String),
    /// A flag.
    Bool(bool),
    /// A floating point number (`f32` in the generated code).
    Float(f64),
    /// An integer (`usize` in the generated code).
    Int(i64),
    /// An inclusive range of floats, `min..=max`.
    Range(f64, f64),
    /// A list of texts (combo box items).
    Items(Vec<String>),
    /// A color: sRGB red, green, blue and alpha, 8 bits each
    /// (`Color::from_srgb8` / `Color::from_srgba8`).
    Color([u8; 4]),
    /// A width and a height in points (`vec2(x, y)`).
    Size(f64, f64),
    /// A border: width in points and color (`Stroke::new(width, color)`).
    Stroke(f64, [u8; 4]),
    /// A text style (`TextStyle::proportional(size)…`).
    Font(Font),
    /// A drop shadow (`Shadow { offset, blur, spread, color }`).
    Shadow(Shadow),
}

/// A drop shadow behind a container.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    /// Moves the shadow, in points (`vec2(x, y)`).
    pub offset: (f64, f64),
    /// How far it fades out, in points.
    pub blur: f64,
    /// Grows (or, negative, shrinks) the shadow before blurring.
    pub spread: f64,
    /// Its color at full strength.
    pub color: [u8; 4],
}

impl Default for Shadow {
    fn default() -> Self {
        Self {
            offset: (0.0, 2.0),
            blur: 8.0,
            spread: 0.0,
            color: [0, 0, 0, 80],
        }
    }
}

/// Typeface of a [`Font`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    /// The bundled proportional font (`TextStyle::proportional`).
    Proportional,
    /// The platform's monospace font (`TextStyle::monospace`).
    Monospace,
    /// The platform's interface font (`TextStyle::system`).
    System,
}

impl FontFamily {
    /// Every family, in menu order.
    pub const ALL: [Self; 3] = [Self::Proportional, Self::Monospace, Self::System];

    /// The `TextStyle` constructor.
    pub fn ctor(self) -> &'static str {
        match self {
            Self::Proportional => "proportional",
            Self::Monospace => "monospace",
            Self::System => "system",
        }
    }

    /// The family of a `TextStyle` constructor.
    pub fn from_ctor(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.ctor() == name)
    }
}

/// A text style: `TextStyle::<family>(size)`, then `.weight(weight)` unless
/// it is 400 and `.bold()` if bold.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Font {
    /// Typeface.
    pub family: FontFamily,
    /// Size in points.
    pub size: f64,
    /// Weight from 100 to 900 (400 regular, 700 bold).
    pub weight: u16,
    /// Bold, whatever the weight.
    pub bold: bool,
}

impl Font {
    /// Regular weight.
    pub const REGULAR: u16 = 400;
}

impl Default for Font {
    fn default() -> Self {
        Self {
            family: FontFamily::Proportional,
            size: 14.0,
            weight: Self::REGULAR,
            bold: false,
        }
    }
}

impl Value {
    /// The text, if this is a text value.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    /// The flag, if this is a bool value.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The number, if this is a float or an integer.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(v) => Some(*v),
            Self::Int(v) => Some(*v as f64),
            _ => None,
        }
    }

    /// The integer, if this is an integer value.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// The bounds, if this is a range.
    pub fn as_range(&self) -> Option<(f64, f64)> {
        match self {
            Self::Range(min, max) => Some((*min, *max)),
            _ => None,
        }
    }

    /// The items, if this is a list.
    pub fn as_items(&self) -> Option<&[String]> {
        match self {
            Self::Items(items) => Some(items),
            _ => None,
        }
    }

    /// The sRGBA components, if this is a color.
    pub fn as_color(&self) -> Option<[u8; 4]> {
        match self {
            Self::Color(c) => Some(*c),
            _ => None,
        }
    }

    /// Width and height, if this is a size.
    pub fn as_size(&self) -> Option<(f64, f64)> {
        match self {
            Self::Size(w, h) => Some((*w, *h)),
            _ => None,
        }
    }

    /// Width and color, if this is a stroke.
    pub fn as_stroke(&self) -> Option<(f64, [u8; 4])> {
        match self {
            Self::Stroke(w, c) => Some((*w, *c)),
            _ => None,
        }
    }

    /// The text style, if this is a font.
    pub fn as_font(&self) -> Option<Font> {
        match self {
            Self::Font(f) => Some(*f),
            _ => None,
        }
    }

    /// The shadow, if this is one.
    pub fn as_shadow(&self) -> Option<Shadow> {
        match self {
            Self::Shadow(s) => Some(*s),
            _ => None,
        }
    }
}

/// Parses a list of [`FlexGrid`](https://docs.rs/rustroke) column tracks
/// written like `120, 1fr, auto`: points, shares of the free space (`fr`)
/// or `auto`.
pub fn parse_tracks(text: &str) -> Result<Vec<Track>, String> {
    let tracks: Vec<Track> = text
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| {
            let number = |n: &str| {
                n.trim()
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite() && *v >= 0.0)
                    .ok_or_else(|| format!("`{t}` is not a number, `<n>fr` or `auto`"))
            };
            if t == "auto" {
                Ok(Track::Auto)
            } else if let Some(n) = t.strip_suffix("fr") {
                number(n).map(Track::Fraction)
            } else {
                number(t.strip_suffix("pt").unwrap_or(t)).map(Track::Points)
            }
        })
        .collect::<Result<_, _>>()?;
    if tracks.is_empty() {
        return Err("at least one column".to_owned());
    }
    Ok(tracks)
}

/// Writes tracks the way [`parse_tracks`] reads them.
pub fn format_tracks(tracks: &[Track]) -> String {
    let num = |v: f64| {
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            v.to_string()
        }
    };
    tracks
        .iter()
        .map(|t| match t {
            Track::Points(v) => num(*v),
            Track::Fraction(v) => format!("{}fr", num(*v)),
            Track::Auto => "auto".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The size of one column of a flex grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Track {
    /// Fixed, in points (`Track::Points`).
    Points(f64),
    /// A share of the free space (`Track::Fraction`, `1fr`).
    Fraction(f64),
    /// As wide as its content (`Track::Auto`).
    Auto,
}

/// The Rust type of a view field bound to a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldType {
    /// `bool`
    Bool,
    /// `String`
    String,
    /// `f32`
    F32,
    /// `usize`
    Usize,
    /// `(u32, u32)`: hour and minute.
    Time,
    /// `Color`
    Color,
    /// `Date`
    Date,
}

impl FieldType {
    /// The type as written in Rust.
    pub fn rust_name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::String => "String",
            Self::F32 => "f32",
            Self::Usize => "usize",
            Self::Time => "(u32, u32)",
            Self::Color => "Color",
            Self::Date => "Date",
        }
    }

    /// Parses a Rust type name written as [`FieldType::rust_name`] gives it.
    pub fn from_rust_name(name: &str) -> Option<Self> {
        match name {
            "bool" => Some(Self::Bool),
            "String" => Some(Self::String),
            "f32" => Some(Self::F32),
            "usize" => Some(Self::Usize),
            "(u32, u32)" => Some(Self::Time),
            "Color" => Some(Self::Color),
            "Date" => Some(Self::Date),
            _ => None,
        }
    }
}

/// What kind of value a property holds, and how it is edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    /// Single-line text.
    Text,
    /// A flag.
    Bool,
    /// A float between `min` and `max`.
    Float {
        /// Smallest value.
        min: f64,
        /// Largest value.
        max: f64,
    },
    /// An integer between `min` and `max`.
    Int {
        /// Smallest value.
        min: i64,
        /// Largest value.
        max: i64,
    },
    /// A float range.
    Range,
    /// A list of texts.
    Items,
    /// The name of the view field the widget reads and writes.
    Binding(FieldType),
    /// One of a fixed set of texts.
    Choice(&'static [&'static str]),
    /// An id salt (`Grid::new("id")`), unique among siblings.
    IdSalt,
    /// Rust statements, kept as written.
    Code,
    /// A color.
    Color,
    /// A width and a height, each at least 0.
    Size,
    /// A largest width and height, each at least 0 or unlimited
    /// (`f64::INFINITY`, written `f32::INFINITY`).
    Limit,
    /// A drop shadow.
    Shadow,
    /// A border.
    Stroke,
    /// A text style.
    Font,
    /// Flex grid columns, as text (see [`parse_tracks`]).
    Tracks,
}

impl PropKind {
    /// Whether `value` is valid for this kind.
    pub fn accepts(&self, value: &Value) -> bool {
        match (self, value) {
            (Self::Text | Self::Code | Self::IdSalt, Value::Text(_)) => true,
            (Self::Binding(_), Value::Text(name)) => crate::names::check_snake_case(name).is_ok(),
            (Self::Choice(choices), Value::Text(text)) => choices.contains(&text.as_str()),
            (Self::Bool, Value::Bool(_)) => true,
            (Self::Float { min, max }, Value::Float(v)) => v.is_finite() && v >= min && v <= max,
            (Self::Int { min, max }, Value::Int(v)) => v >= min && v <= max,
            (Self::Range, Value::Range(min, max)) => min.is_finite() && max.is_finite(),
            (Self::Items, Value::Items(_)) => true,
            (Self::Tracks, Value::Text(text)) => parse_tracks(text).is_ok(),
            (Self::Color, Value::Color(_)) => true,
            (Self::Size, Value::Size(w, h)) => {
                w.is_finite() && h.is_finite() && *w >= 0.0 && *h >= 0.0
            }
            (Self::Limit, Value::Size(w, h)) => {
                !w.is_nan() && !h.is_nan() && *w >= 0.0 && *h >= 0.0
            }
            (Self::Shadow, Value::Shadow(s)) => {
                [s.offset.0, s.offset.1, s.blur, s.spread]
                    .iter()
                    .all(|v| v.is_finite())
                    && s.blur >= 0.0
            }
            (Self::Stroke, Value::Stroke(w, _)) => w.is_finite() && *w >= 0.0,
            (Self::Font, Value::Font(f)) => {
                f.size.is_finite() && f.size > 0.0 && (1..=1000).contains(&f.weight)
            }
            _ => false,
        }
    }
}

/// Initial value of a property in a new node (a `const`-friendly [`Value`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Init {
    /// The property is optional and unset: the generated code omits it.
    None,
    /// [`Value::Text`]
    Text(&'static str),
    /// [`Value::Bool`]
    Bool(bool),
    /// [`Value::Float`]
    Float(f64),
    /// [`Value::Int`]
    Int(i64),
    /// [`Value::Range`]
    Range(f64, f64),
    /// [`Value::Items`]
    Items(&'static [&'static str]),
    /// [`Value::Size`]
    Size(f64, f64),
}

impl Init {
    /// The initial value, or `None` for optional properties.
    pub fn to_value(self) -> Option<Value> {
        Some(match self {
            Self::None => return None,
            Self::Text(text) => Value::Text(text.to_owned()),
            Self::Bool(b) => Value::Bool(b),
            Self::Float(v) => Value::Float(v),
            Self::Int(v) => Value::Int(v),
            Self::Range(min, max) => Value::Range(min, max),
            Self::Items(items) => Value::Items(items.iter().map(|s| (*s).to_owned()).collect()),
            Self::Size(w, h) => Value::Size(w, h),
        })
    }
}

/// Static description of one property of a [`crate::WidgetKind`].
#[derive(Clone, Copy, Debug)]
pub struct PropSpec {
    /// Key in [`crate::Node::props`].
    pub key: &'static str,
    /// Label in the properties panel.
    pub label: &'static str,
    /// Type and editor.
    pub kind: PropKind,
    /// Section of the properties panel (`""`: the first, unnamed one).
    pub group: &'static str,
    /// Value in a new node. Properties initialized to [`Init::None`] are
    /// optional: they can be unset, and are then left out of the code.
    pub init: Init,
}

impl PropSpec {
    /// Whether the property can be unset.
    pub fn optional(&self) -> bool {
        self.init == Init::None
    }
}
