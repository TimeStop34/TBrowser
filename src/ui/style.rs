use ratatui::style::Color;

use crate::core::settings::{Settings, Value};

#[derive(Debug, Clone, Copy)]
pub struct BorderSet {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

impl BorderSet {
    pub const SINGLE: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '─',
        vertical: '│',
    };
    pub const DOUBLE: Self = Self {
        top_left: '╔',
        top_right: '╗',
        bottom_left: '╚',
        bottom_right: '╝',
        horizontal: '═',
        vertical: '║',
    };
    pub const ROUNDED: Self = Self {
        top_left: '╭',
        top_right: '╮',
        bottom_left: '╰',
        bottom_right: '╯',
        horizontal: '─',
        vertical: '│',
    };
    pub const THICK: Self = Self {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '━',
        vertical: '┃',
    };
}

#[derive(Debug, Clone, Copy)]
pub struct BorderStyle {
    pub fg: Color,
    pub bg: Color,
    pub set: BorderSet,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Reset,
            set: BorderSet::SINGLE,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TitleStyle {
    pub fg: Color,
    pub bg: Color,
    pub format: String,
}

impl Default for TitleStyle {
    fn default() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Blue,
            format: " {} ".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ButtonStyle {
    pub fg: Color,
    pub bg: Color,
    pub selected_fg: Color,
    pub selected_bg: Color,
    pub format: String,
    pub selected_format: String,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            fg: Color::White,
            bg: Color::Reset,
            selected_fg: Color::Black,
            selected_bg: Color::White,
            format: "[ {} ]".to_string(),
            selected_format: "[ {} ]".to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InputStyle {
    pub fg: Color,
    pub bg: Color,
}

impl Default for InputStyle {
    fn default() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionStyle {
    pub fg: Color,
    pub bg: Color,
}

impl Default for SelectionStyle {
    fn default() -> Self {
        Self {
            fg: Color::Black,
            bg: Color::White,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeaderStyle {
    pub background: Color,
    pub foreground: Color,
}

impl Default for HeaderStyle {
    fn default() -> Self {
        Self {
            background: Color::White,
            foreground: Color::Black,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalTheme {
    pub accent: Color,
    pub selected_tab_color: Color,
    pub url_bar_color: Color,
    pub scrollbar_color: Color,
    pub keybind_hint_fg: Color,
    pub keybind_hint_bg: Color,
    pub keybind_hint_format: String,
    pub keybind_hint_duration_ms: u64,
    pub button: ButtonStyle,
    pub title: TitleStyle,
    pub input: InputStyle,
    pub border: BorderStyle,
    pub selection: SelectionStyle,
    pub header: HeaderStyle,
    pub modal_bg: Color,
    pub modal_fg: Color,
}

impl GlobalTheme {
    pub fn from_settings(s: &Settings) -> Self {
        let accent = get_color(s, "appearance.accent", Color::Cyan);
        let selected_tab_color = get_color(s, "appearance.selected_tab_color", Color::Green);

        let border_set = match get_index(s, "appearance.layout.border_style") {
            0 => BorderSet::SINGLE,
            1 => BorderSet::DOUBLE,
            2 => BorderSet::ROUNDED,
            3 => BorderSet::THICK,
            _ => BorderSet::SINGLE,
        };

        let btn_fmt = get_string(s, "appearance.layout.btn_format", "[ {} ]");
        let title_fmt = get_string(s, "appearance.layout.title_format", " {} ");

        let border_fg = resolve_color_type(
            s,
            "appearance.interface.borders_color_type",
            "appearance.fg_colors.borders_color",
            accent,
        );
        let selected_bg = resolve_color_type(
            s,
            "appearance.interface.selected_btn_bg_type",
            "appearance.bg_colors.selected_btn_bg_color",
            accent,
        );
        let header_bg = resolve_color_type(
            s,
            "appearance.interface.header_bg_type",
            "appearance.bg_colors.header_bg_color",
            accent,
        );
        let selection_bg = resolve_color_type(
            s,
            "appearance.interface.selection_bg_type",
            "appearance.bg_colors.selection_bg_color",
            accent,
        );
        let url_bar_color = resolve_color_type(
            s,
            "appearance.interface.url_bar_color_type",
            "appearance.fg_colors.url_bar_color",
            accent,
        );
        let scrollbar_color = resolve_color_type(
            s,
            "appearance.interface.scrollbar_color_type",
            "appearance.fg_colors.scrollbar_color",
            accent,
        );
        let keybind_hint_bg = resolve_color_type(
            s,
            "appearance.interface.keybind_hint_bg_type",
            "appearance.bg_colors.keybind_hint_bg",
            accent,
        );
        let keybind_hint_format = get_string(s, "appearance.layout.keybind_hint_format", "< {} >");
        let keybind_hint_duration_ms =
            (get_float(s, "appearance.layout.keybind_hint_duration", 1.0).max(0.0) * 1000.0) as u64;

        Self {
            accent,
            selected_tab_color,
            url_bar_color,
            scrollbar_color,
            keybind_hint_fg: Color::Black,
            keybind_hint_bg,
            keybind_hint_format,
            keybind_hint_duration_ms,
            button: ButtonStyle {
                format: btn_fmt.clone(),
                selected_format: btn_fmt,
                selected_fg: Color::Black,
                selected_bg,
                ..Default::default()
            },
            title: TitleStyle {
                bg: accent,
                format: title_fmt,
                ..Default::default()
            },
            input: InputStyle::default(),
            border: BorderStyle {
                fg: border_fg,
                set: border_set,
                ..Default::default()
            },
            selection: SelectionStyle {
                fg: Color::Black,
                bg: selection_bg,
            },
            header: HeaderStyle {
                background: header_bg,
                foreground: Color::Black,
            },
            modal_bg: Color::Reset,
            modal_fg: Color::White,
        }
    }
}

impl Default for GlobalTheme {
    fn default() -> Self {
        Self {
            accent: Color::Cyan,
            selected_tab_color: Color::Green,
            url_bar_color: Color::Cyan,
            scrollbar_color: Color::White,
            keybind_hint_fg: Color::Black,
            keybind_hint_bg: Color::White,
            keybind_hint_format: "< {} >".to_string(),
            keybind_hint_duration_ms: 1000,
            button: ButtonStyle::default(),
            title: TitleStyle::default(),
            input: InputStyle::default(),
            border: BorderStyle::default(),
            selection: SelectionStyle::default(),
            header: HeaderStyle::default(),
            modal_bg: Color::Reset,
            modal_fg: Color::White,
        }
    }
}

fn get_color(s: &Settings, key: &str, fallback: Color) -> Color {
    match s.get(key) {
        Some(Value::EnumIndex(i)) => color_from_index(*i),
        _ => fallback,
    }
}

fn get_index(s: &Settings, key: &str) -> usize {
    match s.get(key) {
        Some(Value::EnumIndex(i)) => *i,
        _ => 0,
    }
}

fn get_string(s: &Settings, key: &str, fallback: &str) -> String {
    match s.get(key) {
        Some(Value::Str(v)) => v.clone(),
        _ => fallback.to_string(),
    }
}

fn get_float(s: &Settings, key: &str, fallback: f64) -> f64 {
    match s.get(key) {
        Some(Value::Float(f)) => *f,
        Some(Value::Int(i)) => *i as f64,
        _ => fallback,
    }
}

fn resolve_color_type(s: &Settings, type_key: &str, custom_key: &str, accent: Color) -> Color {
    match get_index(s, type_key) {
        0 => Color::White,
        1 => accent,
        2 => get_color(s, custom_key, Color::White),
        _ => Color::White,
    }
}

fn color_from_index(i: usize) -> Color {
    match i {
        0 => Color::Blue,
        1 => Color::Green,
        2 => Color::Magenta,
        3 => Color::Cyan,
        4 => Color::Yellow,
        5 => Color::Red,
        _ => Color::Cyan,
    }
}