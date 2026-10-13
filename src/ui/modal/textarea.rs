use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::Style,
};

use crate::ui::modal::base::{fill_background, render_border};
use crate::ui::GlobalTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextareaFocus {
    Text,
    Buttons,
}

pub struct TextareaDialog {
    pub is_open: bool,
    pub value: String,
    pub cursor_pos: usize,
    offset: Cell<usize>,
    pub focus: TextareaFocus,
    pub button_index: usize,
    pub title: String,
    last_area: Cell<Option<Rect>>,
    submission: Option<String>,
}

impl TextareaDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            value: String::new(),
            cursor_pos: 0,
            offset: Cell::new(0),
            focus: TextareaFocus::Text,
            button_index: 0,
            title: "Edit Textarea".to_string(),
            last_area: Cell::new(None),
            submission: None,
        }
    }

    pub fn open(&mut self, initial: impl Into<String>, title: impl Into<String>) {
        self.value = initial.into();
        self.cursor_pos = self.value.len();
        self.offset.set(0);
        self.focus = TextareaFocus::Text;
        self.button_index = 0;
        self.title = title.into();
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
        let max_w = screen.width.saturating_sub(4).max(20);
        let max_h = screen.height.saturating_sub(6).max(5);
        let w = 70u16.min(max_w);
        let h = 20u16.min(max_h);
        let x = screen.x + screen.width.saturating_sub(w) / 2;
        let y = screen.y + screen.height.saturating_sub(h) / 2;
        Rect {
            x,
            y,
            width: w,
            height: h,
        }
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        if !self.is_open || self.focus != TextareaFocus::Text {
            return None;
        }
        let area = self.last_area.get()?;
        let content_width = area.width.saturating_sub(4) as usize;
        if content_width == 0 {
            return None;
        }
        let content_height = area.height.saturating_sub(3) as usize;
        if content_height == 0 {
            return None;
        }
        let (vl, vc) = cursor_visual_pos(&self.value, self.cursor_pos, content_width);
        let off = self.offset.get();
        if vl < off || vl >= off + content_height {
            return None;
        }
        Some((area.x + 2 + vc as u16, area.y + 1 + (vl - off) as u16))
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
            TextareaFocus::Text => self.handle_text_key(key),
            TextareaFocus::Buttons => self.handle_button_key(key),
        }
    }

    fn handle_text_key(&mut self, key: KeyEvent) -> bool {
        match (key.code, key.modifiers) {
            (KeyCode::Enter, _) => {
                self.value.insert(self.cursor_pos, '\n');
                self.cursor_pos += 1;
                true
            }
            (KeyCode::Tab, _) => {
                self.focus = TextareaFocus::Buttons;
                self.button_index = 0;
                true
            }
            (KeyCode::Backspace, _) => {
                if self.cursor_pos > 0 {
                    let prev = self.value[..self.cursor_pos].chars().last().unwrap();
                    let new_pos = self.cursor_pos - prev.len_utf8();
                    self.value.replace_range(new_pos..self.cursor_pos, "");
                    self.cursor_pos = new_pos;
                }
                true
            }
            (KeyCode::Delete, _) => {
                if self.cursor_pos < self.value.len() {
                    let next = self.value[self.cursor_pos..].chars().next().unwrap();
                    self.value
                        .replace_range(self.cursor_pos..self.cursor_pos + next.len_utf8(), "");
                }
                true
            }
            (KeyCode::Left, _) => {
                if self.cursor_pos > 0 {
                    let prev = self.value[..self.cursor_pos].chars().last().unwrap();
                    self.cursor_pos -= prev.len_utf8();
                }
                true
            }
            (KeyCode::Right, _) => {
                if self.cursor_pos < self.value.len() {
                    let next = self.value[self.cursor_pos..].chars().next().unwrap();
                    self.cursor_pos += next.len_utf8();
                }
                true
            }
            (KeyCode::Up, _) => {
                self.move_up();
                true
            }
            (KeyCode::Down, _) => {
                self.move_down();
                true
            }
            (KeyCode::Home, _) => {
                let (line, _) = cursor_line_col(&self.value, self.cursor_pos);
                self.cursor_pos = byte_offset_of_line_col(&self.value, line, 0);
                true
            }
            (KeyCode::End, _) => {
                let (line, _) = cursor_line_col(&self.value, self.cursor_pos);
                let lines: Vec<&str> = self.value.split('\n').collect();
                let len = lines.get(line).map(|l| l.chars().count()).unwrap_or(0);
                self.cursor_pos = byte_offset_of_line_col(&self.value, line, len);
                true
            }
            (KeyCode::PageUp, _) => {
                let off = self.offset.get();
                self.offset.set(off.saturating_sub(10));
                true
            }
            (KeyCode::PageDown, _) => {
                let off = self.offset.get();
                self.offset.set(off + 10);
                true
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if c.is_control() {
                    return true;
                }
                self.value.insert(self.cursor_pos, c);
                self.cursor_pos += c.len_utf8();
                true
            }
            _ => false,
        }
    }

    fn handle_button_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Left | KeyCode::Right => {
                self.button_index = 1 - self.button_index.min(1);
                true
            }
            KeyCode::Tab | KeyCode::Up => {
                self.focus = TextareaFocus::Text;
                true
            }
            KeyCode::Enter => {
                if self.button_index == 0 {
                    self.submission = Some(self.value.clone());
                    self.close();
                } else {
                    self.close();
                }
                true
            }
            _ => false,
        }
    }

    fn move_up(&mut self) {
        let (line, col) = cursor_line_col(&self.value, self.cursor_pos);
        if line == 0 {
            return;
        }
        let lines: Vec<&str> = self.value.split('\n').collect();
        let prev_len = lines[line - 1].chars().count();
        let target_col = col.min(prev_len);
        self.cursor_pos = byte_offset_of_line_col(&self.value, line - 1, target_col);
    }

    fn move_down(&mut self) {
        let (line, col) = cursor_line_col(&self.value, self.cursor_pos);
        let lines: Vec<&str> = self.value.split('\n').collect();
        if line + 1 >= lines.len() {
            return;
        }
        let next_len = lines[line + 1].chars().count();
        let target_col = col.min(next_len);
        self.cursor_pos = byte_offset_of_line_col(&self.value, line + 1, target_col);
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width < 6 || area.height < 5 {
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

        let content_x = area.x + 2;
        let content_y = area.y + 1;
        let content_width = area.width.saturating_sub(4) as usize;
        let content_height = area.height.saturating_sub(3) as usize;
        if content_width == 0 || content_height == 0 {
            return;
        }

        let visual = visual_lines(&self.value, content_width);

        let (vl, _vc) = cursor_visual_pos(&self.value, self.cursor_pos, content_width);
        let mut off = self.offset.get();
        if vl < off {
            off = vl;
        } else if vl >= off + content_height {
            off = vl - content_height + 1;
        }
        let max_off = visual.len().saturating_sub(content_height);
        if off > max_off {
            off = max_off;
        }
        self.offset.set(off);

        let text_style = Style::default().fg(theme.modal_fg).bg(theme.modal_bg);
        let blank = " ".repeat(content_width);

        for i in 0..content_height {
            let row = content_y + i as u16;
            let idx = off + i;
            if idx < visual.len() {
                let display: String = visual[idx].chars().take(content_width).collect();
                buf.set_string(content_x, row, &display, text_style);
                let used = display.chars().count();
                if used < content_width {
                    let pad = " ".repeat(content_width - used);
                    buf.set_string(content_x + used as u16, row, &pad, text_style);
                }
            } else {
                buf.set_string(content_x, row, &blank, text_style);
            }
        }

        let save_label = theme.button.format.replace("{}", "Save");
        let cancel_label = theme.button.format.replace("{}", "Cancel");
        let save_len = save_label.chars().count() as u16;
        let cancel_len = cancel_label.chars().count() as u16;
        let total = save_len + cancel_len + 4;
        if total > area.width {
            return;
        }
        let btn_y = area.y + area.height.saturating_sub(2);
        let btn_x = area.x + area.width.saturating_sub(total) / 2;

        let save_focused = self.focus == TextareaFocus::Buttons && self.button_index == 0;
        let cancel_focused = self.focus == TextareaFocus::Buttons && self.button_index == 1;

        let save_style = if save_focused {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default().fg(theme.button.fg).bg(theme.button.bg)
        };
        let cancel_style = if cancel_focused {
            Style::default()
                .fg(theme.button.selected_fg)
                .bg(theme.button.selected_bg)
        } else {
            Style::default().fg(theme.button.fg).bg(theme.button.bg)
        };

        buf.set_string(btn_x, btn_y, &save_label, save_style);
        buf.set_string(btn_x + save_len + 4, btn_y, &cancel_label, cancel_style);
    }
}

impl Default for TextareaDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn cursor_line_col(text: &str, cursor_byte: usize) -> (usize, usize) {
    let mut line = 0;
    let mut col = 0;
    for (i, c) in text.char_indices() {
        if i >= cursor_byte {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn byte_offset_of_line_col(text: &str, line: usize, col: usize) -> usize {
    let mut cur_line = 0;
    let mut cur_col = 0;
    for (i, c) in text.char_indices() {
        if cur_line == line && cur_col == col {
            return i;
        }
        if c == '\n' {
            cur_line += 1;
            cur_col = 0;
        } else {
            cur_col += 1;
        }
    }
    text.len()
}

fn visual_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut result = Vec::new();
    for line in text.split('\n') {
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            result.push(String::new());
        } else {
            let mut i = 0;
            while i < chars.len() {
                let end = (i + width).min(chars.len());
                result.push(chars[i..end].iter().collect());
                i = end;
            }
        }
    }
    result
}

fn cursor_visual_pos(text: &str, cursor_byte: usize, width: usize) -> (usize, usize) {
    if width == 0 {
        return (0, 0);
    }
    let mut visual_line = 0;
    let mut byte_offset = 0;
    let lines: Vec<&str> = text.split('\n').collect();
    for line in lines.iter() {
        let line_len = line.len();
        if cursor_byte >= byte_offset && cursor_byte <= byte_offset + line_len {
            let col_bytes = cursor_byte - byte_offset;
            let col_chars = line[..col_bytes].chars().count();
            return (visual_line + col_chars / width, col_chars % width);
        }
        let h = if line.is_empty() {
            1
        } else {
            (line.chars().count() + width - 1) / width
        };
        visual_line += h;
        byte_offset += line_len + 1;
    }
    (visual_line.saturating_sub(1), 0)
}