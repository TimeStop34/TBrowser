use ratatui::{
    buffer::Buffer,
    crossterm::event::{KeyCode, KeyEvent, KeyModifiers},
    layout::Rect,
    style::Style,
};

use crate::ui::modal::base::{compute_area, fill_background, render_border};
use crate::ui::GlobalTheme;

pub struct KeybindDialog {
    pub is_open: bool,
    pub title: String,
    pub current: String,
    pub width: u16,
    pub height: u16,
    submission: Option<String>,
    cancelled: bool,
}

impl KeybindDialog {
    pub fn new() -> Self {
        Self {
            is_open: false,
            title: "Keybind".to_string(),
            current: String::new(),
            width: 50,
            height: 7,
            submission: None,
            cancelled: false,
        }
    }

    pub fn open(&mut self, current: impl Into<String>, title: impl Into<String>) {
        self.current = current.into();
        self.title = title.into();
        self.is_open = true;
        self.submission = None;
        self.cancelled = false;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    pub fn take_submission(&mut self) -> Option<String> {
        self.submission.take()
    }

    pub fn take_cancelled(&mut self) -> bool {
        let c = self.cancelled;
        self.cancelled = false;
        c
    }

    pub fn compute_area(&self, screen: Rect) -> Rect {
        compute_area(screen, self.width, self.height)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if !self.is_open {
            return false;
        }

        // Esc отменяет — чтобы можно было закрыть диалог
        if key.code == KeyCode::Esc && key.modifiers.is_empty() {
            self.cancelled = true;
            self.close();
            return true;
        }

        // Игнорируем чистые модификаторы
        if matches!(
            key.code,
            KeyCode::Null
                | KeyCode::CapsLock
                | KeyCode::ScrollLock
                | KeyCode::NumLock
                | KeyCode::PrintScreen
                | KeyCode::Pause
                | KeyCode::Menu
                | KeyCode::KeypadBegin
                | KeyCode::Media(_)
                | KeyCode::Modifier(_)
        ) {
            return true;
        }

        let Some(bind) = keyevent_to_string(&key) else {
            return true;
        };

        self.submission = Some(bind);
        self.close();
        true
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer, theme: &GlobalTheme) {
        if area.width < 4 || area.height < 3 {
            return;
        }

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

        // Текущий бинд
        let current_line = format!("Current: {}", self.current);
        buf.set_string(
            inner_x,
            inner_y,
            &current_line,
            Style::default().fg(theme.modal_fg),
        );

        // Подсказка
        let hint = "Press any key combination, Esc to cancel";
        buf.set_string(
            inner_x,
            inner_y + 2,
            hint,
            Style::default().fg(theme.modal_fg),
        );

        // Каретка в конце строки current — мигающий индикатор ожидания
        let caret_x = inner_x + current_line.chars().count() as u16 + 1;
        let caret_style = Style::default()
            .fg(theme.accent)
            .bg(theme.modal_bg);
        buf.set_string(caret_x, inner_y, "_", caret_style);
    }
}

impl Default for KeybindDialog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// KeyEvent → String
// ============================================================

pub fn keyevent_to_string(key: &KeyEvent) -> Option<String> {
    let key_part = match &key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "Enter".to_string(),
        KeyCode::Tab => "Tab".to_string(),
        KeyCode::BackTab => return None,
        KeyCode::Backspace => "Backspace".to_string(),
        KeyCode::Delete => "Delete".to_string(),
        KeyCode::Insert => "Insert".to_string(),
        KeyCode::Home => "Home".to_string(),
        KeyCode::End => "End".to_string(),
        KeyCode::PageUp => "PageUp".to_string(),
        KeyCode::PageDown => "PageDown".to_string(),
        KeyCode::Up => "Up".to_string(),
        KeyCode::Down => "Down".to_string(),
        KeyCode::Left => "Left".to_string(),
        KeyCode::Right => "Right".to_string(),
        KeyCode::Esc => "Esc".to_string(),
        KeyCode::F(n) => format!("F{}", n),
        _ => return None,
    };

    let mut out = String::new();
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        out.push_str("Ctrl+");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        out.push_str("Shift+");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        out.push_str("Alt+");
    }
    out.push_str(&key_part);
    Some(out)
}