use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

const FAVORITE_SLOTS: u8 = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub url: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub favorite: Option<u8>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Bookmarks {
    #[serde(rename = "bookmark", default)]
    pub items: Vec<Bookmark>,
}

#[allow(unused)]
impl Bookmarks {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/tbrowser/bookmarks.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            return Self::new();
        };
        match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::path() else {
            anyhow::bail!("no bookmarks path");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(&path, s)?;
        Ok(())
    }

    pub fn add(&mut self, url: String, title: String) {
        if url.is_empty() || url.starts_with("about:") {
            return;
        }
        if self.items.iter().any(|b| b.url == url) {
            return;
        }
        self.items.push(Bookmark {
            url,
            title,
            favorite: None,
        });
        let _ = self.save();
    }

    pub fn remove(&mut self, index: usize) {
        if index < self.items.len() {
            self.items.remove(index);
            let _ = self.save();
        }
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    // ===== Favorites =====

    pub fn free_slots(&self) -> Vec<u8> {
        let used: HashSet<u8> = self.items.iter().filter_map(|b| b.favorite).collect();
        (1..=FAVORITE_SLOTS).filter(|s| !used.contains(s)).collect()
    }

    pub fn has_free_slot(&self) -> bool {
        self.items.iter().filter(|b| b.favorite.is_some()).count() < FAVORITE_SLOTS as usize
    }

    /// Делает закладку фаворитом в первый свободный слот. Возвращает true, если удалось.
    pub fn make_favorite(&mut self, index: usize) -> bool {
        if index >= self.items.len() {
            return false;
        }
        if self.items[index].favorite.is_some() {
            return false;
        }
        let used: HashSet<u8> = self.items.iter().filter_map(|b| b.favorite).collect();
        let Some(slot) = (1..=FAVORITE_SLOTS).find(|s| !used.contains(s)) else {
            return false;
        };
        self.items[index].favorite = Some(slot);
        let _ = self.save();
        true
    }

    pub fn unfavorite(&mut self, index: usize) {
        if index < self.items.len() {
            self.items[index].favorite = None;
            let _ = self.save();
        }
    }

    /// Сдвигает favorite на +delta/-delta в диапазоне 1..=10. Меняет местами,
    /// если целевой слот занят. Никакой ротации.
    pub fn shift_favorite(&mut self, index: usize, delta: i8) {
        if index >= self.items.len() {
            return;
        }
        let Some(slot) = self.items[index].favorite else {
            return;
        };
        let target = slot as i16 + delta as i16;
        if target < 1 || target > FAVORITE_SLOTS as i16 {
            return;
        }
        let target = target as u8;
        // Если слот занят — меняемся местами
        if let Some(other) = self.items.iter().position(|b| b.favorite == Some(target)) {
            self.items[other].favorite = Some(slot);
        }
        self.items[index].favorite = Some(target);
        let _ = self.save();
    }

    pub fn favorite_by_slot(&self, slot: u8) -> Option<&Bookmark> {
        self.items.iter().find(|b| b.favorite == Some(slot))
    }
}