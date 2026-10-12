use std::sync::Arc;

use ratatui::text::Text;
use tokio::sync::RwLock;

use crate::core::content::Content;
use crate::core::document::Document;
use crate::core::history::LocalHistory;

use crate::core::renderer::RenderState;

#[allow(unused)]
pub struct Tab {
    pub id: u64,
    pub name: String,
    pub url: String,
    pub document: Option<Arc<RwLock<Document>>>,
    pub content: Content,
    pub load_generation: u64,
    pub rendered_width: u16,
    pub history: LocalHistory,
    pub interactive_state: RenderState,
}

impl Tab {
    pub fn new(id: u64, name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            url: url.into(),
            document: None,
            content: Content::new(Text::default()),
            load_generation: 0,
            rendered_width: 0,
            history: LocalHistory::new(),
            interactive_state: RenderState::default(),
        }
    }
}

pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active: usize,
    next_id: u64,
}

impl TabManager {
    pub fn new() -> Self {
        let mut m = Self {
            tabs: Vec::new(),
            active: 0,
            next_id: 1,
        };
        m.add("Main", "tmutp://localhost/");
        m
    }

    pub fn active(&self) -> Option<&Tab> {
        self.tabs.get(self.active)
    }

    pub fn active_mut(&mut self) -> Option<&mut Tab> {
        self.tabs.get_mut(self.active)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Tab> {
        self.tabs.get_mut(index)
    }

    pub fn add(&mut self, name: impl Into<String>, url: impl Into<String>) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        let tab = Tab::new(id, name, url);
        let pos = if self.tabs.is_empty() {
            0
        } else {
            self.active + 1
        };
        self.tabs.insert(pos, tab);
        self.active = pos;
        pos
    }

    pub fn remove(&mut self, index: usize) -> bool {
        if index >= self.tabs.len() || self.tabs.len() <= 1 {
            return false;
        }
        self.tabs.remove(index);
        if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
        true
    }

    pub fn switch_to(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    #[allow(unused)]
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn active_name(&self) -> &str {
        self.active().map(|t| t.name.as_str()).unwrap_or("")
    }
}

impl Default for TabManager {
    fn default() -> Self {
        Self::new()
    }
}