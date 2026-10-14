use std::sync::Arc;
use std::time::Duration;

use image::GenericImageView;
use indextree::NodeId;
use tokio::sync::{mpsc, RwLock};
use url::Url;

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span as RatSpan, Text},
};

mod core;
mod guard;
mod network;
mod ui;

use core::bookmarks::Bookmarks;
use core::info::build_info_text;
use core::parser::get_parser;
use core::renderer::{get_renderer, ImageState, RenderedContent, RenderState};
use core::settings::Settings;
use core::{
    build_error_page, Config, Document, FieldKind, FieldValue, FormField, FormItem,
    InteractiveFieldKind, LineKind, NodeData, Tab, TabManager,
};
use guard::TuiGuard;
use network::base::FetchOptions;
use network::factory::{get_post_protocol, get_protocol};
use ui::{
    ButtonCallback, ConfirmDialog, EditKind, GlobalTheme, Header, InputDialog, KeybindDialog,
    KeybindHint, LoadStage, MainFocus, MainSpace, Menu, MenuButton, Scrollbar, SettingsColumn,
    SettingsView, Sidebar, SidebarMenuButton, SidebarMode, TextareaDialog, UrlBar,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NavigateMode {
    Push,
    NewUrl,
    History,
    Reload,
}

#[derive(Clone)]
#[allow(unused)]
enum Action {
    Quit,
    ToggleSidebar(SidebarMode),
    OpenSettings,
    CloseSettings,
    HideHint { token: u64 },
    LoadUrl { tab_index: usize, url: String, mode: NavigateMode },
    LoadStage { tab_index: usize, url: String, stage: LoadStage },
    LoadHide { generation: u64 },
    Loaded {
        tab_index: usize,
        url: String,
        document: Arc<RwLock<Document>>,
        rendered: RenderedContent,
        generation: u64,
        viewport_width: u16,
    },
    LoadFailed {
        tab_index: usize,
        url: String,
        error: String,
        generation: u64,
    },
    FormError {
        tab_index: usize,
        form_id: NodeId,
        message: String,
        generation: u64,
    },
    ImageLoaded {
        tab_index: usize,
        node_id: NodeId,
        result: Result<Vec<Line<'static>>, String>,
        generation: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Screen {
    Main,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsFocus {
    Menu,
    Column,
}

struct InlineEdit {
    form_id: NodeId,
    field_index: usize,
    value: String,
    cursor: usize,
    original: String,
    label: String,
    is_password: bool,
}

fn validate_email(email: &str) -> bool {
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

fn make_menu_button(label: &str, tx: mpsc::Sender<Action>, action: Action) -> MenuButton {
    let cb: ButtonCallback = Arc::new(move || {
        let tx = tx.clone();
        let action = action.clone();
        Box::pin(async move {
            let _ = tx.send(action).await;
        })
    });
    (label.to_string(), cb)
}

fn matches_keybind(key: &KeyEvent, bind: &str) -> bool {
    if bind.is_empty() {
        return false;
    }
    let parts: Vec<&str> = bind.split('+').collect();
    let Some(&key_part) = parts.last() else {
        return false;
    };
    let mods = &parts[..parts.len() - 1];
    let want_ctrl = mods.iter().any(|m| *m == "Ctrl");
    let want_shift = mods.iter().any(|m| *m == "Shift");
    let want_alt = mods.iter().any(|m| *m == "Alt");
    if want_ctrl != key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    if want_shift != key.modifiers.contains(KeyModifiers::SHIFT) {
        return false;
    }
    if want_alt != key.modifiers.contains(KeyModifiers::ALT) {
        return false;
    }
    match key_part {
        "Enter" => key.code == KeyCode::Enter,
        "Tab" => key.code == KeyCode::Tab,
        "Backspace" => key.code == KeyCode::Backspace,
        "Delete" => key.code == KeyCode::Delete,
        "Insert" => key.code == KeyCode::Insert,
        "Home" => key.code == KeyCode::Home,
        "End" => key.code == KeyCode::End,
        "PageUp" => key.code == KeyCode::PageUp,
        "PageDown" => key.code == KeyCode::PageDown,
        "Up" => key.code == KeyCode::Up,
        "Down" => key.code == KeyCode::Down,
        "Left" => key.code == KeyCode::Left,
        "Right" => key.code == KeyCode::Right,
        "Esc" => key.code == KeyCode::Esc,
        other => {
            if let Some(n_str) = other.strip_prefix('F') {
                if let Ok(n) = n_str.parse::<u8>() {
                    return key.code == KeyCode::F(n);
                }
            }
            let chars: Vec<char> = other.chars().collect();
            if chars.len() != 1 {
                return false;
            }
            matches!(key.code, KeyCode::Char(c) if c == chars[0])
        }
    }
}

fn build_special_page(url: &str) -> Option<Document> {
    match url {
        "about:blank" => Some(Document::new("text/plain")),
        "about:new" => {
            let mut doc = Document::new("text/plain");
            doc.title = "New Tab".to_string();
            doc.add_paragraph("New Tab");
            doc.add_paragraph("");
            doc.add_paragraph("Enter a URL via Open, or pick a bookmark.");
            Some(doc)
        }
        _ => None,
    }
}

fn resolve_image_url(base_url: &str, src: &str) -> String {
    if src.contains("://") {
        return src.to_string();
    }
    if let Ok(base) = Url::parse(base_url) {
        if base.scheme() == "tmutp" {
            let host = base.host_str().unwrap_or("localhost");
            let new_path = if src.starts_with('/') {
                src.to_string()
            } else {
                let dir = base
                    .path()
                    .rsplit_once('/')
                    .map(|(d, _)| d)
                    .unwrap_or("");
                format!("{}/{}", dir, src)
            };
            return format!("tbip://{}{}", host, new_path);
        }
        if let Ok(joined) = base.join(src) {
            return joined.to_string();
        }
    }
    src.to_string()
}

async fn load_image_ascii(
    base_url: &str,
    src: &str,
    height_attr: Option<u16>,
    viewport_height: u16,
    timeout_secs: u64,
) -> Result<Vec<Line<'static>>, String> {
    let full_url = resolve_image_url(base_url, src);

    let parsed = Url::parse(&full_url).map_err(|e| e.to_string())?;
    let scheme = parsed.scheme();

    let client = get_protocol(scheme).ok_or_else(|| format!("unsupported scheme: {}", scheme))?;

    let options = FetchOptions {
        follow_redirects: true,
        max_redirects: 10,
    };

    let response = tokio::time::timeout(
        Duration::from_secs(timeout_secs.max(1)),
        client.fetch(&full_url, None, &options),
    )
    .await
    .map_err(|_| format!("timed out after {}s", timeout_secs))?
    .map_err(|e| e.to_string())?;

    let img = image::load_from_memory(&response.body).map_err(|e| e.to_string())?;
    let (src_w, src_h) = img.dimensions();
    if src_w == 0 || src_h == 0 {
        return Err("empty image".to_string());
    }

    let target_height_chars: u32 = match height_attr {
        Some(h) if h > 0 => h as u32,
        _ => {
            let h = viewport_height.saturating_sub(6);
            let h = if h > 15 { h - 5 } else { h };
            h.max(5) as u32
        }
    };

    let target_h = target_height_chars.saturating_mul(2).max(2);
    let aspect = src_w as f64 / src_h as f64;
    let target_w = ((target_h as f64) * aspect).round() as u32;
    let target_w = target_w.max(1);

    let resized = img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
    let rgb = resized.to_rgb8();

    let mut lines: Vec<Line<'static>> = Vec::with_capacity(target_height_chars as usize);

    let mut i = 0u32;
    while i < target_h {
        let mut spans: Vec<RatSpan<'static>> = Vec::new();
        let mut x = 0u32;
        while x < target_w {
            let top = rgb.get_pixel(x, i);
            let bot = if i + 1 < target_h {
                rgb.get_pixel(x, i + 1)
            } else {
                top
            };

            let mut run = 1u32;
            while x + run < target_w {
                let t2 = rgb.get_pixel(x + run, i);
                let b2 = if i + 1 < target_h {
                    rgb.get_pixel(x + run, i + 1)
                } else {
                    t2
                };
                if t2 == top && b2 == bot {
                    run += 1;
                } else {
                    break;
                }
            }

            let style = Style::default()
                .fg(Color::Rgb(top[0], top[1], top[2]))
                .bg(Color::Rgb(bot[0], bot[1], bot[2]));
            let text: String = std::iter::repeat('▀').take(run as usize).collect();
            spans.push(RatSpan::styled(text, style));
            x += run;
        }
        lines.push(Line::from(spans));
        i += 2;
    }

    Ok(lines)
}

async fn load_url(
    url: &str,
    tab_index: usize,
    settings: Settings,
    viewport_width: u16,
    timeout_secs: u64,
    follow_redirects: bool,
    action_tx: mpsc::Sender<Action>,
    generation: u64,
) -> anyhow::Result<(Document, RenderedContent)> {
    let empty_state = RenderState::default();

    if let Some(doc) = build_special_page(url) {
        let renderer = get_renderer(&doc.content_type, &settings);
        let rendered = renderer
            .render(&doc, url, viewport_width, &empty_state)
            .await?;
        return Ok((doc, rendered));
    }

    let _ = action_tx
        .send(Action::LoadStage {
            tab_index,
            url: url.to_string(),
            stage: LoadStage::Fetching,
        })
        .await;

    let parsed = Url::parse(url)?;
    let scheme = parsed.scheme();

    let client = get_protocol(scheme)
        .ok_or_else(|| anyhow::anyhow!("unsupported scheme: {}", scheme))?;

    let options = FetchOptions {
        follow_redirects,
        max_redirects: 10,
    };

    let response = tokio::time::timeout(
        Duration::from_secs(timeout_secs.max(1)),
        client.fetch(url, None, &options),
    )
    .await
    .map_err(|_| anyhow::anyhow!("request timed out after {}s", timeout_secs))??;

    let content_type = response
        .headers
        .get("content-type")
        .map(String::as_str)
        .unwrap_or("text/plain")
        .to_string();

    let _ = action_tx
        .send(Action::LoadStage {
            tab_index,
            url: url.to_string(),
            stage: LoadStage::Parsing,
        })
        .await;

    let parser = get_parser(&content_type);
    let doc = parser.parse(&response.text, &content_type).await;

    let _ = action_tx
        .send(Action::LoadStage {
            tab_index,
            url: url.to_string(),
            stage: LoadStage::Rendering,
        })
        .await;

    let renderer = get_renderer(&content_type, &settings);
    let rendered = renderer
        .render(&doc, url, viewport_width, &empty_state)
        .await?;

    let _ = generation;

    Ok((doc, rendered))
}

fn spawn_hint(hint: &mut KeybindHint, text: &str, duration_ms: u64, tx: mpsc::Sender<Action>) {
    let token = hint.show(text);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        let _ = tx.send(Action::HideHint { token }).await;
    });
}

async fn rerender_tab(tab: &mut Tab, settings: &Settings, width: u16) {
    let Some(doc_arc) = tab.document.clone() else {
        tab.rendered_width = width;
        return;
    };
    let doc = doc_arc.read().await;
    let renderer = get_renderer(&doc.content_type, settings);
    let url = tab.url.clone();
    let state = tab.interactive_state.clone();
    let result = renderer.render(&doc, &url, width, &state).await;
    drop(doc);
    if let Ok(rendered) = result {
        let saved_scroll = tab.content.scroll;
        let saved_selected = tab.content.selected;
        tab.content.text = rendered.text;
        tab.content.meta = rendered.meta;
        let max = tab.content.text.lines.len().saturating_sub(1) as u16;
        tab.content.scroll = saved_scroll.min(max);
        tab.content.selected = saved_selected.min(max);
    }
    tab.rendered_width = width;
}

async fn do_form_reset(
    tabs: &mut TabManager,
    tab_index: usize,
    form_id: NodeId,
    settings: &Settings,
    viewport_width: u16,
) {
    let items: Vec<FormItem> = match tabs.get(tab_index) {
        Some(tab) => match &tab.document {
            Some(arc) => {
                let guard = arc.read().await;
                match guard.find_form(form_id) {
                    Some((_, _, items)) => items.to_vec(),
                    None => return,
                }
            }
            None => return,
        },
        None => return,
    };

    if let Some(tab) = tabs.get_mut(tab_index) {
        let mut field_index = 0usize;
        for item in &items {
            if let FormItem::Field(field) = item {
                let key = (form_id, field_index);
                tab.interactive_state.field_errors.remove(&key);
                tab.interactive_state.select_open.remove(&key);
                match field {
                    FormField::Text { .. } | FormField::Textarea { .. } => {
                        tab.interactive_state.fields.remove(&key);
                    }
                    FormField::Checkbox { checked, .. } => {
                        tab.interactive_state
                            .fields
                            .insert(key, FieldValue::Checked(*checked));
                    }
                    FormField::Radio { options, .. } => {
                        let idx = options.iter().position(|o| o.checked).unwrap_or(0);
                        tab.interactive_state
                            .fields
                            .insert(key, FieldValue::Index(idx));
                    }
                    FormField::Select { options, .. } => {
                        let idx = options.iter().position(|o| o.selected).unwrap_or(0);
                        tab.interactive_state
                            .fields
                            .insert(key, FieldValue::Index(idx));
                    }
                    _ => {}
                }
                field_index += 1;
            }
        }
        tab.interactive_state.form_errors.remove(&form_id);
    }

    if let Some(tab) = tabs.get_mut(tab_index) {
        rerender_tab(tab, settings, viewport_width).await;
    }
}

fn inline_cursor_xy(
    text: &Text<'static>,
    content_area: Rect,
    scroll: u16,
    selected: u16,
    prefix_cols: usize,
    visible_cursor: usize,
) -> Option<(u16, u16)> {
    let w = content_area.width as usize;
    if w == 0 {
        return None;
    }

    let mut y_offset = 0usize;
    for i in (scroll as usize)..(selected as usize) {
        if let Some(line) = text.lines.get(i) {
            let chars: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
            let h = if chars == 0 { 1 } else { (chars + w - 1) / w };
            y_offset += h;
        }
    }

    let line = text.lines.get(selected as usize)?;
    let total: usize = line.spans.iter().map(|s| s.content.chars().count()).sum();
    let pos = (prefix_cols + visible_cursor).min(total);

    let vy = y_offset + pos / w;
    let vx = pos % w;

    if vy >= content_area.height as usize {
        return None;
    }

    Some((content_area.x + vx as u16, content_area.y + vy as u16))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_guard, mut terminal) = TuiGuard::new();
    let size = terminal.size()?;
    let mut viewport_width: u16 = size.width;
    let mut viewport_height: u16 = size.height;

    let (event_tx, mut event_rx) = mpsc::channel::<Event>(64);
    let (action_tx, mut action_rx) = mpsc::channel::<Action>(64);

    std::thread::spawn(move || loop {
        match event::read() {
            Ok(ev) => {
                if event_tx.blocking_send(ev).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    });

    let main_layout = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]);

    let mut menu = Menu::new(vec![
        make_menu_button(
            "Info",
            action_tx.clone(),
            Action::ToggleSidebar(SidebarMode::Info),
        ),
        make_menu_button(
            "Tabs",
            action_tx.clone(),
            Action::ToggleSidebar(SidebarMode::Tabs),
        ),
        make_menu_button(
            "Bookmarks",
            action_tx.clone(),
            Action::ToggleSidebar(SidebarMode::Bookmarks),
        ),
        make_menu_button("Settings", action_tx.clone(), Action::OpenSettings),
        make_menu_button("Quit", action_tx.clone(), Action::Quit),
    ]);

    let mut settings_menu = Menu::new(vec![make_menu_button(
        "Back",
        action_tx.clone(),
        Action::CloseSettings,
    )]);

    let main_space = MainSpace::new();
    let mut dialog = InputDialog::new();
    let mut textarea_dialog = TextareaDialog::new();
    let mut keybind_dialog = KeybindDialog::new();
    let mut confirm_dialog = ConfirmDialog::new();
    let mut settings_view = SettingsView::new();
    let mut config = Config::from_settings(&settings_view.settings);
    let mut tabs = TabManager::new();
    let mut bookmarks = Bookmarks::load();
    let mut url_bar = UrlBar::new();
    let mut keybind_hint = KeybindHint::new();
    let mut sidebar = Sidebar::new();
    let mut sidebar_open = false;
    let mut pending_delete_tab: Option<usize> = None;
    let mut pending_delete_bookmark: Option<usize> = None;
    let mut pending_form_reset: Option<(usize, NodeId)> = None;
    let mut pending_field: Option<(NodeId, usize)> = None;
    let mut inline_edit: Option<InlineEdit> = None;

    if !config.homepage.is_empty() {
        if let Some(tab) = tabs.active_mut() {
            tab.url = config.homepage.clone();
        }
    }

    let tab_index = tabs.active;
    let initial_url = tabs.active().map(|t| t.url.clone()).unwrap_or_default();
    if !initial_url.is_empty() {
        let _ = action_tx
            .send(Action::LoadUrl {
                tab_index,
                url: initial_url,
                mode: NavigateMode::Push,
            })
            .await;
    }

    let mut screen = Screen::Main;
    let mut main_focus = MainFocus::MainSpace;
    let mut settings_focus = SettingsFocus::Menu;
    let mut pending_edit: Option<(String, EditKind)> = None;
    let mut running = true;

    while running {
        let settings_snapshot = settings_view.settings.clone();

        if let Some(tab) = tabs.active_mut() {
            if tab.document.is_some() && tab.rendered_width != viewport_width {
                rerender_tab(tab, &settings_snapshot, viewport_width).await;
            }
        }

        if main_focus == MainFocus::UrlBar {
            if !url_bar.is_editing() {
                let current = tabs
                    .active()
                    .map(|t| t.url.clone())
                    .unwrap_or_default();
                url_bar.start_edit(&current);
            }
        } else if url_bar.is_editing() {
            url_bar.stop_edit();
        }

        let global_theme = GlobalTheme::from_settings(&settings_view.settings);

        menu.focus = screen == Screen::Main && matches!(main_focus, MainFocus::Menu);
        settings_menu.focus =
            screen == Screen::Settings && settings_focus == SettingsFocus::Menu;

        let info_text: ratatui::text::Text<'static> =
            if sidebar_open && sidebar.mode == SidebarMode::Info {
                if let Some(tab) = tabs.active() {
                    if let Some(arc) = tab.document.clone() {
                        let doc = arc.read().await;
                        build_info_text(&tab.url, Some(&doc))
                    } else {
                        build_info_text(&tab.url, None)
                    }
                } else {
                    ratatui::text::Text::default()
                }
            } else {
                ratatui::text::Text::default()
            };

        let active_tab = tabs.active;
        let active_url = tabs.active().map(|t| t.url.clone()).unwrap_or_default();
        let sidebar_is_open = sidebar_open;
        let main_focus_snapshot = main_focus;
        let inline_active = inline_edit.is_some();

        let cursor_calc: Option<(usize, String, usize, bool, String)> = inline_edit.as_ref().map(|e| {
            let cursor_chars = e.value[..e.cursor.min(e.value.len())].chars().count();
            (
                e.field_index,
                e.value.clone(),
                cursor_chars,
                e.is_password,
                e.label.clone(),
            )
        });

        terminal.draw(|frame| {
            let total = frame.area();

            let (main_area, gap_l, border_area, gap_r, side_area) = if sidebar_is_open {
                let [m, gl, b, gr, s] = Layout::horizontal([
                    Constraint::Fill(2),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Fill(1),
                ])
                .areas(total);
                (m, Some(gl), Some(b), Some(gr), Some(s))
            } else {
                (total, None, None, None, None)
            };

            if sidebar_is_open {
                let buf = frame.buffer_mut();
                let top_y = total.y;
                let header_bg_style = Style::default().bg(global_theme.header.background);
                let gl = gap_l.unwrap();
                let gr = gap_r.unwrap();
                for x in gl.x..(gr.x + gr.width) {
                    buf[(x, top_y)].set_char(' ').set_style(header_bg_style);
                }
            }

            if let Some(border_area) = border_area {
                let buf = frame.buffer_mut();
                let vch = global_theme.border.set.vertical;
                let vstyle = Style::default()
                    .fg(global_theme.border.fg)
                    .bg(global_theme.border.bg);
                for y in (total.y + 1)..(border_area.y + border_area.height) {
                    buf[(border_area.x, y)].set_char(vch).set_style(vstyle);
                }
            }

            let [header_area, url_bar_area, content_area, _, menu_area, _] =
                main_layout.areas(main_area);

            match screen {
                Screen::Main => {
                    let header = Header::new("TBrowser v1.0", tabs.active_name());
                    header.render(header_area, frame.buffer_mut(), &global_theme);

                    url_bar.render(
                        url_bar_area,
                        frame.buffer_mut(),
                        active_tab,
                        &active_url,
                        &global_theme,
                    );
                    if let Some((x, y)) = url_bar.cursor_position() {
                        frame.set_cursor_position((x, y));
                    }

                    main_space.render(content_area, frame.buffer_mut(), &global_theme);

                    if let Some(tab) = tabs.active() {
                        let inner = main_space.inner(content_area);
                        if inner.width > 0 && inner.height > 0 {
                            let total_full = tab.content.total_visual_lines(inner.width);
                            let needs_sb =
                                Scrollbar::needed(total_full, inner.height) && inner.width > 1;

                            let (content_inner, sb_area) = if needs_sb {
                                (
                                    ratatui::layout::Rect {
                                        x: inner.x,
                                        y: inner.y,
                                        width: inner.width - 1,
                                        height: inner.height,
                                    },
                                    Some(ratatui::layout::Rect {
                                        x: inner.x + inner.width - 1,
                                        y: inner.y,
                                        width: 1,
                                        height: inner.height,
                                    }),
                                )
                            } else {
                                (inner, None)
                            };

                            tab.content.selected_style(
                                ratatui::style::Style::default()
                                    .fg(global_theme.selection.fg)
                                    .bg(global_theme.selection.bg),
                            );
                            frame.render_widget(&tab.content, content_inner);

                            if let Some(sb_area) = sb_area {
                                let total = tab.content.total_visual_lines(content_inner.width);
                                let sb = Scrollbar::new(
                                    total,
                                    content_inner.height,
                                    tab.content.scroll,
                                );
                                sb.render(sb_area, frame.buffer_mut(), &global_theme);
                            }

                            if inline_active {
                                if let Some((_, ref value, cursor_chars, _is_password, ref label)) =
                                    cursor_calc
                                {
                                    let visible_cursor = cursor_chars;
                                    let prefix_cols = 6 + label.chars().count();
                                    let text_area = Rect {
                                        x: content_inner.x + 1,
                                        y: content_inner.y,
                                        width: content_inner.width.saturating_sub(2),
                                        height: content_inner.height,
                                    };
                                    if let Some((cx, cy)) = inline_cursor_xy(
                                        &tab.content.text,
                                        text_area,
                                        tab.content.scroll,
                                        tab.content.selected,
                                        prefix_cols,
                                        visible_cursor,
                                    ) {
                                        frame.set_cursor_position((cx, cy));
                                    }
                                    let _ = value;
                                }
                            }
                        }
                    }

                    menu.render(menu_area, frame.buffer_mut(), &global_theme);

                    if keybind_hint.is_visible() {
                        keybind_hint.render(menu_area, frame.buffer_mut(), &global_theme);
                    }
                }
                Screen::Settings => {
                    let header = Header::new("TBrowser v1.0", "Settings");
                    header.render(header_area, frame.buffer_mut(), &global_theme);

                    let inner = {
                        if content_area.width > 2 && content_area.height > 2 {
                            ratatui::layout::Rect {
                                x: content_area.x + 1,
                                y: content_area.y + 1,
                                width: content_area.width.saturating_sub(2),
                                height: content_area.height.saturating_sub(2),
                            }
                        } else {
                            content_area
                        }
                    };
                    settings_view.render(
                        inner,
                        frame.buffer_mut(),
                        settings_focus == SettingsFocus::Menu,
                        &global_theme,
                    );
                    settings_menu.render(menu_area, frame.buffer_mut(), &global_theme);
                }
            }

            if let Some(side_area) = side_area {
                let body_focused = main_focus_snapshot == MainFocus::Sidebar;
                let menu_focused = main_focus_snapshot == MainFocus::SidebarMenu;
                sidebar.render(
                    side_area,
                    frame.buffer_mut(),
                    &tabs,
                    &bookmarks,
                    &info_text,
                    body_focused,
                    menu_focused,
                    &global_theme,
                );
            }

            if dialog.is_open {
                let area = dialog.compute_area(total);
                dialog.render(area, frame.buffer_mut(), &global_theme);
                if let Some((x, y)) = dialog.cursor_position() {
                    frame.set_cursor_position((x, y));
                }
            }

            if textarea_dialog.is_open {
                let area = textarea_dialog.compute_area(total);
                textarea_dialog.render(area, frame.buffer_mut(), &global_theme);
                if let Some((x, y)) = textarea_dialog.cursor_position() {
                    frame.set_cursor_position((x, y));
                }
            }

            if keybind_dialog.is_open {
                let area = keybind_dialog.compute_area(total);
                keybind_dialog.render(area, frame.buffer_mut(), &global_theme);
            }

            if confirm_dialog.is_open {
                let area = confirm_dialog.compute_area(total);
                confirm_dialog.render(area, frame.buffer_mut(), &global_theme);
            }
        })?;

        tokio::select! {
            Some(ev) = event_rx.recv() => {
                if let Event::Resize(cols, rows) = ev {
                    viewport_width = cols;
                    viewport_height = rows;
                } else if let Event::Key(key) = ev {
                    if keybind_dialog.is_open {
                        keybind_dialog.handle_key(key);
                        if let Some(bind) = keybind_dialog.take_submission() {
                            if let Some((k, kind)) = pending_edit.take() {
                                if matches!(kind, EditKind::Keybind) {
                                    settings_view.apply_edit(&k, &bind, kind);
                                }
                            }
                        }
                        keybind_dialog.take_cancelled();
                        continue;
                    }
                    if confirm_dialog.is_open {
                        confirm_dialog.handle_key(key);
                        if let Some(answer) = confirm_dialog.take_submission() {
                            if answer {
                                if let Some(idx) = pending_delete_tab.take() {
                                    if tabs.len() > 1 {
                                        tabs.remove(idx);
                                    }
                                    sidebar.clamp_cursor(tabs.len());
                                }
                                if let Some(idx) = pending_delete_bookmark.take() {
                                    bookmarks.remove(idx);
                                    sidebar.clamp_cursor(bookmarks.len());
                                }
                                if let Some((tab_idx, form_id)) = pending_form_reset.take() {
                                    do_form_reset(
                                        &mut tabs,
                                        tab_idx,
                                        form_id,
                                        &settings_snapshot,
                                        viewport_width,
                                    )
                                    .await;
                                }
                            } else {
                                pending_delete_tab = None;
                                pending_delete_bookmark = None;
                                pending_form_reset = None;
                            }
                        }
                        continue;
                    }
                    if dialog.is_open {
                        dialog.handle_key(key);
                        let submission = dialog.take_submission();
                        let still_open = dialog.is_open;

                        if let Some(value) = submission {
                            if let Some((k, kind)) = pending_edit.take() {
                                settings_view.apply_edit(&k, &value, kind);
                            }
                            continue;
                        }

                        if !still_open {
                            pending_edit = None;
                        }
                        continue;
                    }
                    if textarea_dialog.is_open {
                        textarea_dialog.handle_key(key);
                        let submission = textarea_dialog.take_submission();
                        let still_open = textarea_dialog.is_open;

                        if let Some(value) = submission {
                            if let Some((form_id, field_index)) = pending_field.take() {
                                let tab_index = tabs.active;
                                if let Some(tab) = tabs.get_mut(tab_index) {
                                    tab.interactive_state
                                        .fields
                                        .insert((form_id, field_index), FieldValue::Text(value));
                                    tab.interactive_state
                                        .field_errors
                                        .remove(&(form_id, field_index));
                                }
                                if let Some(tab) = tabs.get_mut(tab_index) {
                                    rerender_tab(tab, &settings_snapshot, viewport_width).await;
                                }
                            }
                            continue;
                        }

                        if !still_open {
                            pending_field = None;
                        }
                        continue;
                    }

                    if let Some(mut edit) = inline_edit.take() {
                        let mut exit_apply = false;
                        let mut exit_cancel = false;
                        let mut exit_after_apply_with_move: Option<i32> = None;

                        match (key.code, key.modifiers) {
                            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                                running = false;
                                continue;
                            }
                            (KeyCode::Esc, _) => {
                                exit_cancel = true;
                            }
                            (KeyCode::Enter, _) | (KeyCode::Tab, _) => {
                                exit_apply = true;
                            }
                            (KeyCode::Up, _) => {
                                exit_apply = true;
                                exit_after_apply_with_move = Some(-1);
                            }
                            (KeyCode::Down, _) => {
                                exit_apply = true;
                                exit_after_apply_with_move = Some(1);
                            }
                            (KeyCode::Left, _) => {
                                if edit.cursor > 0 {
                                    let prev = edit.value[..edit.cursor].chars().last().unwrap();
                                    edit.cursor -= prev.len_utf8();
                                }
                            }
                            (KeyCode::Right, _) => {
                                if edit.cursor < edit.value.len() {
                                    let next = edit.value[edit.cursor..].chars().next().unwrap();
                                    edit.cursor += next.len_utf8();
                                }
                            }
                            (KeyCode::Home, _) => {
                                edit.cursor = 0;
                            }
                            (KeyCode::End, _) => {
                                edit.cursor = edit.value.len();
                            }
                            (KeyCode::Backspace, _) => {
                                if edit.cursor > 0 {
                                    let prev = edit.value[..edit.cursor].chars().last().unwrap();
                                    let new_pos = edit.cursor - prev.len_utf8();
                                    edit.value.replace_range(new_pos..edit.cursor, "");
                                    edit.cursor = new_pos;
                                }
                            }
                            (KeyCode::Delete, _) => {
                                if edit.cursor < edit.value.len() {
                                    let next = edit.value[edit.cursor..].chars().next().unwrap();
                                    edit.value.replace_range(
                                        edit.cursor..edit.cursor + next.len_utf8(),
                                        "",
                                    );
                                }
                            }
                            (KeyCode::Char(c), KeyModifiers::NONE)
                            | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                                if !c.is_control() {
                                    edit.value.insert(edit.cursor, c);
                                    edit.cursor += c.len_utf8();
                                }
                            }
                            _ => {}
                        }

                        let tab_index = tabs.active;
                        let key = (edit.form_id, edit.field_index);

                        if exit_cancel {
                            if let Some(tab) = tabs.get_mut(tab_index) {
                                tab.interactive_state
                                    .fields
                                    .insert(key, FieldValue::Text(edit.original.clone()));
                            }
                            if let Some(tab) = tabs.get_mut(tab_index) {
                                rerender_tab(tab, &settings_snapshot, viewport_width).await;
                            }
                            inline_edit = None;
                            continue;
                        }

                        if let Some(tab) = tabs.get_mut(tab_index) {
                            tab.interactive_state
                                .fields
                                .insert(key, FieldValue::Text(edit.value.clone()));
                            tab.interactive_state.field_errors.remove(&key);
                        }

                        if exit_apply {
                            if let Some(tab) = tabs.get_mut(tab_index) {
                                rerender_tab(tab, &settings_snapshot, viewport_width).await;
                            }
                            inline_edit = None;

                            if let Some(delta) = exit_after_apply_with_move {
                                if let Some(tab) = tabs.active_mut() {
                                    if delta < 0 {
                                        tab.content.up();
                                    } else {
                                        if !tab.content.down() {
                                            main_focus = MainFocus::Menu;
                                        }
                                    }
                                }
                            }
                            continue;
                        }

                        if let Some(tab) = tabs.get_mut(tab_index) {
                            rerender_tab(tab, &settings_snapshot, viewport_width).await;
                        }
                        inline_edit = Some(edit);
                        continue;
                    }

                    if url_bar.is_editing() {
                        let handled = url_bar.handle_key(key);
                        if handled {
                            if url_bar.take_cancelled() {
                                main_focus = MainFocus::MainSpace;
                            }
                            if let Some(url) = url_bar.take_submission() {
                                let tab_index = tabs.active;
                                let _ = action_tx
                                    .send(Action::LoadUrl {
                                        tab_index,
                                        url,
                                        mode: NavigateMode::NewUrl,
                                    })
                                    .await;
                                main_focus = MainFocus::MainSpace;
                            }
                            continue;
                        }
                    }

                    if screen == Screen::Settings {
                        handle_settings_key(
                            key,
                            &mut settings_focus,
                            &mut settings_menu,
                            &mut settings_view,
                            &mut dialog,
                            &mut keybind_dialog,
                            &mut pending_edit,
                            &action_tx,
                        )
                        .await;
                        continue;
                    }

                    if key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                    {
                        running = false;
                        continue;
                    }
                    if matches_keybind(&key, &config.kb_quit) {
                        spawn_hint(&mut keybind_hint, "Quit", global_theme.keybind_hint_duration_ms, action_tx.clone());
                        running = false;
                        continue;
                    }

                    let sidebar_target: Option<SidebarMode> =
                        if matches_keybind(&key, &config.kb_tabs) {
                            Some(SidebarMode::Tabs)
                        } else if matches_keybind(&key, &config.kb_bookmarks) {
                            Some(SidebarMode::Bookmarks)
                        } else if matches_keybind(&key, &config.kb_info) {
                            Some(SidebarMode::Info)
                        } else {
                            None
                        };
                    if let Some(target) = sidebar_target {
                        let dur = global_theme.keybind_hint_duration_ms;
                        let label = match target {
                            SidebarMode::Tabs => "Tabs",
                            SidebarMode::Bookmarks => "Bookmarks",
                            SidebarMode::Info => "Info",
                        };
                        spawn_hint(&mut keybind_hint, label, dur, action_tx.clone());
                        if sidebar_open && sidebar.mode == target {
                            sidebar_open = false;
                            if main_focus == MainFocus::Sidebar
                                || main_focus == MainFocus::SidebarMenu
                            {
                                main_focus = MainFocus::MainSpace;
                            }
                        } else {
                            sidebar.reset_for(target, &tabs);
                            sidebar_open = true;
                            main_focus = MainFocus::Sidebar;
                        }
                        continue;
                    }

                    if matches_keybind(&key, &config.kb_settings) {
                        spawn_hint(&mut keybind_hint, "Settings", global_theme.keybind_hint_duration_ms, action_tx.clone());
                        let _ = action_tx.send(Action::OpenSettings).await;
                        continue;
                    }
                    if matches_keybind(&key, &config.kb_back) {
                        spawn_hint(&mut keybind_hint, "Back", global_theme.keybind_hint_duration_ms, action_tx.clone());
                        let tab_index = tabs.active;
                        if let Some(tab) = tabs.active_mut() {
                            if let Some(url) = tab.history.back().map(|s| s.to_string()) {
                                let _ = action_tx
                                    .send(Action::LoadUrl { tab_index, url, mode: NavigateMode::History })
                                    .await;
                            }
                        }
                        continue;
                    }
                    if matches_keybind(&key, &config.kb_forward) {
                        spawn_hint(&mut keybind_hint, "Forward", global_theme.keybind_hint_duration_ms, action_tx.clone());
                        let tab_index = tabs.active;
                        if let Some(tab) = tabs.active_mut() {
                            if let Some(url) = tab.history.forward().map(|s| s.to_string()) {
                                let _ = action_tx
                                    .send(Action::LoadUrl { tab_index, url, mode: NavigateMode::History })
                                    .await;
                            }
                        }
                        continue;
                    }
                    if matches_keybind(&key, &config.kb_reload) {
                        spawn_hint(&mut keybind_hint, "Reload", global_theme.keybind_hint_duration_ms, action_tx.clone());
                        let tab_index = tabs.active;
                        if let Some(tab) = tabs.active() {
                            let url = tab.url.clone();
                            let _ = action_tx
                                .send(Action::LoadUrl { tab_index, url, mode: NavigateMode::Reload })
                                .await;
                        }
                        continue;
                    }

                    if matches_keybind(&key, &config.kb_next_tab) {
                        let dur = global_theme.keybind_hint_duration_ms;
                        if sidebar_open
                            && sidebar.mode == SidebarMode::Bookmarks
                            && main_focus == MainFocus::Sidebar
                        {
                            spawn_hint(&mut keybind_hint, "Shift favorite +1", dur, action_tx.clone());
                            bookmarks.shift_favorite(sidebar.cursor, 1);
                        } else {
                            spawn_hint(&mut keybind_hint, "Next tab", dur, action_tx.clone());
                            if tabs.len() > 1 {
                                let next = (tabs.active + 1) % tabs.len();
                                tabs.switch_to(next);
                            }
                        }
                        continue;
                    }
                    if matches_keybind(&key, &config.kb_prev_tab) {
                        let dur = global_theme.keybind_hint_duration_ms;
                        if sidebar_open
                            && sidebar.mode == SidebarMode::Bookmarks
                            && main_focus == MainFocus::Sidebar
                        {
                            spawn_hint(&mut keybind_hint, "Shift favorite -1", dur, action_tx.clone());
                            bookmarks.shift_favorite(sidebar.cursor, -1);
                        } else {
                            spawn_hint(&mut keybind_hint, "Previous tab", dur, action_tx.clone());
                            if tabs.len() > 1 {
                                let prev = (tabs.active + tabs.len() - 1) % tabs.len();
                                tabs.switch_to(prev);
                            }
                        }
                        continue;
                    }

                    if matches_keybind(&key, &config.kb_favorite) {
                        let dur = global_theme.keybind_hint_duration_ms;
                        spawn_hint(&mut keybind_hint, "Favorite", dur, action_tx.clone());
                        if let Some(tab) = tabs.active() {
                            let url = tab.url.clone();
                            let title = tab.name.clone();
                            if let Some(idx) = bookmarks.items.iter().position(|b| b.url == url) {
                                if bookmarks.items[idx].favorite.is_some() {
                                    bookmarks.unfavorite(idx);
                                } else {
                                    bookmarks.make_favorite(idx);
                                }
                            } else {
                                let before = bookmarks.len();
                                bookmarks.add(url, title);
                                if bookmarks.len() > before {
                                    let idx = bookmarks.len() - 1;
                                    bookmarks.make_favorite(idx);
                                }
                            }
                        }
                        continue;
                    }

                    {
                        let mut matched_slot: Option<u8> = None;
                        for (i, kb) in config.kb_favorites.iter().enumerate() {
                            if matches_keybind(&key, kb) {
                                matched_slot = Some((i + 1) as u8);
                                break;
                            }
                        }
                        if let Some(slot) = matched_slot {
                            if let Some(bm) = bookmarks.favorite_by_slot(slot) {
                                let url = bm.url.clone();
                                let title = bm.title.clone();
                                spawn_hint(
                                    &mut keybind_hint,
                                    &format!("Favorite {}: {}", slot, title),
                                    global_theme.keybind_hint_duration_ms,
                                    action_tx.clone(),
                                );
                                let tab_index = tabs.active;
                                let _ = action_tx
                                    .send(Action::LoadUrl {
                                        tab_index,
                                        url,
                                        mode: NavigateMode::Push,
                                    })
                                    .await;
                                continue;
                            }
                        }
                    }

                    if key.code == KeyCode::Esc && sidebar_open {
                        sidebar_open = false;
                        if main_focus == MainFocus::Sidebar || main_focus == MainFocus::SidebarMenu {
                            main_focus = MainFocus::MainSpace;
                        }
                        continue;
                    }

                    if key.code == KeyCode::Tab {
                        main_focus = if sidebar_open {
                            match main_focus {
                                MainFocus::Sidebar => MainFocus::SidebarMenu,
                                MainFocus::SidebarMenu => MainFocus::Sidebar,
                                _ => MainFocus::Sidebar,
                            }
                        } else {
                            match main_focus {
                                MainFocus::Menu => MainFocus::UrlBar,
                                MainFocus::UrlBar => MainFocus::MainSpace,
                                MainFocus::MainSpace => MainFocus::Menu,
                                _ => MainFocus::MainSpace,
                            }
                        };
                        continue;
                    }

                    if matches_keybind(&key, &config.kb_create_tab) {
                        let dur = global_theme.keybind_hint_duration_ms;
                        if sidebar_open
                            && sidebar.mode == SidebarMode::Bookmarks
                            && main_focus == MainFocus::Sidebar
                        {
                            spawn_hint(&mut keybind_hint, "Add bookmark", dur, action_tx.clone());
                            if let Some(tab) = tabs.active() {
                                let url = tab.url.clone();
                                let title = tab.name.clone();
                                let before = bookmarks.len();
                                bookmarks.add(url, title);
                                if bookmarks.len() > before {
                                    sidebar.cursor = bookmarks.len() - 1;
                                }
                            }
                        } else {
                            spawn_hint(&mut keybind_hint, "Create tab", dur, action_tx.clone());
                            if (tabs.len() as u32) < config.max_tabs {
                                let url = "about:new".to_string();
                                let idx = tabs.add("New Tab", url.clone());
                                if sidebar_open && sidebar.mode == SidebarMode::Tabs {
                                    sidebar.cursor = idx;
                                }
                                let _ = action_tx
                                    .send(Action::LoadUrl {
                                        tab_index: idx,
                                        url,
                                        mode: NavigateMode::Push,
                                    })
                                    .await;
                            }
                        }
                        continue;
                    }

                    if matches_keybind(&key, &config.kb_delete_tab) {
                        let dur = global_theme.keybind_hint_duration_ms;
                        if sidebar_open
                            && sidebar.mode == SidebarMode::Bookmarks
                            && main_focus == MainFocus::Sidebar
                        {
                            spawn_hint(&mut keybind_hint, "Delete bookmark", dur, action_tx.clone());
                            if bookmarks.len() > 0 {
                                bookmarks.remove(sidebar.cursor);
                                sidebar.clamp_cursor(bookmarks.len());
                            }
                        } else if sidebar_open
                            && sidebar.mode == SidebarMode::Tabs
                            && main_focus == MainFocus::Sidebar
                        {
                            spawn_hint(&mut keybind_hint, "Delete tab", dur, action_tx.clone());
                            if tabs.len() > 1 {
                                if config.delete_confirmation {
                                    pending_delete_tab = Some(sidebar.cursor);
                                    confirm_dialog.open("Delete Tab", "Delete this tab?");
                                } else {
                                    tabs.remove(sidebar.cursor);
                                    sidebar.clamp_cursor(tabs.len());
                                }
                            }
                        } else {
                            spawn_hint(&mut keybind_hint, "Delete tab", dur, action_tx.clone());
                            if tabs.len() > 1 {
                                if config.delete_confirmation {
                                    pending_delete_tab = Some(tabs.active);
                                    confirm_dialog.open("Delete Tab", "Delete this tab?");
                                } else {
                                    tabs.remove(tabs.active);
                                }
                            }
                        }
                        continue;
                    }

                    match main_focus {
                        MainFocus::UrlBar => {}
                        MainFocus::SidebarMenu => {
                            let btn_total = sidebar.button_count();
                            match key.code {
                                KeyCode::Up => {
                                    if sidebar.button_index > 0 {
                                        sidebar.button_index -= 1;
                                    } else {
                                        main_focus = MainFocus::Sidebar;
                                        let total = match sidebar.mode {
                                            SidebarMode::Tabs => tabs.len(),
                                            SidebarMode::Bookmarks => bookmarks.len(),
                                            SidebarMode::Info => info_text.lines.len(),
                                        };
                                        sidebar.cursor = total.saturating_sub(1);
                                    }
                                }
                                KeyCode::Down => {
                                    if sidebar.button_index + 1 < btn_total {
                                        sidebar.button_index += 1;
                                    } else {
                                        main_focus = MainFocus::Sidebar;
                                        sidebar.cursor = 0;
                                    }
                                }
                                KeyCode::Left => {
                                    if sidebar.button_index > 0 {
                                        sidebar.button_index -= 1;
                                    }
                                }
                                KeyCode::Right => {
                                    if sidebar.button_index + 1 < btn_total {
                                        sidebar.button_index += 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    let dur = global_theme.keybind_hint_duration_ms;
                                    match sidebar.button_action() {
                                        SidebarMenuButton::Close => {
                                            sidebar_open = false;
                                            main_focus = MainFocus::MainSpace;
                                        }
                                        SidebarMenuButton::Create => match sidebar.mode {
                                            SidebarMode::Tabs => {
                                                spawn_hint(&mut keybind_hint, "Create tab", dur, action_tx.clone());
                                                if (tabs.len() as u32) < config.max_tabs {
                                                    let url = "about:new".to_string();
                                                    let idx = tabs.add("New Tab", url.clone());
                                                    sidebar.cursor = idx;
                                                    let _ = action_tx
                                                        .send(Action::LoadUrl {
                                                            tab_index: idx,
                                                            url,
                                                            mode: NavigateMode::Push,
                                                        })
                                                        .await;
                                                }
                                            }
                                            SidebarMode::Bookmarks => {
                                                spawn_hint(&mut keybind_hint, "Add bookmark", dur, action_tx.clone());
                                                if let Some(tab) = tabs.active() {
                                                    let url = tab.url.clone();
                                                    let title = tab.name.clone();
                                                    let before = bookmarks.len();
                                                    bookmarks.add(url, title);
                                                    if bookmarks.len() > before {
                                                        sidebar.cursor = bookmarks.len() - 1;
                                                    }
                                                }
                                            }
                                            SidebarMode::Info => {}
                                        },
                                        SidebarMenuButton::Favorite => match sidebar.mode {
                                            SidebarMode::Bookmarks => {
                                                spawn_hint(&mut keybind_hint, "Favorite", dur, action_tx.clone());
                                                if sidebar.cursor < bookmarks.len() {
                                                    if bookmarks.items[sidebar.cursor].favorite.is_some() {
                                                        bookmarks.unfavorite(sidebar.cursor);
                                                    } else {
                                                        bookmarks.make_favorite(sidebar.cursor);
                                                    }
                                                }
                                            }
                                            _ => {}
                                        },
                                        SidebarMenuButton::Delete => match sidebar.mode {
                                            SidebarMode::Tabs => {
                                                spawn_hint(&mut keybind_hint, "Delete tab", dur, action_tx.clone());
                                                if tabs.len() > 1 {
                                                    if config.delete_confirmation {
                                                        pending_delete_tab = Some(sidebar.cursor);
                                                        confirm_dialog.open("Delete Tab", "Delete this tab?");
                                                    } else {
                                                        tabs.remove(sidebar.cursor);
                                                        sidebar.clamp_cursor(tabs.len());
                                                    }
                                                }
                                            }
                                            SidebarMode::Bookmarks => {
                                                spawn_hint(&mut keybind_hint, "Delete bookmark", dur, action_tx.clone());
                                                if bookmarks.len() > 0 {
                                                    bookmarks.remove(sidebar.cursor);
                                                    sidebar.clamp_cursor(bookmarks.len());
                                                }
                                            }
                                            SidebarMode::Info => {}
                                        },
                                    }
                                }
                                _ => {}
                            }
                        }
                        MainFocus::Sidebar => {
                            let total = match sidebar.mode {
                                SidebarMode::Tabs => tabs.len(),
                                SidebarMode::Bookmarks => bookmarks.len(),
                                SidebarMode::Info => info_text.lines.len(),
                            };
                            match key.code {
                                KeyCode::Up => {
                                    if sidebar.cursor > 0 {
                                        sidebar.up(total);
                                    } else {
                                        main_focus = MainFocus::SidebarMenu;
                                        sidebar.clamp_button();
                                    }
                                }
                                KeyCode::Down => {
                                    if total > 0 && sidebar.cursor + 1 < total {
                                        sidebar.down(total);
                                    } else {
                                        main_focus = MainFocus::SidebarMenu;
                                        sidebar.button_index = 0;
                                    }
                                }
                                KeyCode::PageUp => {
                                    let step = 10usize.min(sidebar.cursor);
                                    sidebar.cursor = sidebar.cursor.saturating_sub(step);
                                }
                                KeyCode::PageDown => {
                                    if total > 0 {
                                        let new = (sidebar.cursor + 10).min(total - 1);
                                        sidebar.cursor = new;
                                    }
                                }
                                KeyCode::Enter => {
                                    match sidebar.mode {
                                        SidebarMode::Tabs => {
                                            tabs.switch_to(sidebar.cursor);
                                            sidebar_open = false;
                                            main_focus = MainFocus::MainSpace;
                                        }
                                        SidebarMode::Bookmarks => {
                                            if let Some(b) = bookmarks.items.get(sidebar.cursor) {
                                                let url = b.url.clone();
                                                let tab_index = tabs.active;
                                                let _ = action_tx
                                                    .send(Action::LoadUrl {
                                                        tab_index,
                                                        url,
                                                        mode: NavigateMode::Push,
                                                    })
                                                    .await;
                                                sidebar_open = false;
                                                main_focus = MainFocus::MainSpace;
                                            }
                                        }
                                        SidebarMode::Info => {}
                                    }
                                }
                                _ => {}
                            }
                        }
                        MainFocus::Menu => match key.code {
                            KeyCode::Enter => {
                                menu.activate();
                            }
                            KeyCode::Right => menu.next(),
                            KeyCode::Left => menu.prev(),
                            KeyCode::Up => {
                                main_focus = MainFocus::MainSpace;
                            }
                            _ => {}
                        },
                        MainFocus::MainSpace => {
                            if matches_keybind(&key, &config.kb_open_new_tab) {
                                let tab_index = tabs.active;
                                let url_opt = tabs.get(tab_index).and_then(|tab| {
                                    let sel = tab.content.selected as usize;
                                    tab.content
                                        .meta
                                        .get(sel)
                                        .and_then(|m| m.links.first().map(|l| l.url.clone()))
                                });
                                if let Some(url) = url_opt {
                                    if (tabs.len() as u32) < config.max_tabs {
                                        let idx = tabs.add("New Tab", url.clone());
                                        if sidebar_open && sidebar.mode == SidebarMode::Tabs {
                                            sidebar.cursor = idx;
                                        }
                                        let _ = action_tx
                                            .send(Action::LoadUrl {
                                                tab_index: idx,
                                                url,
                                                mode: NavigateMode::Push,
                                            })
                                            .await;
                                    }
                                }
                                continue;
                            }

                            if key.code == KeyCode::Enter {
                                let tab_index = tabs.active;

                                enum Decision {
                                    Nothing,
                                    ToggleSpoiler(NodeId, bool),
                                    ToggleCheckbox(NodeId, usize),
                                    SetIndex(NodeId, usize, usize),
                                    OpenSelect(NodeId, usize),
                                    CloseSelect(NodeId, usize),
                                    OpenText(NodeId, usize, InteractiveFieldKind),
                                    OpenTextarea(NodeId, usize, InteractiveFieldKind),
                                    LoadUrl(String),
                                    SubmitForm(NodeId),
                                    ResetForm(NodeId),
                                }

                                let decision = match tabs.get(tab_index) {
                                    Some(tab) => {
                                        let sel = tab.content.selected as usize;
                                        match tab.content.meta.get(sel) {
                                            Some(meta) => match meta.kind {
                                                LineKind::SpoilerHeader {
                                                    node_id,
                                                    expanded,
                                                } => Decision::ToggleSpoiler(node_id, !expanded),

                                                LineKind::FormText {
                                                    form_id,
                                                    field_index,
                                                    kind,
                                                } => {
                                                    if matches!(kind, InteractiveFieldKind::Textarea) {
                                                        Decision::OpenTextarea(form_id, field_index, kind)
                                                    } else {
                                                        Decision::OpenText(form_id, field_index, kind)
                                                    }
                                                }

                                                LineKind::FormCheckbox {
                                                    form_id,
                                                    field_index,
                                                } => Decision::ToggleCheckbox(form_id, field_index),

                                                LineKind::FormRadioOption {
                                                    form_id,
                                                    field_index,
                                                    option_index,
                                                } => Decision::SetIndex(
                                                    form_id,
                                                    field_index,
                                                    option_index,
                                                ),

                                                LineKind::FormSelectCollapsed {
                                                    form_id,
                                                    field_index,
                                                } => Decision::OpenSelect(form_id, field_index),

                                                LineKind::FormSelectHeader {
                                                    form_id,
                                                    field_index,
                                                } => Decision::CloseSelect(form_id, field_index),

                                                LineKind::FormSelectOption {
                                                    form_id,
                                                    field_index,
                                                    option_index,
                                                } => Decision::SetIndex(
                                                    form_id,
                                                    field_index,
                                                    option_index,
                                                ),

                                                LineKind::FormSubmit { form_id } => {
                                                    Decision::SubmitForm(form_id)
                                                }

                                                LineKind::FormReset { form_id } => {
                                                    Decision::ResetForm(form_id)
                                                }

                                                LineKind::Normal => {
                                                    if let Some(link) = meta.links.first() {
                                                        Decision::LoadUrl(link.url.clone())
                                                    } else {
                                                        Decision::Nothing
                                                    }
                                                }
                                            },
                                            None => Decision::Nothing,
                                        }
                                    }
                                    None => Decision::Nothing,
                                };

                                match decision {
                                    Decision::Nothing => {}

                                    Decision::ToggleSpoiler(id, new_val) => {
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            tab.interactive_state
                                                .spoilers
                                                .insert(id, new_val);
                                        }
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }
                                        continue;
                                    }

                                    Decision::ToggleCheckbox(fid, fi) => {
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            let prev = matches!(
                                                tab.interactive_state.fields.get(&(fid, fi)),
                                                Some(FieldValue::Checked(true))
                                            );
                                            tab.interactive_state.fields.insert(
                                                (fid, fi),
                                                FieldValue::Checked(!prev),
                                            );
                                            tab.interactive_state
                                                .field_errors
                                                .remove(&(fid, fi));
                                        }
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }
                                        continue;
                                    }

                                    Decision::SetIndex(fid, fi, idx) => {
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            tab.interactive_state
                                                .fields
                                                .insert((fid, fi), FieldValue::Index(idx));
                                            tab.interactive_state.select_open.remove(&(fid, fi));
                                            tab.interactive_state
                                                .field_errors
                                                .remove(&(fid, fi));
                                        }
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }
                                        continue;
                                    }

                                    Decision::OpenSelect(fid, fi) => {
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            tab.interactive_state.select_open.insert((fid, fi));
                                        }
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }
                                        continue;
                                    }

                                    Decision::CloseSelect(fid, fi) => {
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            tab.interactive_state.select_open.remove(&(fid, fi));
                                        }
                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }
                                        continue;
                                    }

                                    Decision::OpenText(fid, fi, kind) => {
                                        let info: Option<(String, String, bool)> = {
                                            match tabs.get(tab_index) {
                                                Some(tab) if tab.document.is_some() => {
                                                    let arc = tab.document.clone().unwrap();
                                                    let guard = arc.read().await;
                                                    match guard.find_form_field(fid, fi) {
                                                        Some(FormField::Text {
                                                            label,
                                                            value,
                                                            kind,
                                                            ..
                                                        }) => {
                                                            let is_pw = matches!(
                                                                kind,
                                                                core::document::FieldKind::Password
                                                            );
                                                            Some((
                                                                label.clone(),
                                                                value.clone(),
                                                                is_pw,
                                                            ))
                                                        }
                                                        _ => None,
                                                    }
                                                }
                                                _ => None,
                                            }
                                        };

                                        let Some((label, default_value, is_password)) = info else {
                                            continue;
                                        };

                                        let current = tabs
                                            .active()
                                            .and_then(|tab| {
                                                match tab
                                                    .interactive_state
                                                    .fields
                                                    .get(&(fid, fi))
                                                {
                                                    Some(FieldValue::Text(t)) => {
                                                        Some(t.clone())
                                                    }
                                                    _ => None,
                                                }
                                            })
                                            .unwrap_or_else(|| default_value.clone());

                                        inline_edit = Some(InlineEdit {
                                            form_id: fid,
                                            field_index: fi,
                                            value: current.clone(),
                                            cursor: current.len(),
                                            original: current.clone(),
                                            label,
                                            is_password,
                                        });
                                        let _ = kind;
                                        continue;
                                    }

                                    Decision::OpenTextarea(fid, fi, _kind) => {
                                        let current = tabs
                                            .active()
                                            .and_then(|tab| {
                                                match tab
                                                    .interactive_state
                                                    .fields
                                                    .get(&(fid, fi))
                                                {
                                                    Some(FieldValue::Text(t)) => {
                                                        Some(t.clone())
                                                    }
                                                    _ => None,
                                                }
                                            })
                                            .unwrap_or_default();
                                        textarea_dialog.open(current, "Edit Textarea");
                                        pending_field = Some((fid, fi));
                                        continue;
                                    }

                                    Decision::LoadUrl(url) => {
                                        let _ = action_tx
                                            .send(Action::LoadUrl {
                                                tab_index,
                                                url,
                                                mode: NavigateMode::Push,
                                            })
                                            .await;
                                        continue;
                                    }

                                    Decision::SubmitForm(form_id) => {
                                        let form_data: Option<(String, Vec<FormItem>)> =
                                            match tabs.get(tab_index) {
                                                Some(tab) if tab.document.is_some() => {
                                                    let arc = tab.document.clone().unwrap();
                                                    let guard = arc.read().await;
                                                    guard.find_form(form_id).map(|(a, _m, items)| {
                                                        (a.to_string(), items.to_vec())
                                                    })
                                                }
                                                _ => None,
                                            };

                                        let Some((action_str, items)) = form_data else {
                                            continue;
                                        };

                                        let state = tabs
                                            .get(tab_index)
                                            .map(|t| t.interactive_state.clone())
                                            .unwrap_or_default();

                                        let mut data: Vec<(String, String)> = Vec::new();
                                        let mut errors: Vec<(NodeId, usize)> = Vec::new();
                                        let mut field_index = 0usize;

                                        for item in &items {
                                            if let FormItem::Field(field) = item {
                                                let key = (form_id, field_index);
                                                let val = state.fields.get(&key);

                                                match field {
                                                    FormField::Text {
                                                        name,
                                                        required,
                                                        value,
                                                        kind,
                                                        ..
                                                    } => {
                                                        let text = match val {
                                                            Some(FieldValue::Text(t)) => t.clone(),
                                                            _ => value.clone(),
                                                        };
                                                        if *required && text.trim().is_empty() {
                                                            errors.push(key);
                                                        } else if matches!(
                                                            kind,
                                                            FieldKind::Email
                                                        ) && !text.trim().is_empty()
                                                            && !validate_email(text.trim())
                                                        {
                                                            errors.push(key);
                                                        }
                                                        if !name.is_empty() {
                                                            data.push((name.clone(), text));
                                                        }
                                                    }
                                                    FormField::Textarea {
                                                        name,
                                                        required,
                                                        value,
                                                        ..
                                                    } => {
                                                        let text = match val {
                                                            Some(FieldValue::Text(t)) => t.clone(),
                                                            _ => value.clone(),
                                                        };
                                                        if *required && text.trim().is_empty() {
                                                            errors.push(key);
                                                        }
                                                        if !name.is_empty() {
                                                            data.push((name.clone(), text));
                                                        }
                                                    }
                                                    FormField::Checkbox {
                                                        name,
                                                        checked,
                                                        required,
                                                        ..
                                                    } => {
                                                        let is_checked = match val {
                                                            Some(FieldValue::Checked(b)) => *b,
                                                            _ => *checked,
                                                        };
                                                        if *required && !is_checked {
                                                            errors.push(key);
                                                        }
                                                        if is_checked && !name.is_empty() {
                                                            data.push((name.clone(), "on".to_string()));
                                                        }
                                                    }
                                                    FormField::Radio { name, options, .. } => {
                                                        let idx = match val {
                                                            Some(FieldValue::Index(i)) => *i,
                                                            _ => options
                                                                .iter()
                                                                .position(|o| o.checked)
                                                                .unwrap_or(0),
                                                        };
                                                        if let Some(opt) = options.get(idx) {
                                                            if !name.is_empty() {
                                                                data.push((name.clone(), opt.value.clone()));
                                                            }
                                                        }
                                                    }
                                                    FormField::Select { name, options, .. } => {
                                                        let idx = match val {
                                                            Some(FieldValue::Index(i)) => *i,
                                                            _ => options
                                                                .iter()
                                                                .position(|o| o.selected)
                                                                .unwrap_or(0),
                                                        };
                                                        if let Some(opt) = options.get(idx) {
                                                            if !name.is_empty() {
                                                                data.push((name.clone(), opt.value.clone()));
                                                            }
                                                        }
                                                    }
                                                    FormField::Hidden { name, value } => {
                                                        if !name.is_empty() {
                                                            data.push((name.clone(), value.clone()));
                                                        }
                                                    }
                                                    FormField::Submit { .. }
                                                    | FormField::Reset { .. } => {}
                                                }
                                                field_index += 1;
                                            }
                                        }

                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            tab.interactive_state
                                                .field_errors
                                                .retain(|(fid, _)| *fid != form_id);
                                        }

                                        if !errors.is_empty() {
                                            if let Some(tab) = tabs.get_mut(tab_index) {
                                                tab.interactive_state
                                                    .form_errors
                                                    .remove(&form_id);
                                                for key in errors {
                                                    tab.interactive_state
                                                        .field_errors
                                                        .insert(key);
                                                }
                                            }
                                            if let Some(tab) = tabs.get_mut(tab_index) {
                                                rerender_tab(tab, &settings_snapshot, viewport_width)
                                                    .await;
                                            }
                                            continue;
                                        }

                                        let tab_url = tabs
                                            .get(tab_index)
                                            .map(|t| t.url.clone())
                                            .unwrap_or_default();
                                        let full_url = if action_str.is_empty() {
                                            tab_url.clone()
                                        } else {
                                            Url::parse(&tab_url)
                                                .and_then(|b| b.join(&action_str))
                                                .map(|u| u.to_string())
                                                .unwrap_or(action_str.clone())
                                        };

                                        let scheme = Url::parse(&full_url)
                                            .map(|u| u.scheme().to_string())
                                            .unwrap_or_default();

                                        if scheme != "tmutp" {
                                            if let Some(tab) = tabs.get_mut(tab_index) {
                                                tab.interactive_state.form_errors.insert(
                                                    form_id,
                                                    format!("POST not supported for scheme: {}", scheme),
                                                );
                                            }
                                            if let Some(tab) = tabs.get_mut(tab_index) {
                                                rerender_tab(tab, &settings_snapshot, viewport_width)
                                                    .await;
                                            }
                                            continue;
                                        }

                                        let generation = match tabs.get_mut(tab_index) {
                                            Some(tab) => {
                                                tab.interactive_state
                                                    .form_errors
                                                    .remove(&form_id);
                                                tab.interactive_state.field_errors.clear();
                                                tab.load_generation =
                                                    tab.load_generation.wrapping_add(1);
                                                tab.history.push(full_url.clone());
                                                tab.load_generation
                                            }
                                            None => continue,
                                        };

                                        if let Some(tab) = tabs.get_mut(tab_index) {
                                            rerender_tab(tab, &settings_snapshot, viewport_width)
                                                .await;
                                        }

                                        url_bar.begin(full_url.clone(), tab_index);

                                        let tx = action_tx.clone();
                                        let tab_index_c = tab_index;
                                        let full_url_c = full_url.clone();
                                        let settings_c = settings_snapshot.clone();
                                        let width = viewport_width;
                                        let timeout = config.timeout;

                                        tokio::spawn(async move {
                                            let client = match get_post_protocol("tmutp") {
                                                Some(c) => c,
                                                None => {
                                                    let _ = tx
                                                        .send(Action::FormError {
                                                            tab_index: tab_index_c,
                                                            form_id,
                                                            message: "POST not supported".to_string(),
                                                            generation,
                                                        })
                                                        .await;
                                                    return;
                                                }
                                            };

                                            let data_refs: Vec<(&str, &str)> = data
                                                .iter()
                                                .map(|(k, v)| (k.as_str(), v.as_str()))
                                                .collect();

                                            let result = tokio::time::timeout(
                                                Duration::from_secs(timeout.max(1)),
                                                client.post(&full_url_c, &data_refs, None),
                                            )
                                            .await;

                                            match result {
                                                Ok(Ok(response)) => {
                                                    let content_type = response
                                                        .headers
                                                        .get("content-type")
                                                        .map(String::as_str)
                                                        .unwrap_or("text/plain")
                                                        .to_string();
                                                    let parser = get_parser(&content_type);
                                                    let doc = parser
                                                        .parse(&response.text, &content_type)
                                                        .await;
                                                    let renderer = get_renderer(&content_type, &settings_c);
                                                    let empty = RenderState::default();
                                                    match renderer
                                                        .render(
                                                            &doc,
                                                            &full_url_c,
                                                            width,
                                                            &empty,
                                                        )
                                                        .await
                                                    {
                                                        Ok(rendered) => {
                                                            let _ = tx
                                                                .send(Action::Loaded {
                                                                    tab_index: tab_index_c,
                                                                    url: full_url_c,
                                                                    document: Arc::new(
                                                                        RwLock::new(doc),
                                                                    ),
                                                                    rendered,
                                                                    generation,
                                                                    viewport_width: width,
                                                                })
                                                                .await;
                                                        }
                                                        Err(e) => {
                                                            let _ = tx
                                                                .send(Action::FormError {
                                                                    tab_index: tab_index_c,
                                                                    form_id,
                                                                    message: format!(
                                                                        "Render error: {}",
                                                                        e
                                                                    ),
                                                                    generation,
                                                                })
                                                                .await;
                                                        }
                                                    }
                                                }
                                                Ok(Err(e)) => {
                                                    let _ = tx
                                                        .send(Action::FormError {
                                                            tab_index: tab_index_c,
                                                            form_id,
                                                            message: format!(
                                                                "POST failed: {}",
                                                                e
                                                            ),
                                                            generation,
                                                        })
                                                        .await;
                                                }
                                                Err(_) => {
                                                    let _ = tx
                                                        .send(Action::FormError {
                                                            tab_index: tab_index_c,
                                                            form_id,
                                                            message: format!(
                                                                "POST timed out after {}s",
                                                                timeout
                                                            ),
                                                            generation,
                                                        })
                                                        .await;
                                                }
                                            }
                                        });

                                        continue;
                                    }

                                    Decision::ResetForm(form_id) => {
                                        if config.form_reset {
                                            pending_form_reset = Some((tab_index, form_id));
                                            confirm_dialog
                                                .open("Reset Form", "Reset all fields in this form?");
                                        } else {
                                            do_form_reset(
                                                &mut tabs,
                                                tab_index,
                                                form_id,
                                                &settings_snapshot,
                                                viewport_width,
                                            )
                                            .await;
                                        }
                                        continue;
                                    }
                                }
                            }

                            if let Some(tab) = tabs.active_mut() {
                                match key.code {
                                    KeyCode::Up => {
                                        tab.content.up();
                                    }
                                    KeyCode::Down => {
                                        if !tab.content.down() {
                                            main_focus = MainFocus::Menu;
                                        }
                                    }
                                    KeyCode::PageUp => tab.content.page_up(),
                                    KeyCode::PageDown => tab.content.page_down(),
                                    KeyCode::Home => tab.content.scroll_to(0),
                                    KeyCode::End => {
                                        let last = tab.content.text.lines.len().saturating_sub(1) as u16;
                                        tab.content.scroll_to(last);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }

            Some(action) = action_rx.recv() => {
                match action {
                    Action::Quit => running = false,
                    Action::ToggleSidebar(mode) => {
                        if sidebar_open && sidebar.mode == mode {
                            sidebar_open = false;
                            if main_focus == MainFocus::Sidebar
                                || main_focus == MainFocus::SidebarMenu
                            {
                                main_focus = MainFocus::MainSpace;
                            }
                        } else {
                            sidebar.reset_for(mode, &tabs);
                            sidebar_open = true;
                            main_focus = MainFocus::Sidebar;
                        }
                    }
                    Action::OpenSettings => {
                        screen = Screen::Settings;
                        settings_focus = SettingsFocus::Column;
                        settings_view.column = SettingsColumn::Categories;
                        settings_menu.selected = 0;
                    }
                    Action::CloseSettings => {
                        config = Config::from_settings(&settings_view.settings);
                        for i in 0..tabs.len() {
                            if let Some(tab) = tabs.get_mut(i) {
                                rerender_tab(tab, &settings_view.settings, viewport_width).await;
                            }
                        }
                        screen = Screen::Main;
                        main_focus = MainFocus::MainSpace;
                    }
                    Action::HideHint { token } => {
                        keybind_hint.hide(token);
                    }
                    Action::LoadStage { url, stage, .. } => {
                        if url == url_bar.loading_url {
                            url_bar.set_stage(stage);
                            if stage == LoadStage::Done {
                                let token = url_bar.generation;
                                let tx = action_tx.clone();
                                let hold = config.url_bar_hold_ms;
                                tokio::spawn(async move {
                                    tokio::time::sleep(Duration::from_millis(hold)).await;
                                    let _ = tx.send(Action::LoadHide { generation: token }).await;
                                });
                            }
                        }
                    }
                    Action::LoadHide { generation } => {
                        url_bar.hide(generation);
                    }
                    Action::LoadUrl { tab_index, url, mode } => {
                        inline_edit = None;
                        let generation = match tabs.get_mut(tab_index) {
                            Some(tab) => {
                                tab.load_generation = tab.load_generation.wrapping_add(1);
                                tab.url = url.clone();
                                tab.pending_images = 0;
                                match mode {
                                    NavigateMode::Push => {
                                        tab.history.push(url.clone());
                                    }
                                    NavigateMode::NewUrl => {
                                        tab.history.clear();
                                        tab.history.push(url.clone());
                                    }
                                    _ => {}
                                }
                                tab.load_generation
                            }
                            None => continue,
                        };

                        let is_special = url.starts_with("about:");
                        if !is_special {
                            url_bar.begin(url.clone(), tab_index);
                        }

                        let tx = action_tx.clone();
                        let settings_c = settings_view.settings.clone();
                        let width = viewport_width;
                        let timeout = config.timeout;
                        let follow = config.follow_redirects;
                        tokio::spawn(async move {
                            match load_url(
                                &url,
                                tab_index,
                                settings_c,
                                width,
                                timeout,
                                follow,
                                tx.clone(),
                                generation,
                            )
                            .await
                            {
                                Ok((doc, rendered)) => {
                                    let _ = tx
                                        .send(Action::Loaded {
                                            tab_index,
                                            url,
                                            document: Arc::new(RwLock::new(doc)),
                                            rendered,
                                            generation,
                                            viewport_width: width,
                                        })
                                        .await;
                                }
                                Err(e) => {
                                    let _ = tx
                                        .send(Action::LoadFailed {
                                            tab_index,
                                            url,
                                            error: e.to_string(),
                                            generation,
                                        })
                                        .await;
                                }
                            }
                        });
                    }
                    Action::Loaded {
                        tab_index,
                        url,
                        document,
                        rendered,
                        generation,
                        viewport_width: loaded_width,
                    } => {
                        let title = {
                            let doc = document.read().await;
                            doc.title.clone()
                        };

                        let mut images: Vec<(NodeId, String, Option<u16>)> = Vec::new();
                        if config.autoload_images {
                            let doc = document.read().await;
                            for (id, node) in doc.iter() {
                                if let NodeData::Image { src, height, .. } = node {
                                    images.push((id, src.clone(), *height));
                                }
                            }
                        }

                        if let Some(tab) = tabs.get_mut(tab_index) {
                            if tab.load_generation != generation {
                                continue;
                            }
                            tab.url = url.clone();
                            if !title.is_empty() {
                                tab.name = title;
                            }
                            tab.document = Some(document);
                            tab.interactive_state = RenderState::default();
                            tab.content.set_rendered(rendered);
                            tab.rendered_width = loaded_width;
                            tab.pending_images = images.len();
                        }

                        if images.is_empty() {
                            let _ = action_tx
                                .send(Action::LoadStage {
                                    tab_index,
                                    url: url.clone(),
                                    stage: LoadStage::Done,
                                })
                                .await;
                        } else {
                            let _ = action_tx
                                .send(Action::LoadStage {
                                    tab_index,
                                    url: url.clone(),
                                    stage: LoadStage::Images,
                                })
                                .await;

                            for (node_id, src, height_attr) in images {
                                let tx = action_tx.clone();
                                let base_url = url.clone();
                                let vh = viewport_height;
                                let timeout = config.timeout;

                                tokio::spawn(async move {
                                    let result = load_image_ascii(
                                        &base_url,
                                        &src,
                                        height_attr,
                                        vh,
                                        timeout,
                                    )
                                    .await;
                                    let _ = tx
                                        .send(Action::ImageLoaded {
                                            tab_index,
                                            node_id,
                                            result,
                                            generation,
                                        })
                                        .await;
                                });
                            }
                        }
                    }
                    Action::LoadFailed { tab_index, url, error, generation } => {
                        if let Some(tab) = tabs.get_mut(tab_index) {
                            if tab.load_generation != generation {
                                continue;
                            }
                            if url == url_bar.loading_url {
                                url_bar.force_idle();
                            }
                            tab.url = url.clone();
                            let doc = build_error_page(&url, &error);
                            let renderer = get_renderer(&doc.content_type, &settings_view.settings);
                            let empty = RenderState::default();
                            let rendered = renderer
                                .render(&doc, &url, viewport_width, &empty)
                                .await
                                .unwrap_or_else(|_| RenderedContent::empty());
                            tab.document = Some(Arc::new(RwLock::new(doc)));
                            tab.interactive_state = RenderState::default();
                            tab.content.set_rendered(rendered);
                            tab.rendered_width = viewport_width;
                            tab.pending_images = 0;
                        }
                    }
                    Action::FormError { tab_index, form_id, message, generation } => {
                        if let Some(tab) = tabs.get_mut(tab_index) {
                            if tab.load_generation != generation {
                                continue;
                            }
                            tab.interactive_state.form_errors.insert(form_id, message);
                        }
                        url_bar.force_idle();
                        if let Some(tab) = tabs.get_mut(tab_index) {
                            rerender_tab(tab, &settings_view.settings, viewport_width).await;
                        }
                    }
                    Action::ImageLoaded { tab_index, node_id, result, generation } => {
                        let mut all_done = false;
                        let mut url_for_stage = String::new();

                        if let Some(tab) = tabs.get_mut(tab_index) {
                            if tab.load_generation != generation {
                                continue;
                            }
                            let state = match result {
                                Ok(lines) => ImageState::Ready(lines),
                                Err(e) => ImageState::Failed(e),
                            };
                            tab.interactive_state.images.insert(node_id, state);
                            if tab.pending_images > 0 {
                                tab.pending_images -= 1;
                            }
                            if tab.pending_images == 0 {
                                all_done = true;
                                url_for_stage = tab.url.clone();
                            }
                        }

                        if let Some(tab) = tabs.get_mut(tab_index) {
                            rerender_tab(tab, &settings_view.settings, viewport_width).await;
                        }

                        if all_done {
                            let _ = action_tx
                                .send(Action::LoadStage {
                                    tab_index,
                                    url: url_for_stage,
                                    stage: LoadStage::Done,
                                })
                                .await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn handle_settings_key(
    key: KeyEvent,
    settings_focus: &mut SettingsFocus,
    settings_menu: &mut Menu,
    settings_view: &mut SettingsView,
    dialog: &mut InputDialog,
    keybind_dialog: &mut KeybindDialog,
    pending_edit: &mut Option<(String, EditKind)>,
    action_tx: &mpsc::Sender<Action>,
) {
    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            let _ = action_tx.send(Action::Quit).await;
            return;
        }
        (KeyCode::Esc, _) => {
            let _ = action_tx.send(Action::CloseSettings).await;
            return;
        }
        _ => {}
    }

    match settings_focus {
        SettingsFocus::Menu => match (key.code, key.modifiers) {
            (KeyCode::Tab, _) => {
                *settings_focus = SettingsFocus::Column;
            }
            (KeyCode::Enter, _) => settings_menu.activate(),
            (KeyCode::Right, _) => settings_menu.next(),
            (KeyCode::Left, _) => settings_menu.prev(),
            _ => {}
        },
        SettingsFocus::Column => match key.code {
            KeyCode::Tab => {
                *settings_focus = SettingsFocus::Menu;
            }
            KeyCode::Left => settings_view.left(),
            KeyCode::Right => settings_view.right(),
            KeyCode::Up => settings_view.up(),
            KeyCode::Down => settings_view.down(),
            KeyCode::Enter | KeyCode::Char(' ') => {
                if let Some(req) = settings_view.enter() {
                    if matches!(req.kind, EditKind::Keybind) {
                        keybind_dialog.open(req.current, req.title);
                    } else {
                        dialog.open(req.current, req.title, false);
                    }
                    *pending_edit = Some((req.key, req.kind));
                }
            }
            _ => {}
        },
    }
}