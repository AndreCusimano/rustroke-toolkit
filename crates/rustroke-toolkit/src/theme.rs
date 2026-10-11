//! The toolkit's themes. The dark one is neutral grey/black (rustroke's own
//! dark theme leans blue).

use rustroke::{Color, Stroke, Style};

/// Neutral dark theme: rustroke's dark style with grey backgrounds and
/// widgets; accent and status colors unchanged.
pub fn dark() -> Style {
    let rgb = Color::from_srgb8;
    let mut style = Style::dark();
    let v = &mut style.visuals;
    let text = rgb(222, 222, 222);
    v.background = rgb(30, 30, 30);
    v.panel_fill = rgb(23, 23, 23);
    v.window_fill = rgb(38, 38, 38);
    v.window_stroke = Stroke::new(1.0, rgb(58, 58, 58));
    v.text_field_fill = rgb(18, 18, 18);
    v.text = text;
    v.weak_text = rgb(150, 150, 150);
    v.on_accent = rgb(20, 20, 20);
    v.stripe = rgb(44, 44, 44).with_alpha(0.6);
    v.handle_fill = text;
    v.handle_stroke = Stroke::new(1.0, rgb(30, 30, 30));
    v.inactive.bg_fill = rgb(46, 46, 46);
    v.inactive.stroke = Stroke::new(1.0, rgb(64, 64, 64));
    v.inactive.fg = text;
    v.hovered.bg_fill = rgb(60, 60, 60);
    v.hovered.stroke = Stroke::new(1.0, rgb(86, 86, 86));
    v.hovered.fg = text;
    v.active.bg_fill = rgb(74, 74, 74);
    v.active.stroke = Stroke::new(1.0, rgb(104, 104, 104));
    v.active.fg = text;
    style
}

/// Light theme (rustroke's).
pub fn light() -> Style {
    Style::light()
}
