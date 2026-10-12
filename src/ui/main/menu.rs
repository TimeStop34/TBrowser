use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use crate::ui::GlobalTheme;

pub type ButtonFuture = Pin<Box<dyn Future<Output = ()> + Send>>;
pub type ButtonCallback = Arc<dyn Fn() -> ButtonFuture + Send + Sync>;
pub type MenuButton = (String, ButtonCallback);

pub struct Menu {
    pub buttons: Vec<MenuButton>,
    pub selected: usize,
    pub separator: String,
    pub sep_count: usize,
    pub focus: bool,
}

impl Menu {
    pub fn new(buttons: Vec<MenuButton>) -> Self {
        Self {
            buttons,
            selected: 0,
            separator: " ".to_string(),
            sep_count: 4,
            focus: true,
        }
    }

    #[allow(unused)]
    pub fn separator(mut self, sep: impl Into<String>) -> Self {
        self.separator = sep.into();
        self
    }

    #[allow(unused)]
    pub fn sep_count(mut self, n: usize) -> Self {
        self.sep_count = n;
        self
    }

    pub fn next(&mut self) {
        if !self.buttons.is_empty() {
            self.selected = (self.selected + 1) % self.buttons.len();
        }
    }

    pub fn prev(&mut self) {
        if !self.buttons.is_empty() {
            self.selected = (self.selected + self.buttons.len() - 1) % self.buttons.len();
        }
    }

    pub fn activate(&self) {
        let cb = self.buttons.get(self.selected).map(|(_, cb)| Arc::clone(cb));
        if let Some(cb) = cb {
            tokio::spawn(cb());
        }
    }

    fn format_label(&self, label: &str, is_selected: bool, theme: &GlobalTheme) -> String {
        let fmt = if is_selected {
            &theme.button.selected_format
        } else {
            &theme.button.format
        };
        fmt.replace("{}", label)
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width == 0 || area.height == 0 || self.buttons.is_empty() {
            return;
        }

        let active_selected = self.focus;
        let selected_idx = self.selected;

        let labels: Vec<String> = self
            .buttons
            .iter()
            .enumerate()
            .map(|(i, (l, _))| {
                let is_sel = i == selected_idx && active_selected;
                self.format_label(l, is_sel, theme)
            })
            .collect();
        let widths: Vec<u16> = labels.iter().map(|s| s.chars().count() as u16).collect();

        let labels_total: u16 = widths.iter().copied().sum();
        let gaps = self.buttons.len().saturating_sub(1) as u16;
        let sep_unit_width = self.separator.chars().count() as u16;

        let sep_count = if gaps == 0 || sep_unit_width == 0 || self.sep_count == 0 {
            0
        } else {
            let available = area.width.saturating_sub(labels_total);
            let max_per_gap = available / gaps;
            let max_count = max_per_gap / sep_unit_width;
            (self.sep_count as u16).min(max_count).max(1)
        };

        let sep_string: String = self.separator.repeat(sep_count as usize);
        let sep_width = sep_string.chars().count() as u16;

        let total_width = labels_total.saturating_add(sep_width.saturating_mul(gaps));
        let start_x = area.x + area.width.saturating_sub(total_width) / 2;
        let y = area.y;

        let mut x = start_x;
        for (i, label) in labels.iter().enumerate() {
            if x >= area.right() {
                break;
            }

            let is_sel = i == selected_idx && active_selected;
            let style = if is_sel {
                Style::default()
                    .fg(theme.button.selected_fg)
                    .bg(theme.button.selected_bg)
            } else {
                Style::default()
                    .fg(theme.button.fg)
                    .bg(theme.button.bg)
            };

            let max_w = area.right().saturating_sub(x) as usize;
            let display: String = label.chars().take(max_w).collect();
            let w = display.chars().count() as u16;

            buf.set_string(x, y, &display, style);
            x = x.saturating_add(w);

            if i + 1 < labels.len() {
                let max_w = area.right().saturating_sub(x) as usize;
                let sep_display: String = sep_string.chars().take(max_w).collect();
                buf.set_string(x, y, &sep_display, Style::default());
                x = x.saturating_add(sep_display.chars().count() as u16);
            }
        }
    }
}