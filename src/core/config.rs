use crate::core::settings::{Settings, Value};

#[derive(Debug, Clone)]
pub struct Config {
    pub homepage: String,
    pub max_tabs: u32,
    pub autoload_images: bool,
    pub timeout: u64,
    pub follow_redirects: bool,
    pub url_bar_hold_ms: u64,
    pub delete_confirmation: bool,

    pub kb_info: String,
    pub kb_tabs: String,
    pub kb_settings: String,
    pub kb_quit: String,
    pub kb_create_tab: String,
    pub kb_delete_tab: String,
    pub kb_next_tab: String,
    pub kb_prev_tab: String,
    pub kb_back: String,
    pub kb_forward: String,
    pub kb_reload: String,
    pub kb_bookmarks: String,
    pub kb_favorite: String,
    pub kb_favorites: [String; 10],
}

impl Config {
    pub fn from_settings(settings: &Settings) -> Self {
        let mut cfg = Self::default();

        if let Some(Value::Str(s)) = settings.get("general.homepage") {
            cfg.homepage = s.clone();
        }
        if let Some(Value::Int(i)) = settings.get("general.max_tabs") {
            cfg.max_tabs = (*i).max(1) as u32;
        }
        if let Some(Value::Bool(b)) = settings.get("general.autoload_images") {
            cfg.autoload_images = *b;
        }
        if let Some(Value::Int(i)) = settings.get("network.timeout") {
            cfg.timeout = (*i).max(1) as u64;
        }
        if let Some(Value::Bool(b)) = settings.get("network.follow_redirects") {
            cfg.follow_redirects = *b;
        }
        if let Some(Value::Float(f)) = settings.get("theme.url_bar_hold") {
            cfg.url_bar_hold_ms = (f.max(0.0) * 1000.0) as u64;
        }
        if let Some(Value::Bool(b)) = settings.get("global.keybind.delete_hint") {
            cfg.delete_confirmation = *b;
        }

        macro_rules! get_kb {
            ($key:literal, $field:ident) => {
                if let Some(Value::Str(s)) = settings.get($key) {
                    cfg.$field = s.clone();
                }
            };
        }

        get_kb!("global.keybind.info", kb_info);
        get_kb!("global.keybind.tabs", kb_tabs);
        get_kb!("global.keybind.settings", kb_settings);
        get_kb!("global.keybind.quit", kb_quit);
        get_kb!("global.keybind.create_tab", kb_create_tab);
        get_kb!("global.keybind.delete_tab", kb_delete_tab);
        get_kb!("global.keybind.next_tab", kb_next_tab);
        get_kb!("global.keybind.prev_tab", kb_prev_tab);
        get_kb!("global.keybind.back", kb_back);
        get_kb!("global.keybind.forward", kb_forward);
        get_kb!("global.keybind.reload", kb_reload);
        get_kb!("global.keybind.bookmarks", kb_bookmarks);
        get_kb!("global.keybind.favorite", kb_favorite);

        for (i, key) in [
            "global.keybind.favorite_1",
            "global.keybind.favorite_2",
            "global.keybind.favorite_3",
            "global.keybind.favorite_4",
            "global.keybind.favorite_5",
            "global.keybind.favorite_6",
            "global.keybind.favorite_7",
            "global.keybind.favorite_8",
            "global.keybind.favorite_9",
            "global.keybind.favorite_10",
        ]
        .iter()
        .enumerate()
        {
            if let Some(Value::Str(s)) = settings.get(key) {
                cfg.kb_favorites[i] = s.clone();
            }
        }

        cfg
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            homepage: "tmutp://localhost/".to_string(),
            max_tabs: 20,
            autoload_images: true,
            timeout: 30,
            follow_redirects: true,
            url_bar_hold_ms: 300,
            delete_confirmation: true,

            kb_info: "i".to_string(),
            kb_tabs: "t".to_string(),
            kb_settings: "s".to_string(),
            kb_quit: "q".to_string(),
            kb_create_tab: "c".to_string(),
            kb_delete_tab: "d".to_string(),
            kb_next_tab: "n".to_string(),
            kb_prev_tab: "p".to_string(),
            kb_back: "b".to_string(),
            kb_forward: "f".to_string(),
            kb_reload: "r".to_string(),
            kb_bookmarks: "m".to_string(),
            kb_favorite: "v".to_string(),
            kb_favorites: [
                "1".to_string(),
                "2".to_string(),
                "3".to_string(),
                "4".to_string(),
                "5".to_string(),
                "6".to_string(),
                "7".to_string(),
                "8".to_string(),
                "9".to_string(),
                "0".to_string(),
            ],
        }
    }
}