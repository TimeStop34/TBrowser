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

const RESET_KEY_PREFIX: &str = "__reset__";

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

fn reset_param_active(_: &Settings) -> bool {
    true
}

static RESET_PARAM: ParamDef = ParamDef {
    key: "reset",
    label: "Reset",
    description: "Reset all settings in this group to their default values.",
    kind: ParamKind::Bool,
    active: reset_param_active,
    default: crate::core::settings::DefaultValue::Bool(false),
};

#[derive(Clone)]
struct FlatNode {
    key: String,
    label: String,
    depth: u8,
    expanded: bool,
    has_group_children: bool,
    is_reset: bool,
}

pub struct SettingsView {
    pub settings: Settings,
    pub style: SettingsStyle,
    pub column: SettingsColumn,
    pub active_key: String,
    pub expanded: HashSet<String>,
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
            .map(|n| n.key.to_string())
            .unwrap_or_default();
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
        let has_group_children = node.children.iter().any(|c| matches!(c, ItemDef::Group(_)));
        let has_params = node.children.iter().any(|c| matches!(c, ItemDef::Param(_)));
        let expandable = has_group_children || has_params;
        let expanded = self.expanded.contains(node.key);
        out.push(FlatNode {
            key: node.key.to_string(),
            label: node.label.to_string(),
            depth,
            expanded,
            has_group_children: expandable,
            is_reset: false,
        });
        if expanded {
            for child in node.children {
                if let ItemDef::Group(g) = child {
                    self.collect_flat(g, depth + 1, out);
                }
            }
            out.push(FlatNode {
                key: format!("{}{}", RESET_KEY_PREFIX, node.key),
                label: "Reset".to_string(),
                depth: depth + 1,
                expanded: false,
                has_group_children: false,
                is_reset: true,
            });
        }
    }

    fn current_flat(&self) -> Option<FlatNode> {
        self.flat_visible_nodes()
            .into_iter()
            .find(|n| n.key == self.active_key)
    }

    #[allow(unused)]
    fn current_is_reset(&self) -> bool {
        self.active_key.starts_with(RESET_KEY_PREFIX)
    }

    fn visible_params(&self) -> Vec<(&'static ParamDef, String)> {
        let Some(item) = self.current_flat() else {
            return Vec::new();
        };
        if item.is_reset {
            return vec![(&RESET_PARAM, self.active_key.clone())];
        }
        let Some(node) = Self::find_node(CATEGORIES, &self.active_key) else {
            return Vec::new();
        };
        node.children
            .iter()
            .filter_map(|c| match c {
                ItemDef::Param(p) => Some(*p),
                ItemDef::Group(_) => None,
            })
            .filter(|p| (p.active)(&self.settings))
            .map(|p| (p, full_param_key(node, p)))
            .collect()
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

    fn find_label(key: &str) -> Option<&'static str> {
        Self::find_node(CATEGORIES, key).map(|n| n.label)
    }

    fn ensure_active_valid(&mut self) {
        let flat = self.flat_visible_nodes();
        if flat.iter().any(|n| n.key == self.active_key) {
            return;
        }
        if let Some(first) = flat.first() {
            self.active_key = first.key.clone();
            self.active_param = 0;
            self.active_variant = 0;
        }
    }

    pub fn up(&mut self) {
        self.ensure_active_valid();
        match self.column {
            SettingsColumn::Categories => {
                let flat = self.flat_visible_nodes();
                let idx = flat
                    .iter()
                    .position(|n| n.key == self.active_key)
                    .unwrap_or(0);
                if idx > 0 {
                    self.active_key = flat[idx - 1].key.clone();
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
                    .position(|n| n.key == self.active_key)
                    .unwrap_or(0);
                if idx + 1 < flat.len() {
                    self.active_key = flat[idx + 1].key.clone();
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
                let Some(item) = self.current_flat() else {
                    return None;
                };
                if item.is_reset {
                    self.column = SettingsColumn::Params;
                    self.active_param = 0;
                    self.active_variant = 0;
                    self.scroll_params = 0;
                    return None;
                }
                if item.has_group_children {
                    let k = item.key.clone();
                    if self.expanded.contains(&k) {
                        self.expanded.remove(&k);
                    } else {
                        self.expanded.insert(k);
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
                let Some((param, full_key)) = params.get(self.active_param) else {
                    return None;
                };

                self.column = SettingsColumn::Description;
                self.active_variant = self.initial_variant_index(param, full_key);
                self.scroll_variants = 0;
                None
            }
            SettingsColumn::Description => self.activate_description_item(),
        }
    }

    fn initial_variant_index(&self, param: &'static ParamDef, full_key: &str) -> usize {
        if full_key.starts_with(RESET_KEY_PREFIX) {
            return 1; // Off
        }
        match param.kind {
            ParamKind::Bool => match self.settings.get(full_key) {
                Some(Value::Bool(true)) => 0,
                Some(Value::Bool(false)) => 1,
                _ => 0,
            },
            ParamKind::Enum(_) | ParamKind::Color(_) => match self.settings.get(full_key) {
                Some(Value::EnumIndex(i)) => *i,
                _ => 0,
            },
            _ => 0,
        }
    }

    fn activate_description_item(&mut self) -> Option<EditRequest> {
        let params = self.visible_params();
        let Some((param, full_key)) = params.get(self.active_param) else {
            return None;
        };

        if let Some(parent_key) = full_key.strip_prefix(RESET_KEY_PREFIX) {
            if self.active_variant == 0 {
                self.reset_group(parent_key);
            }
            self.column = SettingsColumn::Params;
            return None;
        }

        match param.kind {
            ParamKind::Bool => {
                match self.active_variant {
                    0 => {
                        if let Some(v) = self.settings.get_mut(full_key) {
                            *v = Value::Bool(true);
                        }
                    }
                    1 => {
                        if let Some(v) = self.settings.get_mut(full_key) {
                            *v = Value::Bool(false);
                        }
                    }
                    2 => {
                        if let Some(v) = self.settings.get_mut(full_key) {
                            *v = param.default.to_value();
                        }
                    }
                    _ => {}
                }
                let _ = self.settings.save();
                self.column = SettingsColumn::Params;
                None
            }
            ParamKind::Enum(opts) | ParamKind::Color(opts) => {
                if self.active_variant == opts.len() {
                    if let Some(v) = self.settings.get_mut(full_key) {
                        *v = param.default.to_value();
                    }
                } else if let Some(v) = self.settings.get_mut(full_key) {
                    *v = Value::EnumIndex(self.active_variant);
                }
                let _ = self.settings.save();
                self.column = SettingsColumn::Params;
                None
            }
            ParamKind::Int | ParamKind::Float | ParamKind::String | ParamKind::Keybind => {
                match self.active_variant {
                    0 => {
                        let kind = match param.kind {
                            ParamKind::Int => EditKind::Int,
                            ParamKind::Float => EditKind::Float,
                            ParamKind::String => EditKind::String,
                            ParamKind::Keybind => EditKind::Keybind,
                            _ => return None,
                        };
                        let current = match self.settings.get(full_key) {
                            Some(Value::Int(i)) => i.to_string(),
                            Some(Value::Float(f)) => format!("{}", f),
                            Some(Value::Str(s)) => s.clone(),
                            _ => String::new(),
                        };
                        Some(EditRequest {
                            key: full_key.clone(),
                            current,
                            title: param.label.to_string(),
                            kind,
                        })
                    }
                    1 => {
                        if let Some(v) = self.settings.get_mut(full_key) {
                            *v = param.default.to_value();
                        }
                        let _ = self.settings.save();
                        self.column = SettingsColumn::Params;
                        None
                    }
                    _ => None,
                }
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

    fn reset_group(&mut self, group_key: &str) {
        let Some(node) = Self::find_node(CATEGORIES, group_key) else {
            return;
        };
        Self::reset_node(&mut self.settings, node);
        let _ = self.settings.save();
    }

    fn reset_node(settings: &mut Settings, node: &'static NodeDef) {
        for child in node.children {
            match child {
                ItemDef::Param(p) => {
                    let full = if node.key.is_empty() {
                        p.key.to_string()
                    } else {
                        format!("{}.{}", node.key, p.key)
                    };
                    if let Some(v) = settings.get_mut(&full) {
                        *v = p.default.to_value();
                    }
                }
                ItemDef::Group(g) => Self::reset_node(settings, g),
            }
        }
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
        let (param, full_key) = params.get(self.active_param)?;
        if full_key.starts_with(RESET_KEY_PREFIX) {
            return Some(2); // On / Off
        }
        match param.kind {
            ParamKind::Bool => Some(3),
            ParamKind::Enum(opts) | ParamKind::Color(opts) => Some(opts.len() + 1),
            ParamKind::Int | ParamKind::Float | ParamKind::String | ParamKind::Keybind => Some(2),
        }
    }

    fn current_variant_index(&self, full_key: &str) -> Option<usize> {
        match self.settings.get(full_key) {
            Some(Value::EnumIndex(i)) => Some(*i),
            _ => None,
        }
    }

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
            .position(|n| n.key == self.active_key)
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
            let is_active = item.key == self.active_key;

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

            let item_label = if item.is_reset {
                theme.button.format.replace("{}", "Reset")
            } else {
                item.label.clone()
            };

            let label = format!("{}{}{}", indent, marker, item_label);

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
        params: &[(&'static ParamDef, String)],
    ) {
        self.render_frame(area, buf, "Parameters", focused, theme);

        let inner = base_inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let start = self.scroll_params as usize;
        let viewport = inner.height as usize;
        let end = (start + viewport).min(params.len());

        for (i, (param, full_key)) in params[start..end].iter().enumerate() {
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

            let line = if let Some(parent_key) = full_key.strip_prefix(RESET_KEY_PREFIX) {
                let parent_label = Self::find_label(parent_key).unwrap_or("");
                format!(" Reset settings of the {} group", parent_label)
            } else {
                let value_str = self.format_value(full_key, param.kind);
                format!(" {}: {}", param.label, value_str)
            };

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
        params: &[(&'static ParamDef, String)],
    ) {
        self.render_frame(area, buf, "Description", focused, theme);

        let inner = base_inner(area);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let Some((param, full_key)) = params.get(self.active_param) else {
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

        if y + 1 >= inner.y + inner.height {
            return;
        }
        y += 1;

        let is_reset = full_key.starts_with(RESET_KEY_PREFIX);

        let items: Vec<(String, bool)> = if is_reset {
            vec![
                ("On".to_string(), false),
                ("Off".to_string(), true),
            ]
        } else {
            match param.kind {
                ParamKind::Bool => {
                    let cur = self.settings.get(full_key);
                    vec![
                        ("On".to_string(), matches!(cur, Some(Value::Bool(true)))),
                        ("Off".to_string(), matches!(cur, Some(Value::Bool(false)))),
                        ("< Reset >".to_string(), false),
                    ]
                }
                ParamKind::Enum(opts) | ParamKind::Color(opts) => {
                    let cur = self.current_variant_index(full_key);
                    let mut v: Vec<(String, bool)> = opts
                        .iter()
                        .enumerate()
                        .map(|(i, s)| (s.to_string(), Some(i) == cur))
                        .collect();
                    v.push(("< Reset >".to_string(), false));
                    v
                }
                ParamKind::Int | ParamKind::Float | ParamKind::String | ParamKind::Keybind => {
                    vec![
                        ("< Edit >".to_string(), false),
                        ("< Reset >".to_string(), false),
                    ]
                }
            }
        };

        let total = items.len();
        let start = self.scroll_variants as usize;
        let viewport = (inner.y + inner.height).saturating_sub(y) as usize;
        let end = (start + viewport).min(total);

        for real_idx in start..end {
            let row = y + (real_idx - start) as u16;
            if row >= inner.y + inner.height {
                break;
            }
            let is_active = focused && real_idx == self.active_variant;

            let mut style = Style::default().fg(self.style.item_fg);
            if is_active {
                style = Style::default()
                    .fg(theme.selection.fg)
                    .bg(theme.selection.bg);
            }

            let (text, is_current) = &items[real_idx];
            let line = if text.starts_with('<') {
                format!("   {}", text)
            } else {
                let marker = if *is_current { "*" } else { " " };
                format!(" {} {}", marker, text)
            };
            buf.set_string(inner.x, row, &line, style);
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
        let Some((param, _)) = params.get(self.active_param) else {
            return inner.height;
        };
        let desc_lines =
            wrap_text(param.description, inner.width.saturating_sub(1) as usize).len() as u16;
        inner.height.saturating_sub(desc_lines).saturating_sub(1)
    }

    fn format_value(&self, full_key: &str, kind: ParamKind) -> String {
        match (self.settings.get(full_key), kind) {
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

fn full_param_key(node: &NodeDef, param: &ParamDef) -> String {
    if node.key.is_empty() {
        param.key.to_string()
    } else {
        format!("{}.{}", node.key, param.key)
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