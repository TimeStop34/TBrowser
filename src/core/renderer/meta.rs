use std::collections::{HashMap, HashSet};

use indextree::NodeId;
use ratatui::text::Text;

#[derive(Debug, Clone)]
pub struct Link {
    pub url: String,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FieldValue {
    Text(String),
    Checked(bool),
    Index(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractiveFieldKind {
    Text,
    Email,
    Password,
    Textarea,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    Normal,
    SpoilerHeader {
        node_id: NodeId,
        expanded: bool,
    },
    FormText {
        form_id: NodeId,
        field_index: usize,
        kind: InteractiveFieldKind,
    },
    FormCheckbox {
        form_id: NodeId,
        field_index: usize,
    },
    FormRadioOption {
        form_id: NodeId,
        field_index: usize,
        option_index: usize,
    },
    FormSelectCollapsed {
        form_id: NodeId,
        field_index: usize,
    },
    FormSelectHeader {
        form_id: NodeId,
        field_index: usize,
    },
    FormSelectOption {
        form_id: NodeId,
        field_index: usize,
        option_index: usize,
    },
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

    pub fn form_text(form_id: NodeId, field_index: usize, kind: InteractiveFieldKind) -> Self {
        Self {
            kind: LineKind::FormText {
                form_id,
                field_index,
                kind,
            },
            links: Vec::new(),
        }
    }

    pub fn form_checkbox(form_id: NodeId, field_index: usize) -> Self {
        Self {
            kind: LineKind::FormCheckbox {
                form_id,
                field_index,
            },
            links: Vec::new(),
        }
    }

    pub fn form_radio_option(form_id: NodeId, field_index: usize, option_index: usize) -> Self {
        Self {
            kind: LineKind::FormRadioOption {
                form_id,
                field_index,
                option_index,
            },
            links: Vec::new(),
        }
    }

    pub fn form_select_collapsed(form_id: NodeId, field_index: usize) -> Self {
        Self {
            kind: LineKind::FormSelectCollapsed {
                form_id,
                field_index,
            },
            links: Vec::new(),
        }
    }

    pub fn form_select_header(form_id: NodeId, field_index: usize) -> Self {
        Self {
            kind: LineKind::FormSelectHeader {
                form_id,
                field_index,
            },
            links: Vec::new(),
        }
    }

    pub fn form_select_option(form_id: NodeId, field_index: usize, option_index: usize) -> Self {
        Self {
            kind: LineKind::FormSelectOption {
                form_id,
                field_index,
                option_index,
            },
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

#[derive(Debug, Clone, Default)]
pub struct RenderState {
    pub spoilers: HashMap<NodeId, bool>,
    pub fields: HashMap<(NodeId, usize), FieldValue>,
    pub select_open: HashSet<(NodeId, usize)>,
}