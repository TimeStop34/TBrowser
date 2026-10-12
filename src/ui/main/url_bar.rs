use std::cell::Cell;

use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::{Color, Style},
};

use crate::ui::GlobalTheme;

const PREFIX: &str = "  URL: ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStage {
    Idle,
    Fetching,
    Parsing,
    Rendering,
    Done,
}

impl LoadStage {
    pub fn progress(&self) -> f64 {
        match self {
            LoadStage::Idle => 0.0,
            LoadStage::Fetching => 0.25,
            LoadStage::Parsing => 0.50,
            LoadStage::Rendering => 0.75,
            LoadStage::Done => 1.0,
        }
    }

    pub fn is_loading(&self) -> bool {
        !matches!(self, LoadStage::Idle)
    }
}

pub struct UrlBar {
    pub stage: LoadStage,
    pub loading_url: String,
    pub loading_tab: usize,
    pub generation: u64,

    pub editing: bool,
    edit_value: String,
    edit_cursor: usize,
    edit_offset: usize,
    last_area: Cell<Option<Rect>>,
    submission: Option<String>,
    cancelled: bool,
}

impl UrlBar {
    pub fn new() -> Self {
        Self {
            stage: LoadStage::Idle,
            loading_url: String::new(),
            loading_tab: 0,
            generation: 0,
            editing: false,
            edit_value: String::new(),
            edit_cursor: 0,
            edit_offset: 0,
            last_area: Cell::new(None),
            submission: None,
            cancelled: false,
        }
    }

    pub fn is_editing(&self) -> bool {
        self.editing
    }

    pub fn start_edit(&mut self, current_url: &str) {
        self.editing = true;
        self.edit_value = current_url.to_string();
        self.edit_cursor = self.edit_value.chars().count();
        self.edit_offset = 0;
        self.submission = None;
        self.cancelled = false;
    }

    pub fn stop_edit(&mut self) {
        self.editing = false;
        self.edit_value.clear();
        self.edit_cursor = 0;
        self.edit_offset = 0;
        self.submission = None;
        self.cancelled = false;
    }

    pub fn take_submission(&mut self) -> Option<String> {
        self.submission.take()
    }

    pub fn take_cancelled(&mut self) -> bool {
        let c = self.cancelled;
        self.cancelled = false;
        c
    }

    pub fn begin(&mut self, url: impl Into<String>, tab_index: usize) -> u64 {
        self.loading_url = url.into();
        self.loading_tab = tab_index;
        self.stage = LoadStage::Fetching;
        self.generation += 1;
        self.generation
    }

    pub fn set_stage(&mut self, stage: LoadStage) {
        self.stage = stage;
    }

    pub fn hide(&mut self, generation: u64) {
        if self.generation == generation {
            self.stage = LoadStage::Idle;
        }
    }

    pub fn force_idle(&mut self) {
        self.stage = LoadStage::Idle;
        self.generation += 1;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.editing {
            return false;
        }

        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => {
                self.cancelled = true;
                self.editing = false;
                true
            }
            (KeyCode::Enter, _) => {
                self.submission = Some(self.edit_value.clone());
                self.editing = false;
                true
            }
            (KeyCode::Backspace, _) => {
                if self.edit_cursor > 0 {
                    remove_char_at(&mut self.edit_value, self.edit_cursor - 1);
                    self.edit_cursor -= 1;
                    self.update_offset();
                }
                true
            }
            (KeyCode::Delete, _) => {
                if self.edit_cursor < self.edit_value.chars().count() {
                    remove_char_at(&mut self.edit_value, self.edit_cursor);
                    self.update_offset();
                }
                true
            }
            (KeyCode::Left, _) => {
                if self.edit_cursor > 0 {
                    self.edit_cursor -= 1;
                    self.update_offset();
                }
                true
            }
            (KeyCode::Right, _) => {
                if self.edit_cursor < self.edit_value.chars().count() {
                    self.edit_cursor += 1;
                    self.update_offset();
                }
                true
            }
            (KeyCode::Home, _) => {
                self.edit_cursor = 0;
                self.update_offset();
                true
            }
            (KeyCode::End, _) => {
                self.edit_cursor = self.edit_value.chars().count();
                self.update_offset();
                true
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if c.is_control() {
                    return true;
                }
                insert_char_at(&mut self.edit_value, self.edit_cursor, c);
                self.edit_cursor += 1;
                self.update_offset();
                true
            }
            _ => false,
        }
    }

    pub fn cursor_position(&self) -> Option<(u16, u16)> {
        if !self.editing {
            return None;
        }
        let area = self.last_area.get()?;
        let x = area.x
            + PREFIX.chars().count() as u16
            + (self.edit_cursor.saturating_sub(self.edit_offset)) as u16;
        Some((x, area.y))
    }

    fn update_offset(&mut self) {
        let Some(area) = self.last_area.get() else {
            return;
        };
        let avail = (area.width as usize).saturating_sub(PREFIX.chars().count());
        if avail == 0 {
            return;
        }
        if self.edit_cursor < self.edit_offset {
            self.edit_offset = self.edit_cursor;
        } else if self.edit_cursor >= self.edit_offset + avail {
            self.edit_offset = self.edit_cursor - avail + 1;
        }
    }

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        active_tab: usize,
        active_url: &str,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        self.last_area.set(Some(area));

        let editing = self.editing;
        let loading_here =
            !editing && self.stage.is_loading() && self.loading_tab == active_tab;

        if loading_here {
            self.render_split(area, buf, theme);
        } else {
            let url = if editing {
                self.edit_value.as_str()
            } else {
                active_url
            };
            self.render_full(area, buf, url, editing, theme);
        }
    }

    fn render_full(
        &self,
        area: Rect,
        buf: &mut Buffer,
        url: &str,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        let (fg, bg) = if focused {
            (theme.selection.fg, theme.selection.bg)
        } else {
            (theme.modal_fg, theme.modal_bg)
        };
        let style = Style::default().fg(fg).bg(bg);

        for x in area.x..area.right() {
            buf[(x, area.y)].set_char(' ').set_style(style);
        }

        let prefix_len = PREFIX.chars().count();
        let avail = (area.width as usize).saturating_sub(prefix_len);

        let visible: String = if focused {
            url.chars()
                .skip(self.edit_offset)
                .take(avail)
                .collect()
        } else {
            truncate_end(url, avail)
        };

        let line = format!("{}{}", PREFIX, visible);
        buf.set_string(area.x, area.y, &line, style);
    }

    fn render_split(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        let w = area.width;
        if w < 12 {
            self.render_full(area, buf, &self.loading_url, false, theme);
            return;
        }

        let half = w / 2;
        let left_width = half.saturating_sub(1);
        let right_x_off = half + 1;
        let right_width = w.saturating_sub(right_x_off);

        let style = Style::default().fg(theme.modal_fg).bg(theme.modal_bg);

        for x in area.x..area.right() {
            buf[(x, area.y)].set_char(' ').set_style(style);
        }

        let prefix_len = PREFIX.chars().count() as u16;
        let avail = left_width.saturating_sub(prefix_len);
        let url_text = truncate_end(&self.loading_url, avail as usize);
        let left_line = format!("{}{}", PREFIX, url_text);
        buf.set_string(area.x, area.y, &left_line, style);

        let filled = (self.stage.progress() * right_width as f64).round() as u16;
        let empty = right_width.saturating_sub(filled);

        let filled_style = Style::default()
            .fg(theme.url_bar_color)
            .bg(theme.modal_bg);
        let empty_style = Style::default()
            .fg(Color::DarkGray)
            .bg(theme.modal_bg);

        let right_x = area.x + right_x_off;

        let filled_str: String = std::iter::repeat('━').take(filled as usize).collect();
        let empty_str: String = std::iter::repeat('─').take(empty as usize).collect();

        buf.set_string(right_x, area.y, &filled_str, filled_style);
        buf.set_string(right_x + filled, area.y, &empty_str, empty_style);
    }
}

impl Default for UrlBar {
    fn default() -> Self {
        Self::new()
    }
}

fn truncate_end(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        return s.to_string();
    }
    if max <= 3 {
        return chars.iter().take(max).collect();
    }
    let mut out: String = chars[..max - 3].iter().collect();
    out.push_str("...");
    out
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