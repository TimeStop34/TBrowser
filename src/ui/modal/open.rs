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
pub enum DialogFocus {
    Input,
    Ok,
    Cancel,
}

pub struct InputDialog {
    pub is_open: bool,
    pub value: String,
    pub cursor_pos: usize,
    pub offset: usize,
    pub focus: DialogFocus,
    pub title: String,
    pub is_email: bool,
    pub width: u16,
    pub height: u16,
    last_area: Cell<Option<Rect>>,
    submission: Option<String>,
}

impl InputDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            value: String::new(),
            cursor_pos: 0,
            offset: 0,
            focus: DialogFocus::Input,
            title: "Input".to_string(),
            is_email: false,
            width: 60,
            height: 6,
            last_area: Cell::new(None),
            submission: None,
        }
    }

    pub fn open(&mut self, initial: impl Into<String>, title: impl Into<String>, is_email: bool) {
        self.value = initial.into();
        self.cursor_pos = self.value.chars().count();
        self.offset = 0;
        self.focus = DialogFocus::Input;
        self.title = title.into();
        self.is_email = is_email;
        self.is_open = true;
        self.submission = None;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn take_submission(&mut self) -> Option<String> {
        self.submission.take()
    }

    pub fn compute_area(&self, screen: Rect) -> Rect {
        compute_area(screen, self.width, self.height)
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        if !self.is_open || self.focus != DialogFocus::Input {
            return None;
        }
        let area = self.last_area.get()?;
        let x = area.x + 2 + (self.cursor_pos.saturating_sub(self.offset)) as u16;
        let y = area.y + 2;
        Some((x, y))
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_open {
            return false;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.close();
                return true;
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.close();
                return true;
            }
            _ => {}
        }

        match self.focus {
            DialogFocus::Input => self.handle_input_key(key),
            DialogFocus::Ok | DialogFocus::Cancel => self.handle_button_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => {
                if self.is_email && !validate_email(&self.value) {
                    self.title = "Invalid email".to_string();
                    return true;
                }
                self.focus = DialogFocus::Ok;
                true
            }
            (KeyCode::Down, _) | (KeyCode::Tab, _) => {
                self.focus = DialogFocus::Ok;
                true
            }
            (KeyCode::Up, _) => true,
            (KeyCode::Backspace, _) => {
                if self.cursor_pos > 0 {
                    remove_char_at(&mut self.value, self.cursor_pos - 1);
                    self.cursor_pos -= 1;
                    self.update_offset();
                }
                if self.title == "Invalid email" {
                    self.title = "Edit Email".to_string();
                }
                true
            }
            (KeyCode::Delete, _) => {
                if self.cursor_pos < self.value.chars().count() {
                    remove_char_at(&mut self.value, self.cursor_pos);
                    self.update_offset();
                }
                true
            }
            (KeyCode::Left, _) => {
                if self.cursor_pos > 0 {
                    self.cursor_pos -= 1;
                    self.update_offset();
                }
                true
            }
            (KeyCode::Right, _) => {
                if self.cursor_pos < self.value.chars().count() {
                    self.cursor_pos += 1;
                    self.update_offset();
                }
                true
            }
            (KeyCode::Home, _) => {
                self.cursor_pos = 0;
                self.update_offset();
                true
            }
            (KeyCode::End, _) => {
                self.cursor_pos = self.value.chars().count();
                self.update_offset();
                true
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if c.is_control() {
                    return true;
                }
                insert_char_at(&mut self.value, self.cursor_pos, c);
                self.cursor_pos += 1;
                self.update_offset();
                if self.title == "Invalid email" {
                    self.title = "Edit Email".to_string();
                }
                true
            }
            _ => false,
        }
    }

    fn handle_button_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                self.focus = match self.focus {
                    DialogFocus::Ok => DialogFocus::Cancel,
                    DialogFocus::Cancel => DialogFocus::Ok,
                    _ => self.focus,
                };
                true
            }
            KeyCode::Up | KeyCode::Tab => {
                self.focus = DialogFocus::Input;
                true
            }
            KeyCode::Down => true,
            KeyCode::Enter => {
                match self.focus {
                    DialogFocus::Ok => {
                        if self.is_email && !validate_email(&self.value) {
                            self.title = "Invalid email".to_string();
                            self.focus = DialogFocus::Input;
                            return true;
                        }
                        self.submission = Some(self.value.clone());
                        self.close();
                    }
                    DialogFocus::Cancel => {
                        self.close();
                    }
                    _ => {}
                }
                true
            }
            _ => false,
        }
    }

    fn update_offset(&mut self) {
        let Some(area) = self.last_area.get() else {
            return;
        };
        let input_width = area.width.saturating_sub(4) as usize;
        if input_width == 0 {
            return;
        }
        if self.cursor_pos < self.offset {
            self.offset = self.cursor_pos;
        } else if self.cursor_pos >= self.offset + input_width {
            self.offset = self.cursor_pos - input_width + 1;
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

        let input_x = area.x + 2;
        let input_y = area.y + 2;
        let input_width = area.width.saturating_sub(4);
        if input_width == 0 {
            return;
        }

        let input_style = Style::default().fg(theme.input.fg).bg(theme.input.bg);

        let input_area = Rect {
            x: input_x,
            y: input_y,
            width: input_width,
            height: 1,
        };
        buf.set_style(input_area, input_style);

        let visible: String = self
            .value
            .chars()
            .skip(self.offset)
            .take(input_width as usize)
            .collect();
        buf.set_string(input_x, input_y, &visible, input_style);

        let ok_label = theme.button.format.replace("{}", "OK");
        let cancel_label = theme.button.format.replace("{}", "Cancel");
        let ok_len = ok_label.chars().count() as u16;
        let cancel_len = cancel_label.chars().count() as u16;
        let total = ok_len + cancel_len + 4;
        if total > area.width {
            return;
        }
        let btn_y = area.y + area.height.saturating_sub(2);
        let btn_x = area.x + area.width.saturating_sub(total) / 2;

        let ok_style = if self.focus == DialogFocus::Ok {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default()
                .fg(theme.button.fg)
                .bg(theme.button.bg)
        };
        let cancel_style = if self.focus == DialogFocus::Cancel {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default()
                .fg(theme.button.fg)
                .bg(theme.button.bg)
        };

        buf.set_string(btn_x, btn_y, &ok_label, ok_style);
        buf.set_string(btn_x + ok_len + 4, btn_y, &cancel_label, cancel_style);
    }
}

impl Default for InputDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn insert_char_at(s: &mut String, char_pos: usize, ch: char) {
    let byte_pos = s
        .char_indices()
        .nth(char_pos)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    s.insert(byte_pos, ch);
}

fn remove_char_at(s: &mut String, char_pos: usize) {
    let byte_start = s.char_indices().nth(char_pos).map(|(i, _)| i);
    let byte_end = s
        .char_indices()
        .nth(char_pos + 1)
        .map(|(i, _)| i)
        .unwrap_or(s.len());
    if let Some(start) = byte_start {
        s.replace_range(start..byte_end, "");
    }
}

fn validate_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}