use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::ui::GlobalTheme;

#[derive(Debug, Clone)]
pub struct Header<'a> {
    left: &'a str,
    right: &'a str,
    padding: u16,
}

impl<'a> Header<'a> {
    pub fn new(left: &'a str, right: &'a str) -> Self {
        Self {
            left,
            right,
            padding: 2,
        }
    }

    #[allow(unused)]
    pub fn padding(mut self, padding: u16) -> Self {
        self.padding = padding;
        self
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let base_style = Style::default().bg(theme.header.background);
        buf.set_style(area, base_style);

        let padding = self.padding.min(area.width / 2);
        let inner_x = area.x + padding;
        let inner_width = area.width.saturating_sub(padding * 2);

        if inner_width == 0 {
            return;
        }

        let text_style = Style::default().fg(theme.header.foreground);

        let left_len = self.left.chars().count() as u16;
        let left_display = truncate_to(self.left, inner_width);
        buf.set_string(inner_x, area.y, &left_display, text_style);

        let right_len = self.right.chars().count() as u16;
        let right_x = inner_x + inner_width;
        let right_start = right_x.saturating_sub(right_len);

        let min_right_x = inner_x + left_len;
        let right_start = right_start.max(min_right_x);

        let available = (right_x - right_start) as usize;
        let right_display = truncate_to(self.right, available as u16);
        buf.set_string(right_start, area.y, &right_display, text_style);
    }
}

fn truncate_to(s: &str, max_chars: u16) -> String {
    s.chars().take(max_chars as usize).collect()
}