use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
};

use crate::ui::style::BorderSet;
use crate::ui::GlobalTheme;

pub struct Scrollbar {
    pub total: u16,
    pub visible: u16,
    pub scroll: u16,
}

impl Scrollbar {
    pub fn new(total: u16, visible: u16, scroll: u16) -> Self {
        Self {
            total,
            visible,
            scroll,
        }
    }

    /// Стоит ли вообще рисовать скроллбар — весь контент влез?
    pub fn needed(total: u16, visible: u16) -> bool {
        total > visible && visible > 0
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        if !Self::needed(self.total, self.visible) {
            return;
        }

        let track_h = area.height;
        let thumb_h = (((track_h as u32) * (self.visible as u32)) / (self.total as u32))
            .max(1)
            .min(track_h as u32) as u16;

        let max_scroll = self.total.saturating_sub(self.visible);
        let max_thumb_pos = track_h.saturating_sub(thumb_h);
        let thumb_pos = if max_scroll == 0 {
            0
        } else {
            (((max_thumb_pos as u32) * (self.scroll as u32)) / (max_scroll as u32)) as u16
        };

        let track_ch = BorderSet::SINGLE.vertical; // │
        let thumb_ch = BorderSet::THICK.vertical; // ┃

        let track_style = Style::default()
            .fg(theme.modal_fg)
            .bg(theme.modal_bg);
        let thumb_style = Style::default()
            .fg(theme.scrollbar_color)
            .bg(theme.modal_bg);

        for y in 0..track_h {
            let ch = if y >= thumb_pos && y < thumb_pos.saturating_add(thumb_h) {
                thumb_ch
            } else {
                track_ch
            };
            let style = if y >= thumb_pos && y < thumb_pos.saturating_add(thumb_h) {
                thumb_style
            } else {
                track_style
            };
            buf[(area.x, area.y + y)].set_char(ch).set_style(style);
        }
    }
}