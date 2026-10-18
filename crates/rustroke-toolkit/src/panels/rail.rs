//! The icon bars along the window edges: one button per side panel, which
//! shows or hides it.

use rustroke::{Button, Stack, StackAlign, Ui};

use crate::app::{Section, Toolkit};

/// Shows the buttons of `sections`, one over the other.
pub fn show(ui: &mut Ui<'_>, app: &mut Toolkit, sections: &[Section]) {
    let icons = app.icons();
    let mut toggled = None;
    Stack::column()
        .gap(2.0)
        .align(StackAlign::Center)
        .show(ui, |ui| {
            for &section in sections {
                let open = app.sections.is_open(section);
                let icon = match section {
                    Section::Widgets => icons.panel_widgets,
                    Section::Views => icons.panel_views,
                    Section::Properties => icons.panel_properties,
                    Section::Code => icons.panel_code,
                };
                let button = Button::icon_only(icon)
                    .icon_size(20.0)
                    .frame(false)
                    .selected(open)
                    .accessible_label(format!("{} panel", section.title()));
                let hint = if open {
                    format!("Hide {}", section.title())
                } else {
                    format!("Show {}", section.title())
                };
                if ui.add(button).on_hover_text(ui, hint).clicked() {
                    toggled = Some(section);
                }
            }
        });
    if let Some(section) = toggled {
        app.sections.toggle(section);
    }
}
