use ratatui::style::Color;

use crate::core::renderer::theme::RendererTheme;
use crate::core::settings::colors::{color_from_index, COLOR_NAMES};
use crate::core::settings::{always_active, DefaultValue, ParamDef, ParamKind};
use crate::core::settings::{ItemDef, NodeDef, Settings, Value};

pub const BORDER_KINDS: &[&str] = &["Left", "Single", "Double", "Rounded", "Thick"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockBorder {
    Left,
    Single,
    Double,
    Rounded,
    Thick,
}

impl BlockBorder {
    pub fn from_index(i: usize) -> Self {
        match i {
            0 => BlockBorder::Left,
            1 => BlockBorder::Single,
            2 => BlockBorder::Double,
            3 => BlockBorder::Rounded,
            4 => BlockBorder::Thick,
            _ => BlockBorder::Rounded,
        }
    }

    /// (top_left, top_right, bottom_left, bottom_right, horizontal, vertical)
    pub fn chars(&self) -> (char, char, char, char, char, char) {
        match self {
            BlockBorder::Left => (' ', ' ', ' ', ' ', ' ', '│'),
            BlockBorder::Single => ('┌', '┐', '└', '┘', '─', '│'),
            BlockBorder::Double => ('╔', '╗', '╚', '╝', '═', '║'),
            BlockBorder::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            BlockBorder::Thick => ('┏', '┓', '┗', '┛', '━', '┃'),
        }
    }

    pub fn is_left_only(&self) -> bool {
        matches!(self, BlockBorder::Left)
    }

    pub fn table_chars(&self) -> TableBorderChars {
        match self {
            BlockBorder::Left | BlockBorder::Single => TableBorderChars {
                top_left: '┌',
                top_mid: '┬',
                top_right: '┐',
                mid_left: '├',
                cross: '┼',
                mid_right: '┤',
                bot_left: '└',
                bot_mid: '┴',
                bot_right: '┘',
                horizontal: '─',
                vertical: '│',
            },
            BlockBorder::Double => TableBorderChars {
                top_left: '╔',
                top_mid: '╦',
                top_right: '╗',
                mid_left: '╠',
                cross: '╬',
                mid_right: '╣',
                bot_left: '╚',
                bot_mid: '╩',
                bot_right: '╝',
                horizontal: '═',
                vertical: '│',
            },
            BlockBorder::Rounded => TableBorderChars {
                top_left: '╭',
                top_mid: '┬',
                top_right: '╮',
                mid_left: '├',
                cross: '┼',
                mid_right: '┤',
                bot_left: '╰',
                bot_mid: '┴',
                bot_right: '╯',
                horizontal: '─',
                vertical: '│',
            },
            BlockBorder::Thick => TableBorderChars {
                top_left: '┏',
                top_mid: '┳',
                top_right: '┓',
                mid_left: '┣',
                cross: '╋',
                mid_right: '┫',
                bot_left: '┗',
                bot_mid: '┻',
                bot_right: '┛',
                horizontal: '━',
                vertical: '┃',
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TableBorderChars {
    pub top_left: char,
    pub top_mid: char,
    pub top_right: char,
    pub mid_left: char,
    pub cross: char,
    pub mid_right: char,
    pub bot_left: char,
    pub bot_mid: char,
    pub bot_right: char,
    pub horizontal: char,
    pub vertical: char,
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

    pub label_format: String,
    pub spoiler_line_format: String,

    pub info_border: BlockBorder,
    pub spoiler_border: BlockBorder,
    pub quote_border: BlockBorder,
    pub form_border: BlockBorder,
    pub code_border: BlockBorder,
    pub table_outer_border: BlockBorder,
    pub table_inner_border: BlockBorder,
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

            label_format: get_string(s, "label_format", " {} "),
            spoiler_line_format: get_string(s, "spoiler_line_format", "[{marker}] {title}"),

            info_border: get_border(s, "info_border", BlockBorder::Rounded),
            spoiler_border: get_border(s, "spoiler_border", BlockBorder::Rounded),
            quote_border: get_border(s, "quote_border", BlockBorder::Left),
            form_border: get_border(s, "form_border", BlockBorder::Rounded),
            code_border: get_border(s, "code_border", BlockBorder::Rounded),
            table_outer_border: get_border(s, "table_outer_border", BlockBorder::Double),
            table_inner_border: get_border(s, "table_inner_border", BlockBorder::Double),
        }
    }

    pub fn format_label(&self, label: &str) -> String {
        self.label_format.replace("{}", label)
    }

    pub fn format_spoiler_line(&self, marker: &str, title: &str) -> String {
        self.spoiler_line_format
            .replace("{marker}", marker)
            .replace("{title}", title)
    }
}

impl Default for TmkupTheme {
    fn default() -> Self {
        Self::from_settings(&Settings::new())
    }
}

fn get_color(s: &Settings, short: &str, fallback: Color) -> Color {
    let full = format!("{}.{}", NODE_TMKUP_THEME.key, short);
    match s.get(&full) {
        Some(Value::EnumIndex(i)) => color_from_index(*i),
        _ => fallback,
    }
}

fn get_string(s: &Settings, short: &str, fallback: &str) -> String {
    let full = format!("{}.{}", NODE_TMKUP_THEME.key, short);
    match s.get(&full) {
        Some(Value::Str(v)) => v.clone(),
        _ => fallback.to_string(),
    }
}

fn get_border(s: &Settings, short: &str, fallback: BlockBorder) -> BlockBorder {
    let full = format!("{}.{}", NODE_TMKUP_THEME.key, short);
    match s.get(&full) {
        Some(Value::EnumIndex(i)) => BlockBorder::from_index(*i),
        _ => fallback,
    }
}

macro_rules! theme_color_param {
    ($name:ident, $key:literal, $label:literal, $desc:literal, $default:expr) => {
        static $name: ParamDef = ParamDef {
            key: $key,
            label: $label,
            description: $desc,
            kind: ParamKind::Color(COLOR_NAMES),
            active: always_active,
            default: DefaultValue::EnumIndex($default),
        };
    };
}

macro_rules! theme_border_param {
    ($name:ident, $key:literal, $label:literal, $desc:literal, $default:expr) => {
        static $name: ParamDef = ParamDef {
            key: $key,
            label: $label,
            description: $desc,
            kind: ParamKind::Enum(BORDER_KINDS),
            active: always_active,
            default: DefaultValue::EnumIndex($default),
        };
    };
}

theme_color_param!(P_HEADING_FG, "heading_fg", "Heading color", "Foreground color of headings.", 6);
theme_color_param!(P_LINK_FG, "link_fg", "Link color", "Foreground color of links.", 4);
theme_color_param!(P_CODE_FG, "code_fg", "Code color", "Foreground color of inline code.", 3);
theme_color_param!(P_BORDER_FG, "border_fg", "Border color", "Color of block borders (form, code).", 15);
theme_color_param!(P_INFO_FG, "info_fg", "Info color", "Color of info blocks.", 4);
theme_color_param!(P_WARNING_FG, "warning_fg", "Warning color", "Color of warning blocks.", 3);
theme_color_param!(P_SUCCESS_FG, "success_fg", "Success color", "Color of success blocks.", 2);
theme_color_param!(P_ERROR_FG, "error_fg", "Error color", "Color of error blocks.", 1);
theme_color_param!(P_SPOILER_FG, "spoiler_fg", "Spoiler color", "Foreground color of spoilers.", 15);
theme_color_param!(P_QUOTE_FG, "quote_fg", "Quote color", "Foreground color of quotes.", 15);

static P_LABEL_FORMAT: ParamDef = ParamDef {
    key: "label_format",
    label: "Label format",
    description: "Template for block labels (SPOILER, FORM, INFO, code language).\nMust contain {} — expands into the label.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str(" {} "),
};

static P_SPOILER_LINE_FORMAT: ParamDef = ParamDef {
    key: "spoiler_line_format",
    label: "Spoiler line format",
    description: "Template for spoiler header line.\n{marker} — marker (\u{25B6}/\u{25BC}), {title} — title.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str("[{marker}] {title}"),
};

theme_border_param!(P_INFO_BORDER, "info_border", "Info border", "Border style for info blocks.", 3);
theme_border_param!(P_SPOILER_BORDER, "spoiler_border", "Spoiler border", "Border style for spoilers.", 3);
theme_border_param!(P_QUOTE_BORDER, "quote_border", "Quote border", "Border style for quotes.", 0);
theme_border_param!(P_FORM_BORDER, "form_border", "Form border", "Border style for forms.", 3);
theme_border_param!(P_CODE_BORDER, "code_border", "Code border", "Border style for code blocks.", 3);
theme_border_param!(P_TABLE_OUTER_BORDER, "table_outer_border", "Table outer border", "Border style for the outer frame of tables.", 2);
theme_border_param!(P_TABLE_INNER_BORDER, "table_inner_border", "Table inner border", "Border style for table row separators.", 2);

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
    ItemDef::Param(&P_LABEL_FORMAT),
    ItemDef::Param(&P_SPOILER_LINE_FORMAT),
    ItemDef::Param(&P_INFO_BORDER),
    ItemDef::Param(&P_SPOILER_BORDER),
    ItemDef::Param(&P_QUOTE_BORDER),
    ItemDef::Param(&P_FORM_BORDER),
    ItemDef::Param(&P_CODE_BORDER),
    ItemDef::Param(&P_TABLE_OUTER_BORDER),
    ItemDef::Param(&P_TABLE_INNER_BORDER),
];

pub static NODE_TMKUP_THEME: NodeDef = NodeDef {
    key: "render_themes.tmkup",
    label: "TMkUp",
    active: always_active,
    children: TMKUP_THEME_CHILDREN,
};