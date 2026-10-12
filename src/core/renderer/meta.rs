use std::collections::HashMap;

use indextree::NodeId;
use ratatui::text::Text;

#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    SpoilerHeader { node_id: NodeId, expanded: bool },
}

#[derive(Debug, Clone)]
pub struct LineMeta {
    pub kind: LineKind,
    pub links: Vec<Link>,
}

impl LineMeta {
    pub fn normal() -> Self {
        Self {
            kind: LineKind::Normal,
            links: Vec::new(),
        }
    }

    pub fn with_links(links: Vec<Link>) -> Self {
        Self {
            kind: LineKind::Normal,
            links,
        }
    }

    pub fn spoiler(node_id: NodeId, expanded: bool) -> Self {
        Self {
            kind: LineKind::SpoilerHeader { node_id, expanded },
            links: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderedContent {
    pub text: Text<'static>,
    pub meta: Vec<LineMeta>,
}

impl RenderedContent {
    pub fn new(text: Text<'static>, meta: Vec<LineMeta>) -> Self {
        Self { text, meta }
    }

    pub fn empty() -> Self {
        Self {
            text: Text::default(),
            meta: Vec::new(),
        }
    }
}

pub type InteractiveState = HashMap<NodeId, bool>;