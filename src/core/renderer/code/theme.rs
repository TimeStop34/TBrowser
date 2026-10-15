use ratatui::style::Color;

use crate::core::settings::colors::{color_from_index, COLOR_NAMES};
use crate::core::settings::{always_active, DefaultValue, ParamDef, ParamKind};
use crate::core::settings::{ItemDef, NodeDef, Settings, Value};

pub const CODE_NODE_KEY: &str = "render_themes.code";

#[derive(Debug, Clone)]
pub struct CodeTheme {
    pub keyword: Color,
    pub string: Color,
    pub comment: Color,
    pub number: Color,
    pub function: Color,
    pub type_: Color,
    pub constant: Color,
    pub operator: Color,
    pub punctuation: Color,
    pub default_fg: Color,
}

impl CodeTheme {
    pub fn from_settings(s: &Settings) -> Self {
        Self {
            keyword: get_color(s, "keyword", Color::Magenta),
            string: get_color(s, "string", Color::Green),
            comment: get_color(s, "comment", Color::DarkGray),
            number: get_color(s, "number", Color::Yellow),
            function: get_color(s, "function", Color::Cyan),
            type_: get_color(s, "type", Color::Yellow),
            constant: get_color(s, "constant", Color::Red),
            operator: get_color(s, "operator", Color::White),
            punctuation: get_color(s, "punctuation", Color::Gray),
            default_fg: get_color(s, "default_fg", Color::White),
        }
    }
}

fn get_color(s: &Settings, short: &str, fallback: Color) -> Color {
    let full = format!("{}.{}", CODE_NODE_KEY, short);
    match s.get(&full) {
        Some(Value::EnumIndex(i)) => color_from_index(*i),
        _ => fallback,
    }
}

macro_rules! code_color_param {
    ($name:ident, $key:literal, $label:literal, $default:expr) => {
        static $name: ParamDef = ParamDef {
            key: $key,
            label: $label,
            description: "Token color",
            kind: ParamKind::Color(COLOR_NAMES),
            active: always_active,
            default: DefaultValue::EnumIndex($default),
        };
    };
}

code_color_param!(P_KEYWORD, "keyword", "Keyword", 5);
code_color_param!(P_STRING, "string", "String", 2);
code_color_param!(P_COMMENT, "comment", "Comment", 8);
code_color_param!(P_NUMBER, "number", "Number", 3);
code_color_param!(P_FUNCTION, "function", "Function", 6);
code_color_param!(P_TYPE, "type", "Type", 3);
code_color_param!(P_CONSTANT, "constant", "Constant", 1);
code_color_param!(P_OPERATOR, "operator", "Operator", 15);
code_color_param!(P_PUNCTUATION, "punctuation", "Punctuation", 7);
code_color_param!(P_DEFAULT_FG, "default_fg", "Default", 15);

const CODE_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_KEYWORD),
    ItemDef::Param(&P_STRING),
    ItemDef::Param(&P_COMMENT),
    ItemDef::Param(&P_NUMBER),
    ItemDef::Param(&P_FUNCTION),
    ItemDef::Param(&P_TYPE),
    ItemDef::Param(&P_CONSTANT),
    ItemDef::Param(&P_OPERATOR),
    ItemDef::Param(&P_PUNCTUATION),
    ItemDef::Param(&P_DEFAULT_FG),
];

pub static NODE_CODE: NodeDef = NodeDef {
    key: CODE_NODE_KEY,
    label: "Code",
    active: always_active,
    children: CODE_CHILDREN,
};

/// Приводит алиас языка к каноническому имени.
/// Пустая строка — язык не поддерживается (→ fallback, без подсветки).
pub fn canonical_lang(lang: &str) -> &'static str {
    match lang.to_ascii_lowercase().as_str() {
        "python" | "py" => "python",
        "rust" | "rs" => "rust",
        "c" | "h" => "c",
        "cpp" | "c++" | "cxx" | "cc" | "hpp" | "hxx" | "h++" => "cpp",
        "bash" | "sh" | "shell" => "bash",
        "json" => "json",
        "lua" => "lua",
        "markdown" | "md" | "mkd" | "mkdn" | "mdown" | "mkdown" => "markdown",
        _ => "",
    }
}