use ratatui::style::Color;

use crate::core::renderer::theme::RendererTheme;
use crate::core::settings::{always_active, DefaultValue, ParamDef, ParamKind};
use crate::core::settings::{ItemDef, NodeDef, Settings, Value};

pub const TMKUP_COLORS: &[&str] = &[
    "Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "Gray", "White",
];

fn color_from_index(i: usize) -> Color {
    match i {
        0 => Color::Black,
        1 => Color::Red,
        2 => Color::Green,
        3 => Color::Yellow,
        4 => Color::Blue,
        5 => Color::Magenta,
        6 => Color::Cyan,
        7 => Color::Gray,
        8 => Color::White,
        _ => Color::White,
    }
}

#[derive(Debug, Clone)]
pub struct TmkupTheme {
    pub heading_fg: Color,
    pub link_fg: Color,
    pub code_fg: Color,
    pub border_fg: Color,
    pub info_fg: Color,
    pub warning_fg: Color,
    pub success_fg: Color,
    pub error_fg: Color,
    pub spoiler_fg: Color,
    pub quote_fg: Color,
}

impl RendererTheme for TmkupTheme {}

impl TmkupTheme {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            heading_fg: get_color(s, "heading_fg", Color::Cyan),
            link_fg: get_color(s, "link_fg", Color::Blue),
            code_fg: get_color(s, "code_fg", Color::Yellow),
            border_fg: get_color(s, "border_fg", Color::White),
            info_fg: get_color(s, "info_fg", Color::Blue),
            warning_fg: get_color(s, "warning_fg", Color::Yellow),
            success_fg: get_color(s, "success_fg", Color::Green),
            error_fg: get_color(s, "error_fg", Color::Red),
            spoiler_fg: get_color(s, "spoiler_fg", Color::White),
            quote_fg: get_color(s, "quote_fg", Color::White),
        }
    }
}

impl Default for TmkupTheme {
    fn default() -> Self {
        Self {
            heading_fg: Color::Cyan,
            link_fg: Color::Blue,
            code_fg: Color::Yellow,
            border_fg: Color::White,
            info_fg: Color::Blue,
            warning_fg: Color::Yellow,
            success_fg: Color::Green,
            error_fg: Color::Red,
            spoiler_fg: Color::White,
            quote_fg: Color::White,
        }
    }
}

fn get_color(s: &Settings, short: &str, fallback: Color) -> Color {
    let full = format!("{}.{}", NODE_TMKUP_THEME.key, short);
    match s.get(&full) {
        Some(Value::EnumIndex(i)) => color_from_index(*i),
        _ => fallback,
    }
}

macro_rules! theme_color_param {
    ($name:ident, $key:literal, $label:literal, $desc:literal, $default:expr) => {
        static $name: ParamDef = ParamDef {
            key: $key,
            label: $label,
            description: $desc,
            kind: ParamKind::Color(TMKUP_COLORS),
            active: always_active,
            default: DefaultValue::EnumIndex($default),
        };
    };
}

theme_color_param!(P_HEADING_FG, "heading_fg", "Heading color", "Foreground color of headings.", 6);
theme_color_param!(P_LINK_FG, "link_fg", "Link color", "Foreground color of links.", 4);
theme_color_param!(P_CODE_FG, "code_fg", "Code color", "Foreground color of inline code.", 3);
theme_color_param!(P_BORDER_FG, "border_fg", "Border color", "Color of block borders.", 8);
theme_color_param!(P_INFO_FG, "info_fg", "Info color", "Color of info blocks.", 4);
theme_color_param!(P_WARNING_FG, "warning_fg", "Warning color", "Color of warning blocks.", 3);
theme_color_param!(P_SUCCESS_FG, "success_fg", "Success color", "Color of success blocks.", 2);
theme_color_param!(P_ERROR_FG, "error_fg", "Error color", "Color of error blocks.", 1);
theme_color_param!(P_SPOILER_FG, "spoiler_fg", "Spoiler color", "Foreground color of spoilers.", 8);
theme_color_param!(P_QUOTE_FG, "quote_fg", "Quote color", "Foreground color of quotes.", 8);

const TMKUP_THEME_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_HEADING_FG),
    ItemDef::Param(&P_LINK_FG),
    ItemDef::Param(&P_CODE_FG),
    ItemDef::Param(&P_BORDER_FG),
    ItemDef::Param(&P_INFO_FG),
    ItemDef::Param(&P_WARNING_FG),
    ItemDef::Param(&P_SUCCESS_FG),
    ItemDef::Param(&P_ERROR_FG),
    ItemDef::Param(&P_SPOILER_FG),
    ItemDef::Param(&P_QUOTE_FG),
];

pub static NODE_TMKUP_THEME: NodeDef = NodeDef {
    key: "render_themes.tmkup",
    label: "TMkUp",
    active: always_active,
    children: TMKUP_THEME_CHILDREN,
};