use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
};

use crate::ui::style::{BorderStyle, TitleStyle};

pub fn compute_area(screen: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(screen.width);
    let h = height.min(screen.height);
    let x = screen.x + screen.width.saturating_sub(w) / 2;
    let y = screen.y + screen.height.saturating_sub(h) / 2;
    Rect {
        x,
        y,
        width: w,
        height: h,
    }
}

pub fn inner_area(area: Rect) -> Rect {
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

pub fn fill_background(area: Rect, buf: &mut Buffer, style: Style) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].set_char(' ').set_style(style);
        }
    }
}

pub fn render_border(
    area: Rect,
    buf: &mut Buffer,
    border: &BorderStyle,
    title: Option<(&str, &TitleStyle)>,
) {
    if area.width < 2 || area.height < 2 {
        return;
    }

    let set = border.set;
    let style = Style::default().fg(border.fg).bg(border.bg);
    let right = area.right() - 1;
    let bottom = area.bottom() - 1;

    for x in (area.x + 1)..right {
        buf[(x, area.y)].set_char(set.horizontal).set_style(style);
        buf[(x, bottom)].set_char(set.horizontal).set_style(style);
    }
    for y in (area.y + 1)..bottom {
        buf[(area.x, y)].set_char(set.vertical).set_style(style);
        buf[(right, y)].set_char(set.vertical).set_style(style);
    }
    buf[(area.x, area.y)].set_char(set.top_left).set_style(style);
    buf[(right, area.y)].set_char(set.top_right).set_style(style);
    buf[(area.x, bottom)].set_char(set.bottom_left).set_style(style);
    buf[(right, bottom)].set_char(set.bottom_right).set_style(style);

    if let Some((title_text, title_style)) = title {
        if title_text.is_empty() {
            return;
        }
        let formatted = title_style.format.replace("{}", title_text);
        let len = formatted.chars().count() as u16;
        if len + 2 <= area.width {
            let tx = area.x + area.width.saturating_sub(len) / 2;
            let ts = Style::default().fg(title_style.fg).bg(title_style.bg);
            buf.set_string(tx, area.y, &formatted, ts);
        }
    }
}