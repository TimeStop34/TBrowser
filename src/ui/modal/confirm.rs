use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::Style,
};

use crate::ui::modal::base::{compute_area, fill_background, render_border};
use crate::ui::GlobalTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmFocus {
    Ok,
    Cancel,
}

pub struct ConfirmDialog {
    pub is_open: bool,
    pub title: String,
    pub message: String,
    pub ok_label: String,
    pub cancel_label: String,
    pub focus: ConfirmFocus,
    pub width: u16,
    pub height: u16,
    last_area: Cell<Option<Rect>>,
    submission: Option<bool>,
}

impl ConfirmDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: "Confirm".to_string(),
            message: String::new(),
            ok_label: "Yes".to_string(),
            cancel_label: "No".to_string(),
            focus: ConfirmFocus::Ok,
            width: 50,
            height: 7,
            last_area: Cell::new(None),
            submission: None,
        }
    }

    pub fn open(&mut self, title: impl Into<String>, message: impl Into<String>) {
        self.title = title.into();
        self.message = message.into();
        self.focus = ConfirmFocus::Ok;
        self.is_open = true;
        self.submission = None;

        let msg_w = self.message.chars().count() as u16 + 4;
        let title_w = self.title.chars().count() as u16 + 6;
        self.width = msg_w.max(title_w).max(30).min(70);
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn take_submission(&mut self) -> Option<bool> {
        self.submission.take()
    }

    pub fn compute_area(&self, screen: Rect) -> Rect {
        compute_area(screen, self.width, self.height)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_open {
            return false;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.submission = Some(false);
                self.close();
                return true;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.submission = Some(false);
                self.close();
                return true;
            }
            _ => {}
        }

        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                self.focus = match self.focus {
                    ConfirmFocus::Ok => ConfirmFocus::Cancel,
                    ConfirmFocus::Cancel => ConfirmFocus::Ok,
                };
                true
            }
            KeyCode::Enter => {
                self.submission = Some(self.focus == ConfirmFocus::Ok);
                self.close();
                true
            }
            _ => false,
        }
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width < 4 || area.height < 3 {
            return;
        }

        self.last_area.set(Some(area));

        let bg_style = Style::default().bg(theme.modal_bg).fg(theme.modal_fg);
        fill_background(area, buf, bg_style);

        let title = if self.title.is_empty() {
            None
        } else {
            Some((self.title.as_str(), &theme.title))
        };
        render_border(area, buf, &theme.border, title);

        let inner_x = area.x + 2;
        let inner_y = area.y + 2;
        let inner_w = area.width.saturating_sub(4);

        let msg_line: String = self.message.chars().take(inner_w as usize).collect();
        buf.set_string(
            inner_x,
            inner_y,
            &msg_line,
            Style::default().fg(theme.modal_fg),
        );

        let ok_label = theme.button.format.replace("{}", &self.ok_label);
        let cancel_label = theme.button.format.replace("{}", &self.cancel_label);
        let ok_len = ok_label.chars().count() as u16;
        let cancel_len = cancel_label.chars().count() as u16;
        let total = ok_len + cancel_len + 4;
        if total > area.width {
            return;
        }
        let btn_y = area.y + area.height.saturating_sub(2);
        let btn_x = area.x + area.width.saturating_sub(total) / 2;

        let ok_style = if self.focus == ConfirmFocus::Ok {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default().fg(theme.button.fg).bg(theme.button.bg)
        };
        let cancel_style = if self.focus == ConfirmFocus::Cancel {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default().fg(theme.button.fg).bg(theme.button.bg)
        };

        buf.set_string(btn_x, btn_y, &ok_label, ok_style);
        buf.set_string(btn_x + ok_len + 4, btn_y, &cancel_label, cancel_style);
    }
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new()
    }
}