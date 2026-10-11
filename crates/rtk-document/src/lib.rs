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
    CONTENT_STYLE, Category, FLEX_ALIGN, FLEX_DIRECTIONS, FLEX_JUSTIFY, KindSpec, LAYOUT,
    PANEL_SIDES, SCROLL_DIRECTIONS, SEPARATOR_ORIENTATIONS, STYLE, WidgetKind,
    content_style_target,
};
pub use model::{BindingConflict, Field, Node, NodeId, Project, RawField, View, WindowSettings};
pub use ops::EditError;
pub use sample::sample_project;
pub use value::{
    FieldType, Font, FontFamily, Init, PropKind, PropSpec, Shadow, Track, Value, format_tracks,
    parse_tracks,
};
