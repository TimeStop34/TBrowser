use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, RwLock};
use url::Url;

use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::Style,
    text::Text,
};

mod core;
mod guard;
mod network;
mod ui;

use core::bookmarks::Bookmarks;
use core::info::build_info_text;
use core::parser::get_parser;
use core::renderer::{get_renderer, NoTheme, RendererTheme};
use core::{build_error_page, Config, Document, Tab, TabManager};
use guard::TuiGuard;
use network::base::FetchOptions;
use network::factory::get_protocol;
use ui::{
    ButtonCallback, ConfirmDialog, EditKind, GlobalTheme, Header, InputDialog, KeybindDialog,
    KeybindHint, LoadStage, MainFocus, MainSpace, Menu, MenuButton, Scrollbar, SettingsColumn,
    SettingsView, Sidebar, SidebarMenuButton, SidebarMode, UrlBar,
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
        text: Text<'static>,
        generation: u64,
        viewport_width: u16,
    },
    LoadFailed {
        tab_index: usize,
        url: String,
        error: String,
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

async fn load_url(
    url: &str,
    tab_index: usize,
    theme: Arc<dyn RendererTheme>,
    viewport_width: u16,
    timeout_secs: u64,
    follow_redirects: bool,
    action_tx: mpsc::Sender<Action>,
    generation: u64,
) -> anyhow::Result<(Document, Text<'static>)> {
    if let Some(doc) = build_special_page(url) {
        let renderer = get_renderer(&doc.content_type);
        let text = renderer
            .render(&doc, url, theme.as_ref(), viewport_width)
            .await?;
        return Ok((doc, text));
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

    let renderer = get_renderer(&content_type);
    let text = renderer
        .render(&doc, url, theme.as_ref(), viewport_width)
        .await?;

    let _ = action_tx
        .send(Action::LoadStage {
            tab_index,
            url: url.to_string(),
            stage: LoadStage::Done,
        })
        .await;

    let _ = generation;

    Ok((doc, text))
}

fn spawn_hint(hint: &mut KeybindHint, text: &str, duration_ms: u64, tx: mpsc::Sender<Action>) {
    let token = hint.show(text);
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(duration_ms)).await;
        let _ = tx.send(Action::HideHint { token }).await;
    });
}

async fn rerender_tab(tab: &mut Tab, theme: &dyn RendererTheme, width: u16) {
    let Some(doc_arc) = tab.document.clone() else {
        tab.rendered_width = width;
        return;
    };
    let doc = doc_arc.read().await;
    let renderer = get_renderer(&doc.content_type);
    let url = tab.url.clone();
    let result = renderer.render(&doc, &url, theme, width).await;
    drop(doc);
    if let Ok(text) = result {
        let saved_scroll = tab.content.scroll;
        let saved_selected = tab.content.selected;
        tab.content.text = text;
        let max = tab.content.text.lines.len().saturating_sub(1) as u16;
        tab.content.scroll = saved_scroll.min(max);
        tab.content.selected = saved_selected.min(max);
    }
    tab.rendered_width = width;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (_guard, mut terminal) = TuiGuard::new();
    let mut viewport_width: u16 = terminal.size()?.width;

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

    if !config.homepage.is_empty() {
        if let Some(tab) = tabs.active_mut() {
            tab.url = config.homepage.clone();
        }
    }

    let theme: Arc<dyn RendererTheme> = Arc::new(NoTheme);

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
        if let Some(tab) = tabs.active_mut() {
            if tab.document.is_some() && tab.rendered_width != viewport_width {
                rerender_tab(tab, theme.as_ref(), viewport_width).await;
            }
        }

        // Sync url_bar editing with focus
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

        let info_text: Text<'static> = if sidebar_open && sidebar.mode == SidebarMode::Info {
            if let Some(tab) = tabs.active() {
                if let Some(arc) = tab.document.clone() {
                    let doc = arc.read().await;
                    build_info_text(&tab.url, Some(&doc))
                } else {
                    build_info_text(&tab.url, None)
                }
            } else {
                Text::default()
            }
        } else {
            Text::default()
        };

        let active_tab = tabs.active;
        let active_url = tabs.active().map(|t| t.url.clone()).unwrap_or_default();
        let sidebar_is_open = sidebar_open;
        let main_focus_snapshot = main_focus;

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
                if let Event::Resize(cols, _rows) = ev {
                    viewport_width = cols;
                } else if let Event::Key(key) = ev {
                    // ===== Модалки =====
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
                            } else {
                                pending_delete_tab = None;
                                pending_delete_bookmark = None;
                            }
                        }
                        continue;
                    }
                    if dialog.is_open {
                        dialog.handle_key(key);
                        if let Some(value) = dialog.take_submission() {
                            if let Some((k, kind)) = pending_edit.take() {
                                settings_view.apply_edit(&k, &value, kind);
                            }
                        }
                        continue;
                    }

                    // URL bar inline editing — приоритет над глобальными keybind'ами
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
                        // Tab, Ctrl+C и т.п. — пропускаем, обработаются ниже
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

                    // ===== Screen::Main =====

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

                    // t / m / i — sidebar
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

                    // n / p — сдвиг favorite или next/prev tab
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

                    // favorite (v)
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

                    // Digit shortcuts — открытие favorite по слоту
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

                    // Esc
                    if key.code == KeyCode::Esc && sidebar_open {
                        sidebar_open = false;
                        if main_focus == MainFocus::Sidebar || main_focus == MainFocus::SidebarMenu {
                            main_focus = MainFocus::MainSpace;
                        }
                        continue;
                    }

                    // Tab — циклический фокус
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

                    // c — глобально
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

                    // d — глобально
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

                    // ===== Фокус-зависимая навигация =====
                    match main_focus {
                        MainFocus::UrlBar => {
                            // Всё уже обработано выше — сюда не попадаем в editing-режиме
                        }
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
                        let generation = match tabs.get_mut(tab_index) {
                            Some(tab) => {
                                tab.load_generation = tab.load_generation.wrapping_add(1);
                                tab.url = url.clone();
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
                        let theme = Arc::clone(&theme);
                        let width = viewport_width;
                        let timeout = config.timeout;
                        let follow = config.follow_redirects;
                        tokio::spawn(async move {
                            match load_url(
                                &url,
                                tab_index,
                                theme,
                                width,
                                timeout,
                                follow,
                                tx.clone(),
                                generation,
                            )
                            .await
                            {
                                Ok((doc, text)) => {
                                    let _ = tx
                                        .send(Action::Loaded {
                                            tab_index,
                                            url,
                                            document: Arc::new(RwLock::new(doc)),
                                            text,
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
                        text,
                        generation,
                        viewport_width: loaded_width,
                    } => {
                        let title = {
                            let doc = document.read().await;
                            doc.title.clone()
                        };
                        if let Some(tab) = tabs.get_mut(tab_index) {
                            if tab.load_generation != generation {
                                continue;
                            }
                            tab.url = url;
                            if !title.is_empty() {
                                tab.name = title;
                            }
                            tab.document = Some(document);
                            tab.content.set_text(text);
                            tab.rendered_width = loaded_width;
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
                            let renderer = get_renderer(&doc.content_type);
                            let text = renderer
                                .render(&doc, &url, theme.as_ref(), viewport_width)
                                .await
                                .unwrap_or_default();
                            tab.document = Some(Arc::new(RwLock::new(doc)));
                            tab.content.set_text(text);
                            tab.rendered_width = viewport_width;
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