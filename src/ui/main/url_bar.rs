use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
};

use crate::ui::GlobalTheme;

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
}

impl UrlBar {
    pub fn new() -> Self {
        Self {
            stage: LoadStage::Idle,
            loading_url: String::new(),
            loading_tab: 0,
            generation: 0,
        }
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

    pub fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        active_tab: usize,
        active_url: &str,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        let loading_here = self.stage.is_loading() && self.loading_tab == active_tab;

        if loading_here {
            self.render_split(area, buf, focused, theme);
        } else {
            self.render_full(area, buf, active_url, focused, theme);
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

        let prefix = "  URL: ";
        let prefix_len = prefix.chars().count();
        let avail = (area.width as usize).saturating_sub(prefix_len);
        let url_text = truncate_end(url, avail);
        let line = format!("{}{}", prefix, url_text);

        buf.set_string(area.x, area.y, &line, style);
    }

    fn render_split(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        let w = area.width;
        if w < 12 {
            self.render_full(area, buf, &self.loading_url, focused, theme);
            return;
        }

        let half = w / 2;
        let left_width = half.saturating_sub(1);
        let right_x_off = half + 1;
        let right_width = w.saturating_sub(right_x_off);

        let (fg, bg) = if focused {
            (theme.selection.fg, theme.selection.bg)
        } else {
            (theme.modal_fg, theme.modal_bg)
        };
        let style = Style::default().fg(fg).bg(bg);

        for x in area.x..area.right() {
            buf[(x, area.y)].set_char(' ').set_style(style);
        }

        let prefix = "  URL: ";
        let prefix_len = prefix.chars().count() as u16;
        let avail = left_width.saturating_sub(prefix_len);
        let url_text = truncate_end(&self.loading_url, avail as usize);
        let left_line = format!("{}{}", prefix, url_text);
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