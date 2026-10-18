//! The toolkit's SVG icons (repository folder `icons/`), compiled into the
//! binary and loaded once as two-tone icons: black parts take the text
//! color, `#1E6FFF` parts the theme's accent.

use rtk_document::{Category, Node, WidgetKind};
use rustroke::{Fonts, IconId};

macro_rules! icons {
    ($($name:ident),* $(,)?) => {
        /// Every icon, by file name.
        #[derive(Clone, Copy, Debug)]
        #[allow(missing_docs)]
        pub struct Icons {
            $(pub $name: IconId,)*
        }

        impl Icons {
            /// Loads every icon into `fonts`.
            pub fn load(fonts: &mut Fonts) -> Self {
                Self {
                    $($name: fonts
                        .add_svg_icon(include_bytes!(concat!(
                            "../../../icons/",
                            stringify!($name),
                            ".svg"
                        )))
                        .unwrap_or_else(|err| {
                            panic!("icons/{}.svg: {err}", stringify!($name))
                        }),)*
                }
            }
        }
    };
}

// `app_icon.svg` is full color and only meant for the operating system.
icons! {
    code_block, code_comment, code_error, code_format,
    edit_copy, edit_cut, edit_delete, edit_drag, edit_duplicate, edit_move_down,
    edit_move_up, edit_paste, edit_redo, edit_undo,
    layout_collapsing, layout_end_row, layout_flex, layout_flex_grid, layout_flex_item,
    layout_grid, layout_horizontal, layout_indent, layout_scroll_area, layout_vertical,
    layout_vertical_centered,
    menu_buttons, menu_code, menu_containers, menu_edit, menu_input, menu_text, menu_values,
    mode_design, mode_preview,
    panel_code, panel_properties, panel_views, panel_widgets,
    project, project_new, project_open, project_open_recent, project_save, project_save_as,
    prop_binding, prop_conflict, prop_field_written,
    status_info, status_saved, status_unsaved, status_warning,
    theme_dark, theme_light,
    view_new, view_rename, view_window,
    widget_button, widget_checkbox, widget_color_picker, widget_combo_box,
    widget_date_picker, widget_drag_value, widget_heading, widget_hyperlink, widget_icon,
    widget_image, widget_label, widget_progress_bar, widget_radio, widget_search_field,
    widget_selectable, widget_separator, widget_slider, widget_space, widget_spinner,
    widget_tab_bar, widget_table, widget_text_area, widget_text_field, widget_time_picker,
    window_fixed, window_resizable, window_resize_handle, window_size_preset,
}

impl Icons {
    /// The icon of a kind of node, as in the widget menus.
    pub fn kind(&self, kind: WidgetKind) -> IconId {
        use WidgetKind as K;
        match kind {
            K::Label => self.widget_label,
            K::Heading => self.widget_heading,
            K::Hyperlink => self.widget_hyperlink,
            K::Separator => self.widget_separator,
            K::Space => self.widget_space,
            K::Button => self.widget_button,
            K::Checkbox => self.widget_checkbox,
            K::RadioValue => self.widget_radio,
            K::SelectableValue => self.widget_selectable,
            K::ComboBox => self.widget_combo_box,
            K::Slider => self.widget_slider,
            K::DragValue => self.widget_drag_value,
            K::ProgressBar => self.widget_progress_bar,
            K::Spinner => self.widget_spinner,
            K::TextEdit => self.widget_text_field,
            K::SearchField => self.widget_search_field,
            K::TimePicker => self.widget_time_picker,
            K::ColorPicker => self.widget_color_picker,
            K::DatePicker => self.widget_date_picker,
            // Until `layout_stack`, `layout_stack_item`, `layout_grid_cell`,
            // `layout_panel`, `layout_central_panel` and the chart icons
            // are drawn.
            K::Stack => self.layout_flex,
            K::StackItem => self.layout_flex_item,
            K::GridCell => self.layout_flex_grid,
            K::Panel => self.panel_views,
            K::CentralPanel => self.view_window,
            K::Grid => self.layout_grid,
            K::EndRow => self.layout_end_row,
            K::Collapsing => self.layout_collapsing,
            K::ScrollArea => self.layout_scroll_area,
            K::Plot => self.menu_values,
            K::PlotLine => self.widget_slider,
            K::PlotPoints => self.widget_radio,
            K::PlotHLine | K::PlotVLine => self.widget_separator,
            K::PieChart => self.widget_spinner,
            K::RawCode => self.code_block,
        }
    }

    /// The icon of a node in the component tree: like its kind, but
    /// multi-line text fields, stacks (by direction) and comment-only code
    /// have their own.
    pub fn node(&self, node: &Node) -> IconId {
        match node.kind {
            WidgetKind::TextEdit if node.flag("multiline") => self.widget_text_area,
            WidgetKind::Stack => match node.text("direction") {
                "row" => self.layout_horizontal,
                "column" if node.text("align") == "Center" => self.layout_vertical_centered,
                "column" => self.layout_vertical,
                _ => self.layout_flex,
            },
            WidgetKind::RawCode
                if node.text("code").lines().all(|l| {
                    let l = l.trim_start();
                    l.is_empty() || l.starts_with("//")
                }) =>
            {
                self.code_comment
            }
            kind => self.kind(kind),
        }
    }

    /// The icon of a widget menu.
    pub fn category(&self, category: Category) -> IconId {
        match category {
            Category::Text => self.menu_text,
            Category::Buttons => self.menu_buttons,
            Category::Values => self.menu_values,
            Category::Input => self.menu_input,
            Category::Containers => self.menu_containers,
            Category::Charts => self.menu_values,
            Category::Code => self.menu_code,
        }
    }
}
