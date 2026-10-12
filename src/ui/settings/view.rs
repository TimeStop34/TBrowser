use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
};

use crate::core::settings::{
    ItemDef, NodeDef, ParamDef, ParamKind, Settings, Value, CATEGORIES,
};
use crate::ui::{inner_area as base_inner, render_border};
use crate::ui::settings::SettingsStyle;
use crate::ui::GlobalTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsColumn {
    Categories,
    Params,
    Description,
}

impl SettingsColumn {
    pub fn next(self) -> Self {
        match self {
            SettingsColumn::Categories => SettingsColumn::Params,
            SettingsColumn::Params => SettingsColumn::Description,
            SettingsColumn::Description => SettingsColumn::Categories,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            SettingsColumn::Categories => SettingsColumn::Description,
            SettingsColumn::Params => SettingsColumn::Categories,
            SettingsColumn::Description => SettingsColumn::Params,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum EditKind {
    Int,
    Float,
    String,
    Keybind,
}

pub struct EditRequest {
    pub key: String,
    pub current: String,
    pub title: String,
    pub kind: EditKind,
}

struct FlatNode {
    node: &'static NodeDef,
    depth: u8,
    expanded: bool,
    has_group_children: bool,
}

pub struct SettingsView {
    pub settings: Settings,
    pub style: SettingsStyle,
    pub column: SettingsColumn,
    pub active_key: &'static str,
    pub expanded: HashSet<&'static str>,
    pub active_param: usize,
    pub active_variant: usize,
    scroll_categories: u16,
    scroll_params: u16,
    scroll_variants: u16,
}

impl SettingsView {
    pub fn new() -> Self {
        let settings = Settings::load();
        let first_key = CATEGORIES
            .iter()
            .find(|n| (n.active)(&settings))
            .map(|n| n.key)
            .unwrap_or("");
        Self {
            settings,
            style: SettingsStyle::default(),
            column: SettingsColumn::Categories,
            active_key: first_key,
            expanded: HashSet::new(),
            active_param: 0,
            active_variant: 0,
            scroll_categories: 0,
            scroll_params: 0,
            scroll_variants: 0,
        }
    }

    // ========================================================
    // Обход дерева
    // ========================================================

    fn flat_visible_nodes(&self) -> Vec<FlatNode> {
        let mut out = Vec::new();
        for node in CATEGORIES.iter() {
            self.collect_flat(node, 0, &mut out);
        }
        out
    }

    fn collect_flat(&self, node: &'static NodeDef, depth: u8, out: &mut Vec<FlatNode>) {
        if !(node.active)(&self.settings) {
            return;
        }
        let has_group_children = node
            .children
            .iter()
            .any(|c| matches!(c, ItemDef::Group(_)));
        let expanded = self.expanded.contains(node.key);
        out.push(FlatNode {
            node,
            depth,
            expanded,
            has_group_children,
        });
        if expanded {
            for child in node.children {
                if let ItemDef::Group(g) = child {
                    self.collect_flat(g, depth + 1, out);
                }
            }
        }
    }

    fn visible_params(&self) -> Vec<&'static ParamDef> {
        let Some(node) = self.current_node() else {
            return Vec::new();
        };
        node.children
            .iter()
            .filter_map(|c| match c {
                ItemDef::Param(p) => Some(*p),
                ItemDef::Group(_) => None,
            })
            .filter(|p| (p.active)(&self.settings))
            .collect()
    }

    fn current_node(&self) -> Option<&'static NodeDef> {
        Self::find_node(CATEGORIES, self.active_key)
    }

    fn find_node(list: &'static [NodeDef], key: &str) -> Option<&'static NodeDef> {
        for node in list {
            if let Some(found) = Self::find_node_recursive(node, key) {
                return Some(found);
            }
        }
        None
    }

    fn find_node_recursive(node: &'static NodeDef, key: &str) -> Option<&'static NodeDef> {
        if node.key == key {
            return Some(node);
        }
        for child in node.children {
            if let ItemDef::Group(g) = child {
                if let Some(found) = Self::find_node_recursive(g, key) {
                    return Some(found);
                }
            }
        }
        None
    }

    fn ensure_active_valid(&mut self) {
        let flat = self.flat_visible_nodes();
        if flat.iter().any(|n| n.node.key == self.active_key) {
            return;
        }
        if let Some(first) = flat.first() {
            self.active_key = first.node.key;
            self.active_param = 0;
            self.active_variant = 0;
        }
    }

    // ========================================================
    // Навигация
    // ========================================================

    pub fn up(&mut self) {
        self.ensure_active_valid();
        match self.column {
            SettingsColumn::Categories => {
                let flat = self.flat_visible_nodes();
                let idx = flat
                    .iter()
                    .position(|n| n.node.key == self.active_key)
                    .unwrap_or(0);
                if idx > 0 {
                    self.active_key = flat[idx - 1].node.key;
                    self.active_param = 0;
                    self.active_variant = 0;
                }
            }
            SettingsColumn::Params => {
                if self.active_param > 0 {
                    self.active_param -= 1;
                    self.active_variant = 0;
                }
            }
            SettingsColumn::Description => {
                if self.active_variant > 0 {
                    self.active_variant -= 1;
                }
            }
        }
    }

    pub fn down(&mut self) {
        self.ensure_active_valid();
        match self.column {
            SettingsColumn::Categories => {
                let flat = self.flat_visible_nodes();
                let idx = flat
                    .iter()
                    .position(|n| n.node.key == self.active_key)
                    .unwrap_or(0);
                if idx + 1 < flat.len() {
                    self.active_key = flat[idx + 1].node.key;
                    self.active_param = 0;
                    self.active_variant = 0;
                }
            }
            SettingsColumn::Params => {
                let params = self.visible_params();
                if self.active_param + 1 < params.len() {
                    self.active_param += 1;
                    self.active_variant = 0;
                }
            }
            SettingsColumn::Description => {
                if let Some(n) = self.variants_count() {
                    if self.active_variant + 1 < n {
                        self.active_variant += 1;
                    }
                }
            }
        }
    }

    pub fn left(&mut self) {
        self.column = self.column.prev();
        self.clamp_variant();
    }

    pub fn right(&mut self) {
        self.column = self.column.next();
        self.clamp_variant();
    }

    pub fn enter(&mut self) -> Option<EditRequest> {
        match self.column {
            SettingsColumn::Categories => {
                let Some(node) = self.current_node() else {
                    return None;
                };
                if Self::has_group_children(node) {
                    if self.expanded.contains(node.key) {
                        self.expanded.remove(node.key);
                    } else {
                        self.expanded.insert(node.key);
                    }
                    None
                } else {
                    self.column = SettingsColumn::Params;
                    self.active_param = 0;
                    self.active_variant = 0;
                    self.scroll_params = 0;
                    None
                }
            }
            SettingsColumn::Params => {
                let params = self.visible_params();
                let Some(param) = params.get(self.active_param) else {
                    return None;
                };
                match param.kind {
                    ParamKind::Bool => {
                        self.toggle_bool(param.key);
                        None
                    }
                    ParamKind::Enum(_) | ParamKind::Color(_) => {
                        self.column = SettingsColumn::Description;
                        self.active_variant = self.current_variant_index(param.key);
                        self.scroll_variants = 0;
                        None
                    }
                    ParamKind::Int => {
                        let current = match self.settings.get(param.key) {
                            Some(Value::Int(i)) => i.to_string(),
                            _ => "0".to_string(),
                        };
                        Some(EditRequest {
                            key: param.key.to_string(),
                            current,
                            title: param.label.to_string(),
                            kind: EditKind::Int,
                        })
                    }
                    ParamKind::Float => {
                        let current = match self.settings.get(param.key) {
                            Some(Value::Float(f)) => format!("{}", f),
                            _ => "0.0".to_string(),
                        };
                        Some(EditRequest {
                            key: param.key.to_string(),
                            current,
                            title: param.label.to_string(),
                            kind: EditKind::Float,
                        })
                    }
                    ParamKind::String => {
                        let current = match self.settings.get(param.key) {
                            Some(Value::Str(s)) => s.clone(),
                            _ => String::new(),
                        };
                        Some(EditRequest {
                            key: param.key.to_string(),
                            current,
                            title: param.label.to_string(),
                            kind: EditKind::String,
                        })
                    }
                    ParamKind::Keybind => {
                        let current = match self.settings.get(param.key) {
                            Some(Value::Str(s)) => s.clone(),
                            _ => String::new(),
                        };
                        Some(EditRequest {
                            key: param.key.to_string(),
                            current,
                            title: param.label.to_string(),
                            kind: EditKind::Keybind,
                        })
                    }
                }
            }
            SettingsColumn::Description => {
                self.commit_variant();
                None
            }
        }
    }

    pub fn apply_edit(&mut self, key: &str, value: &str, kind: EditKind) {
        match kind {
            EditKind::Int => {
                if let Ok(n) = value.parse::<i64>() {
                    if let Some(v) = self.settings.get_mut(key) {
                        *v = Value::Int(n);
                    }
                }
            }
            EditKind::Float => {
                let normalized = value.replace(',', ".");
                if let Ok(n) = normalized.parse::<f64>() {
                    if let Some(v) = self.settings.get_mut(key) {
                        *v = Value::Float(n);
                    }
                }
            }
            EditKind::String => {
                if let Some(v) = self.settings.get_mut(key) {
                    *v = Value::Str(value.to_string());
                }
            }
            EditKind::Keybind => {
                if !value.is_empty() {
                    if let Some(v) = self.settings.get_mut(key) {
                        *v = Value::Str(value.to_string());
                    }
                }
            }
        }
        let _ = self.settings.save();
    }

    // ========================================================
    // Хелперы
    // ========================================================

    fn has_group_children(node: &NodeDef) -> bool {
        node.children
            .iter()
            .any(|c| matches!(c, ItemDef::Group(_)))
    }

    fn clamp_variant(&mut self) {
        if let Some(n) = self.variants_count() {
            if self.active_variant >= n {
                self.active_variant = 0;
            }
        } else {
            self.active_variant = 0;
        }
    }

    fn variants_count(&self) -> Option<usize> {
        let params = self.visible_params();
        let param = params.get(self.active_param)?;
        match param.kind {
            ParamKind::Enum(opts) | ParamKind::Color(opts) => Some(opts.len()),
            _ => None,
        }
    }

    fn current_variant_index(&self, key: &str) -> usize {
        match self.settings.get(key) {
            Some(Value::EnumIndex(i)) => *i,
            _ => 0,
        }
    }

    fn toggle_bool(&mut self, key: &str) {
        if let Some(Value::Bool(b)) = self.settings.get_mut(key) {
            *b = !*b;
        }
        let _ = self.settings.save();
    }

    fn commit_variant(&mut self) {
        let params = self.visible_params();
        let Some(param) = params.get(self.active_param) else {
            return;
        };
        if matches!(param.kind, ParamKind::Enum(_) | ParamKind::Color(_)) {
            if let Some(v) = self.settings.get_mut(param.key) {
                *v = Value::EnumIndex(self.active_variant);
            }
        }
        let _ = self.settings.save();
        self.column = SettingsColumn::Params;
    }

    // ========================================================
    // Рендер
    // ========================================================

    pub fn render(
        &mut self,
        area: Rect,
        buf: &mut Buffer,
        menu_focused: bool,
        theme: &GlobalTheme,
    ) {
        if area.width < 20 || area.height < 5 {
            return;
        }

        self.ensure_active_valid();

        let [cat_area, par_area, desc_area] = Layout::horizontal([
            Constraint::Percentage(25),
            Constraint::Percentage(40),
            Constraint::Percentage(35),
        ])
        .areas(area);

        let cat_focused = !menu_focused && self.column == SettingsColumn::Categories;
        let par_focused = !menu_focused && self.column == SettingsColumn::Params;
        let desc_focused = !menu_focused && self.column == SettingsColumn::Description;

        let cat_inner = base_inner(cat_area);
        let par_inner = base_inner(par_area);
        let desc_inner = base_inner(desc_area);

        let flat = self.flat_visible_nodes();
        let idx = flat
            .iter()
            .position(|n| n.node.key == self.active_key)
            .unwrap_or(0);
        self.scroll_categories = clamp_scroll(
            self.scroll_categories,
            idx,
            cat_inner.height,
            flat.len(),
        );

        let params = self.visible_params();
        self.scroll_params = clamp_scroll(
            self.scroll_params,
            self.active_param,
            par_inner.height,
            params.len(),
        );

        let variants_viewport = self.variants_viewport(desc_inner);
        let variants_total = self.variants_count().unwrap_or(0);
        self.scroll_variants = clamp_scroll(
            self.scroll_variants,
            self.active_variant,
            variants_viewport,
            variants_total,
        );

        self.render_categories(cat_area, buf, cat_focused, theme, &flat);
        self.render_params(par_area, buf, par_focused, theme, &params);
        self.render_description(desc_area, buf, desc_focused, theme, &params);
    }

    fn render_categories(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        theme: &GlobalTheme,
        flat: &[FlatNode],
    ) {
        self.render_frame(area, buf, "Categories", focused, theme);

        let inner = base_inner(area);
        if inner.width == 0 || inner.height == 0 || flat.is_empty() {
            return;
        }

        let start = self.scroll_categories as usize;
        let viewport = inner.height as usize;
        let end = (start + viewport).min(flat.len());

        for (i, item) in flat[start..end].iter().enumerate() {
            let row = inner.y + i as u16;
            let is_active = item.node.key == self.active_key;

            let style = if is_active {
                Style::default()
                    .fg(theme.selection.fg)
                    .bg(theme.selection.bg)
            } else {
                Style::default().fg(self.style.item_fg)
            };

            let marker = if item.has_group_children {
                if item.expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };
            let indent = "  ".repeat(item.depth as usize);
            let label = format!("{}{}{}", indent, marker, item.node.label);

            let max = inner.width as usize;
            let display: String = label.chars().take(max).collect();
            let pad = inner.width.saturating_sub(display.chars().count() as u16);

            buf.set_string(inner.x, row, &display, style);
            if pad > 0 && is_active {
                buf.set_string(
                    inner.x + display.chars().count() as u16,
                    row,
                    &" ".repeat(pad as usize),
                    style,
                );
            }
        }
    }

    fn render_params(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        theme: &GlobalTheme,
        params: &[&'static ParamDef],
    ) {
        self.render_frame(area, buf, "Parameters", focused, theme);

        let inner = base_inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let start = self.scroll_params as usize;
        let viewport = inner.height as usize;
        let end = (start + viewport).min(params.len());

        for (i, param) in params[start..end].iter().enumerate() {
            let row = inner.y + i as u16;
            let real_idx = start + i;
            let is_active = real_idx == self.active_param;
            let base_style = if is_active {
                Style::default()
                    .fg(theme.selection.fg)
                    .bg(theme.selection.bg)
            } else {
                Style::default().fg(self.style.item_fg)
            };

            let value_str = self.format_value(param.key, param.kind);
            let line = format!(" {}: {}", param.label, value_str);
            let max = inner.width as usize;
            let display: String = line.chars().take(max).collect();

            buf.set_string(inner.x, row, &display, base_style);

            if is_active {
                let used = display.chars().count() as u16;
                if used < inner.width {
                    buf.set_string(
                        inner.x + used,
                        row,
                        &" ".repeat((inner.width - used) as usize),
                        base_style,
                    );
                }
            }
        }
    }

    fn render_description(
        &self,
        area: Rect,
        buf: &mut Buffer,
        focused: bool,
        theme: &GlobalTheme,
        params: &[&'static ParamDef],
    ) {
        self.render_frame(area, buf, "Description", focused, theme);

        let inner = base_inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let Some(param) = params.get(self.active_param) else {
            return;
        };

        let mut y = inner.y;
        let desc_lines = wrap_text(param.description, inner.width.saturating_sub(1) as usize);
        for line in &desc_lines {
            if y >= inner.y + inner.height {
                break;
            }
            let padded = format!(" {}", line);
            buf.set_string(
                inner.x,
                y,
                &padded,
                Style::default().fg(self.style.description_fg),
            );
            y += 1;
        }

        if matches!(param.kind, ParamKind::Enum(_) | ParamKind::Color(_))
            && y + 1 < inner.y + inner.height
        {
            y += 1;
            let opts = match param.kind {
                ParamKind::Enum(o) | ParamKind::Color(o) => o,
                _ => &[],
            };
            let current = self.current_variant_index(param.key);

            let start = self.scroll_variants as usize;
            let viewport = (inner.y + inner.height).saturating_sub(y) as usize;
            let end = (start + viewport).min(opts.len());

            for (i, opt) in opts[start..end].iter().enumerate() {
                let row = y + i as u16;
                if row >= inner.y + inner.height {
                    break;
                }
                let real_idx = start + i;
                let is_active = focused && real_idx == self.active_variant;
                let is_current = real_idx == current;
                let mut style = Style::default().fg(self.style.item_fg);
                if is_active {
                    style = Style::default()
                        .fg(theme.selection.fg)
                        .bg(theme.selection.bg);
                }
                let marker = if is_current { "*" } else { " " };
                let line = format!(" {} {}", marker, opt);
                buf.set_string(inner.x, row, &line, style);
            }
        }
    }

    fn render_frame(
        &self,
        area: Rect,
        buf: &mut Buffer,
        title: &str,
        focused: bool,
        theme: &GlobalTheme,
    ) {
        let mut border = theme.border.clone();
        if focused {
            border.fg = theme.accent;
        }
        render_border(area, buf, &border, Some((title, &theme.title)));
    }

    fn variants_viewport(&self, inner: Rect) -> u16 {
        let params = self.visible_params();
        let Some(param) = params.get(self.active_param) else {
            return inner.height;
        };
        if !matches!(param.kind, ParamKind::Enum(_) | ParamKind::Color(_)) {
            return 0;
        }
        let desc_lines =
            wrap_text(param.description, inner.width.saturating_sub(1) as usize).len() as u16;
        inner.height.saturating_sub(desc_lines).saturating_sub(1)
    }

    fn format_value(&self, key: &str, kind: ParamKind) -> String {
        match (self.settings.get(key), kind) {
            (Some(Value::Bool(b)), _) => (if *b { "on" } else { "off" }).to_string(),
            (Some(Value::Int(i)), _) => i.to_string(),
            (Some(Value::Float(f)), _) => format!("{}", f),
            (Some(Value::Str(s)), _) => s.clone(),
            (Some(Value::EnumIndex(i)), ParamKind::Enum(opts) | ParamKind::Color(opts)) => {
                opts.get(*i).copied().unwrap_or("?").to_string()
            }
            _ => "?".to_string(),
        }
    }
}

impl Default for SettingsView {
    fn default() -> Self {
        Self::new()
    }
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![];
    }
    let mut out = Vec::new();
    for raw_line in text.split('\n') {
        let mut line = String::new();
        for word in raw_line.split_whitespace() {
            if line.is_empty() {
                line.push_str(word);
            } else if line.chars().count() + 1 + word.chars().count() <= width {
                line.push(' ');
                line.push_str(word);
            } else {
                out.push(std::mem::take(&mut line));
                line.push_str(word);
            }
        }
        out.push(line);
    }
    out
}

fn clamp_scroll(current: u16, selected: usize, viewport: u16, total: usize) -> u16 {
    if viewport == 0 || total == 0 {
        return 0;
    }
    let v = viewport as usize;
    let mut s = current as usize;

    if selected < s {
        s = selected;
    } else if selected >= s + v {
        s = selected - v + 1;
    }

    let max = total.saturating_sub(v);
    s.min(max) as u16
}

#[allow(unused)]
fn normalize_keybind(input: &str) -> Option<String> {
    let s = input.trim();
    if s.is_empty() {
        return None;
    }

    // Один символ без '+'
    if !s.contains('+') {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() == 1 {
            return Some(s.to_string());
        }
        return None;
    }

    let parts: Vec<&str> = s
        .split('+')
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();

    if parts.len() < 2 {
        return None;
    }

    let mut has_ctrl = false;
    let mut has_shift = false;
    let mut has_alt = false;
    let mut key: Option<String> = None;

    for (i, part) in parts.iter().enumerate() {
        let is_last = i == parts.len() - 1;
        if is_last {
            let chars: Vec<char> = part.chars().collect();
            if chars.len() != 1 {
                return None;
            }
            key = Some((*part).to_string());
        } else {
            match part.to_lowercase().as_str() {
                "ctrl" | "control" => has_ctrl = true,
                "shift" => has_shift = true,
                "alt" => has_alt = true,
                _ => return None,
            }
        }
    }

    let key = key?;

    let mut out = String::new();
    if has_ctrl {
        out.push_str("Ctrl+");
    }
    if has_shift {
        out.push_str("Shift+");
    }
    if has_alt {
        out.push_str("Alt+");
    }
    out.push_str(&key);
    Some(out)
}