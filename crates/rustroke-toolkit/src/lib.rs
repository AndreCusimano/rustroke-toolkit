//! Rustroke Toolkit: a visual designer for rustroke views. The binary only
//! opens the window; the app lives here so tests can drive it.

pub mod app;
mod highlight;
pub mod history;
pub mod icons;
mod panels;
mod project_io;
mod theme;

pub use app::Toolkit;
