use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Text,
};

use crate::core::bookmarks::Bookmarks;
use crate::core::tabs::TabManager;
use crate::ui::main::Header;
use crate::ui::{inner_area, render_border};
use crate::ui::GlobalTheme;

const PADDING_H: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Tabs,
    Bookmarks,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMenuButton {
    Create,
    Favorite,
    Delete,
    Close,
}

pub struct Sidebar {
    pub mode: SidebarMode,
    pub cursor: usize,
    pub scroll: u16,
    pub button_index: usize,
}

impl Sidebar {
    pub fn new() -> Self {
        Self {
            mode: SidebarMode::Tabs,
            cursor: 0,
            scroll: 0,
            button_index: 0,
        }
    }

    pub fn reset_for(&mut self, mode: SidebarMode, tabs: &TabManager) {
        self.mode = mode;
        self.scroll = 0;
        self.button_index = 0;
        self.cursor = match mode {
            SidebarMode::Tabs => tabs.active,
            _ => 0,
        };
    }

    pub fn button_labels(&self) -> &'static [&'static str] {
        match self.mode {
            SidebarMode::Tabs => &["Create", "Delete", "Close"],
            SidebarMode::Bookmarks => &["Create", "Favorite", "Delete", "Close"],
            SidebarMode::Info => &["Close"],
        }
    }

    pub fn button_count(&self) -> usize {
        self.button_labels().len()
    }

    pub fn button_action(&self) -> SidebarMenuButton {
        match self.mode {
            SidebarMode::Tabs => match self.button_index {
                0 => SidebarMenuButton::Create,
                1 => SidebarMenuButton::Delete,
                _ => SidebarMenuButton::Close,
            },
            SidebarMode::Bookmarks => match self.button_index {
                0 => SidebarMenuButton::Create,
                1 => SidebarMenuButton::Favorite,
                2 => SidebarMenuButton::Delete,
                _ => SidebarMenuButton::Close,
            },
            SidebarMode::Info => SidebarMenuButton::Close,
        }
    }

    pub fn up(&mut self, total: usize) {
        if total == 0 {
            return;
        }
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn down(&mut self, total: usize) {
        if total == 0 {
            return;
        }
        if self.cursor + 1 < total {
            self.cursor += 1;
        }
    }

    pub fn clamp_cursor(&mut self, total: usize) {
        if total == 0 {
            self.cursor = 0;
        } else if self.cursor >= total {
            self.cursor = total - 1;
        }
    }

    pub fn clamp_button(&mut self) {
        let n = self.button_count();
        if n == 0 {
            self.button_index = 0;
        } else if self.button_index >= n {
            self.button_index = n - 1;
        }
    }

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tabs: &TabManager,
        bookmarks: &Bookmarks,
        info_text: &Text<'static>,
        body_focused: bool,
        menu_focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width < 2 || area.height < 6 {
            return;
        }

        let [header_area, _gap1, body_area, _gap2, menu_area, _gap3] =
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ])
            .areas(area);

        let title = match self.mode {
            SidebarMode::Tabs => "Tabs",
            SidebarMode::Bookmarks => "Bookmarks",
            SidebarMode::Info => "Info",
        };
        let header = Header::new(title, "");
        header.render(header_area, buf, theme);

        render_border(body_area, buf, &theme.border, None);
        let inner = inner_area(body_area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let pad = PADDING_H.min(inner.width / 2);
        let content_area = Rect {
            x: inner.x + pad,
            y: inner.y,
            width: inner.width.saturating_sub(pad * 2),
            height: inner.height,
        };
        if content_area.width == 0 {
            return;
        }

        match self.mode {
            SidebarMode::Tabs => {
                self.render_tabs(content_area, buf, tabs, body_focused, theme)
            }
            SidebarMode::Bookmarks => {
                self.render_bookmarks(content_area, buf, bookmarks, body_focused, theme)
            }
            SidebarMode::Info => {
                self.render_info(content_area, buf, info_text, body_focused, theme)
            }
        }

        self.render_menu(menu_area, buf, menu_focused, theme);
    }

    fn render_menu(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let labels = self.button_labels();
        let rendered: Vec<String> = labels
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let selected = focused && self.button_index == i;
                let fmt = if selected {
                    &theme.button.selected_format
                } else {
                    &theme.button.format
                };
                fmt.replace("{}", l)
            })
            .collect();

        let sum: u16 = rendered.iter().map(|s| s.chars().count() as u16).sum();
        let gaps = rendered.len().saturating_sub(1) as u16;
        let total = sum.saturating_add(4u16.saturating_mul(gaps));

        if total > area.width {
            return;
        }
        let mut bx = area.x + area.width.saturating_sub(total) / 2;

        for (i, label) in rendered.iter().enumerate() {
            let selected = focused && self.button_index == i;
            let style = if selected {
                Style::default()
                    .fg(theme.button.selected_fg)
                    .bg(theme.button.selected_bg)
            } else {
                Style::default().fg(theme.button.fg).bg(theme.button.bg)
            };
            buf.set_string(bx, area.y, label, style);
            bx = bx.saturating_add(label.chars().count() as u16 + 4);
        }
    }

    fn compute_scroll(&self, viewport: usize, total: usize) -> usize {
        if viewport == 0 || total <= viewport {
            return 0;
        }
        let mut scroll = self.scroll as usize;
        if self.cursor < scroll {
            scroll = self.cursor;
        } else if self.cursor >= scroll + viewport {
            scroll = self.cursor - viewport + 1;
        }
        let max = total - viewport;
        scroll.min(max)
    }

    fn render_tabs(
        &self,
        area: Rect,
        buf: &mut Buffer,
        tabs: &TabManager,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let viewport = area.height as usize;
        let total = tabs.len();
        if total == 0 {
            return;
        }
        let scroll = self.compute_scroll(viewport, total);
        let start = scroll;
        let end = (start + viewport).min(total);

        for (i, tab) in tabs.tabs.iter().enumerate().skip(start).take(end - start) {
            let row = area.y + (i - start) as u16;
            let is_active = i == tabs.active;
            let is_cursor = focused && i == self.cursor;

            let base_label = format!("{}. {}", i + 1, tab.name);
            let label = if is_active {
                format!("{} [x]", base_label)
            } else {
                base_label
            };

            let base_style = if is_active {
                Style::default().fg(theme.selected_tab_color)
            } else {
                Style::default().fg(theme.modal_fg)
            };

            let style = if is_cursor {
                Style::default()
                    .fg(theme.selection.fg)
                    .bg(theme.selection.bg)
            } else {
                base_style
            };

            let display: String = label.chars().take(area.width as usize).collect();
            buf.set_string(area.x, row, &display, style);

            if is_cursor {
                let used = display.chars().count() as u16;
                if used < area.width {
                    buf.set_string(
                        area.x + used,
                        row,
                        &" ".repeat((area.width - used) as usize),
                        style,
                    );
                }
            }
        }
    }

    fn render_bookmarks(
        &self,
        area: Rect,
        buf: &mut Buffer,
        bookmarks: &Bookmarks,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let viewport = area.height as usize;
        let total = bookmarks.len();
        if total == 0 {
            buf.set_string(
                area.x,
                area.y,
                "(no bookmarks)",
                Style::default().fg(theme.modal_fg),
            );
            return;
        }
        let scroll = self.compute_scroll(viewport, total);
        let start = scroll;
        let end = (start + viewport).min(total);

        for (i, bm) in bookmarks.items.iter().enumerate().skip(start).take(end - start) {
            let row = area.y + (i - start) as u16;
            let is_cursor = focused && i == self.cursor;

            let prefix = match bm.favorite {
                Some(slot) => format!("{}.", slot),
                None => "-".to_string(),
            };
            let label = format!("{} {}", prefix, bm.title);

            let style = if is_cursor {
                Style::default()
                    .fg(theme.selection.fg)
                    .bg(theme.selection.bg)
            } else {
                Style::default().fg(theme.modal_fg)
            };

            let display: String = label.chars().take(area.width as usize).collect();
            buf.set_string(area.x, row, &display, style);

            if is_cursor {
                let used = display.chars().count() as u16;
                if used < area.width {
                    buf.set_string(
                        area.x + used,
                        row,
                        &" ".repeat((area.width - used) as usize),
                        style,
                    );
                }
            }
        }
    }

    fn render_info(
        &self,
        area: Rect,
        buf: &mut Buffer,
        info_text: &Text<'static>,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let viewport = area.height as usize;
        let total = info_text.lines.len();
        if total == 0 {
            return;
        }
        let scroll = self.compute_scroll(viewport, total);
        let start = scroll;
        let end = (start + viewport).min(total);

        let sel = Style::default()
            .fg(theme.selection.fg)
            .bg(theme.selection.bg);

        for (i, line) in info_text.lines.iter().enumerate().skip(start).take(end - start) {
            let row = area.y + (i - start) as u16;
            let is_cursor = focused && i == self.cursor;

            let mut x = area.x;
            'outer: for span in &line.spans {
                for ch in span.content.chars() {
                    if x >= area.right() {
                        break 'outer;
                    }
                    buf[(x, row)].set_char(ch).set_style(span.style);
                    x = x.saturating_add(1);
                }
            }

            if is_cursor {
                for xx in area.x..area.right() {
                    let cell = &mut buf[(xx, row)];
                    let patched = cell.style().patch(sel);
                    cell.set_style(patched);
                }
            }
        }
    }
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}