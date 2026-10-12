use ratatui::style::Color;

use crate::ui::style::SelectionStyle;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct SettingsStyle {
    pub item_fg: Color,
    pub item_selected_fg: Color,
    pub value_fg: Color,
    pub description_fg: Color,
    pub variant_selected: SelectionStyle,
}

impl Default for SettingsStyle {
    fn default() -> Self {
        Self {
            item_fg: Color::Gray,
            item_selected_fg: Color::White,
            value_fg: Color::Cyan,
            description_fg: Color::Gray,
            variant_selected: SelectionStyle {
                fg: Color::Black,
                bg: Color::White,
            },
        }
    }
}