use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

use crate::ui::GlobalTheme;

pub struct KeybindHint {
    text: String,
    generation: u64,
}

impl KeybindHint {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            generation: 0,
        }
    }

    pub fn show(&mut self, text: impl Into<String>) -> u64 {
        self.text = text.into();
        self.generation += 1;
        self.generation
    }

    pub fn hide(&mut self, token: u64) {
        if self.generation == token {
            self.text.clear();
        }
    }

    pub fn is_visible(&self) -> bool {
        !self.text.is_empty()
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Заливаем всю строку пробелами с полностью сброшенным стилем
        let reset = Style::default().fg(Color::Reset).bg(Color::Reset);
        let blank = " ".repeat(area.width as usize);
        buf.set_string(area.x, area.y, &blank, reset);

        if self.text.is_empty() {
            return;
        }

        let formatted = theme.keybind_hint_format.replace("{}", &self.text);
        let len = formatted.chars().count() as u16;
        let style = Style::default()
            .fg(theme.keybind_hint_fg)
            .bg(theme.keybind_hint_bg);

        if len > area.width {
            let truncated: String = formatted.chars().take(area.width as usize).collect();
            buf.set_string(area.x, area.y, &truncated, style);
        } else {
            let x = area.x + area.width.saturating_sub(len) / 2;
            buf.set_string(x, area.y, &formatted, style);
        }
    }
}

impl Default for KeybindHint {
    fn default() -> Self {
        Self::new()
    }
}