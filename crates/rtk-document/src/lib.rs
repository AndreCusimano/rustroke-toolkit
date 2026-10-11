//! UI document model for Rustroke Toolkit.
//!
//! A [`Project`] holds [`View`]s; each view is a tree of [`Node`]s (widgets,
//! containers, free code) with typed properties described by
//! [`WidgetKind::spec`]. The view's Rust code is generated from the tree
//! ([`codegen`]) and read back into it ([`parser`]).

mod builders;
pub mod codegen;
mod kind;
mod model;
pub mod names;
mod ops;
pub mod parser;
pub mod project;
mod sample;
mod value;

pub use kind::{
    CONTENT_STYLE, CORNERS, Category, JUSTIFY, KindSpec, LAYOUT, LINE_STYLES, MARKER_SHAPES,
    PANEL_SIDES, SCROLL_DIRECTIONS, SEPARATOR_ORIENTATIONS, STACK_ALIGN, STACK_DIRECTIONS, STYLE,
    WidgetKind, content_style_target,
};
pub use model::{BindingConflict, Field, Node, NodeId, Project, RawField, View, WindowSettings};
pub use ops::EditError;
pub use sample::sample_project;
pub use value::{
    FieldType, Font, FontFamily, Init, PropKind, PropSpec, Shadow, Sizing, Track, Value,
    format_tracks, parse_tracks,
};
