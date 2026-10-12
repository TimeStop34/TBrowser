use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::ui::GlobalTheme;

pub struct MainSpace;

impl MainSpace {
    pub fn new() -> Self {
        Self
    }

    pub fn inner(&self, area: Rect) -> Rect {
        if area.width < 2 || area.height < 2 {
            return Rect {
                x: area.x,
                y: area.y,
                width: 0,
                height: 0,
            };
        }
        Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let set = theme.border.set;
        let border_style = Style::default().fg(theme.border.fg).bg(theme.border.bg);

        let right = area.right() - 1;
        let bottom = area.bottom() - 1;

        for x in (area.x + 1)..right {
            buf[(x, area.y)].set_char(set.horizontal).set_style(border_style);
            buf[(x, bottom)].set_char(set.horizontal).set_style(border_style);
        }
        for y in (area.y + 1)..bottom {
            buf[(area.x, y)].set_char(set.vertical).set_style(border_style);
            buf[(right, y)].set_char(set.vertical).set_style(border_style);
        }

        buf[(area.x, area.y)].set_char(set.top_left).set_style(border_style);
        buf[(right, area.y)].set_char(set.top_right).set_style(border_style);
        buf[(area.x, bottom)].set_char(set.bottom_left).set_style(border_style);
        buf[(right, bottom)].set_char(set.bottom_right).set_style(border_style);
    }
}

impl Default for MainSpace {
    fn default() -> Self {
        Self::new()
    }
}