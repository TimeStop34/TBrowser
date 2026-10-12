use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Text},
    widgets::Widget,
};

const PADDING_H: u16 = 1;

pub struct Content {
    pub text: Text<'static>,
    pub scroll: u16,
    pub selected: u16,
    pub style: Style,
    pub wrap: bool,
    selected_style: Cell<Style>,
    last_viewport: Cell<Option<Rect>>,
}

impl Content {
    pub fn new(text: Text<'static>) -> Self {
        Self {
            text,
            scroll: 0,
            selected: 0,
            style: Style::default(),
            wrap: true,
            selected_style: Cell::new(Style::default()),
            last_viewport: Cell::new(None),
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn selected_style(&self, style: Style) -> &Self {
        self.selected_style.set(style);
        self
    }

    pub fn set_text(&mut self, text: Text<'static>) {
        self.text = text;
        self.scroll = 0;
        self.selected = 0;
    }

        pub fn up(&mut self) -> bool {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    pub fn down(&mut self) -> bool {
        let max_line = self.text.lines.len().saturating_sub(1) as u16;
        if self.selected < max_line {
            self.selected += 1;
            self.ensure_visible();
            true
        } else {
            false
        }
    }

    pub fn page_up(&mut self) {
        let page = self.visible_source_lines();
        self.selected = self.selected.saturating_sub(page);
        self.ensure_visible();
    }

    pub fn page_down(&mut self) {
        let page = self.visible_source_lines();
        let max_line = self.text.lines.len().saturating_sub(1) as u16;
        self.selected = self.selected.saturating_add(page).min(max_line);
        self.ensure_visible();
    }

    pub fn scroll_to(&mut self, line: u16) {
        let max_line = self.text.lines.len().saturating_sub(1) as u16;
        self.selected = line.min(max_line);
        self.ensure_visible();
    }

    fn content_area(&self, area: Rect) -> Rect {
        let pad = PADDING_H.min(area.width / 2);
        Rect {
            x: area.x + pad,
            y: area.y,
            width: area.width.saturating_sub(pad * 2),
            height: area.height,
        }
    }

    fn ensure_visible(&mut self) {
        let Some(area) = self.last_viewport.get() else {
            return;
        };
        let inner = self.content_area(area);
        if inner.height == 0 || inner.width == 0 {
            return;
        }

        if self.selected < self.scroll {
            self.scroll = self.selected;
        }

        loop {
            let mut used = 0u16;
            let mut reached = false;
            for i in self.scroll..=self.selected {
                let Some(line) = self.text.lines.get(i as usize) else {
                    reached = true;
                    break;
                };
                let h = line_height(line, inner.width, self.wrap);
                if used.saturating_add(h) > inner.height {
                    break;
                }
                used = used.saturating_add(h);
                if i == self.selected {
                    reached = true;
                }
            }
            if reached {
                break;
            }
            if self.scroll >= self.selected {
                break;
            }
            self.scroll = self.scroll.saturating_add(1);
        }

        let max = self.max_scroll_for(&inner);
        if self.scroll > max {
            self.scroll = max;
        }
    }

    fn max_scroll_for(&self, inner: &Rect) -> u16 {
        let total = self.text.lines.len();
        if total == 0 || inner.height == 0 || inner.width == 0 {
            return 0;
        }

        let mut used = 0u16;
        let mut start = total;
        for i in (0..total).rev() {
            let h = line_height(&self.text.lines[i], inner.width, self.wrap);
            if used.saturating_add(h) > inner.height {
                break;
            }
            used = used.saturating_add(h);
            start = i;
        }
        start as u16
    }

    fn visible_source_lines(&self) -> u16 {
        let Some(area) = self.last_viewport.get() else {
            return 1;
        };
        let inner = self.content_area(area);
        if inner.height == 0 || inner.width == 0 {
            return 1;
        }

        let mut used = 0u16;
        let mut count = 0u16;

        for line in self.text.lines.iter().skip(self.scroll as usize) {
            let h = line_height(line, inner.width, self.wrap);
            if used.saturating_add(h) > inner.height {
                break;
            }
            used = used.saturating_add(h);
            count = count.saturating_add(1);
        }

        count.max(1)
    }

    pub fn total_visual_lines(&self, width: u16) -> u16 {
        if width == 0 {
            return self.text.lines.len() as u16;
        }
        let mut sum: u16 = 0;
        for line in &self.text.lines {
            sum = sum.saturating_add(line_height(line, width, self.wrap));
        }
        sum
    }
}

impl Default for Content {
    fn default() -> Self {
        Self::new(Text::default())
    }
}

fn line_height(line: &Line, width: u16, wrap: bool) -> u16 {
    if width == 0 {
        return 1;
    }
    let chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    if chars == 0 || !wrap {
        return 1;
    }
    ((chars as u16).saturating_sub(1) / width) + 1
}

impl Widget for &Content {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        self.last_viewport.set(Some(area));
        buf.set_style(area, self.style);

        let inner = self.content_area(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let bottom = inner.bottom();
        let mut y = inner.y;
        let sel = self.selected_style.get();

        for (idx, line) in self.text.lines.iter().enumerate().skip(self.scroll as usize) {
            if y >= bottom {
                break;
            }

            let is_selected = idx as u16 == self.selected;

            let rows = if self.wrap {
                render_wrapped_line(line, inner, y, buf)
            } else {
                render_truncated_line(line, inner, y, buf);
                1
            };

            if is_selected {
                let right = inner.right();
                for row in y..(y.saturating_add(rows)).min(bottom) {
                    for x in inner.x..right {
                        let cell = &mut buf[(x, row)];
                        let patched = cell.style().patch(sel);
                        cell.set_style(patched);
                    }
                }
            }

            y = y.saturating_add(rows);
        }
    }
}

fn render_wrapped_line(line: &Line, area: Rect, start_y: u16, buf: &mut Buffer) -> u16 {
    let mut x = area.x;
    let mut y = start_y;
    let right = area.right();
    let bottom = area.bottom();

    for span in &line.spans {
        for ch in span.content.chars() {
            if y >= bottom {
                return y.saturating_sub(start_y);
            }
            if x >= right {
                x = area.x;
                y = y.saturating_add(1);
                if y >= bottom {
                    return y.saturating_sub(start_y);
                }
            }
            buf[(x, y)].set_char(ch).set_style(span.style);
            x = x.saturating_add(1);
        }
    }

    y.saturating_sub(start_y).saturating_add(1)
}

fn render_truncated_line(line: &Line, area: Rect, y: u16, buf: &mut Buffer) {
    let mut x = area.x;
    let right = area.right();

    'outer: for span in &line.spans {
        for ch in span.content.chars() {
            if x >= right {
                break 'outer;
            }
            buf[(x, y)].set_char(ch).set_style(span.style);
            x = x.saturating_add(1);
        }
    }
}

