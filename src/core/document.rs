use std::collections::HashMap;

use indextree::{Arena, NodeId};
use ratatui::style::Color;

#[derive(Debug, Clone)]
pub enum MetaValue {
    Str(String),
    Map(HashMap<String, String>),
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SpanStyle {
    pub bold: bool,
    pub italic: bool,
    pub strikethrough: bool,
    pub code: bool,
    pub superscript: bool,
    pub subscript: bool,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct LinkInfo {
    pub url: String,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub text: String,
    pub style: SpanStyle,
    pub link: Option<LinkInfo>,
}

impl Span {
    pub fn raw(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: SpanStyle::default(),
            link: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoKind {
    Info,
    Warning,
    Success,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorBorder {
    Rounded,
    Left,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub enum ListMarker {
    Bullet,
    Ordered(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    Text,
    Email,
    Password,
}

#[derive(Debug, Clone)]
pub struct RadioOption {
    pub label: String,
    pub value: String,
    pub checked: bool,
}

#[derive(Debug, Clone)]
pub struct SelectOption {
    pub label: String,
    pub value: String,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub enum FormField {
    Text {
        label: String,
        name: String,
        placeholder: String,
        required: bool,
        value: String,
        kind: FieldKind,
    },
    Textarea {
        label: String,
        name: String,
        rows: u16,
        placeholder: String,
        required: bool,
        value: String,
    },
    Checkbox {
        label: String,
        name: String,
        checked: bool,
        required: bool,
    },
    Radio {
        name: String,
        options: Vec<RadioOption>,
    },
    Select {
        name: String,
        options: Vec<SelectOption>,
        multiple: bool,
    },
    Submit {
        label: String,
    },
    Reset {
        label: String,
    },
    Hidden {
        name: String,
        value: String,
    },
}

#[derive(Debug, Clone)]
pub enum FormItem {
    Line(Vec<Span>),
    Field(FormField),
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum NodeData {
    Root,

    Paragraph(Vec<Span>),
    Heading { level: u8, spans: Vec<Span> },
    ListItem { indent: u8, marker: ListMarker, spans: Vec<Span> },
    Definition { term: Vec<Span> },
    HorizontalRule { thick: bool },
    CodeBlock { language: Option<String>, text: String },
    Image { alt: String, src: String, width: Option<u16>, height: Option<u16> },

    BlockQuote { cite: Option<String>, date: Option<String> },
    InfoBlock { kind: InfoKind },
    ColorBlock { color: Color, border: ColorBorder },
    Spoiler { title: Option<String> },

    Table {
        headers: Vec<Vec<Span>>,
        rows: Vec<Vec<Vec<Span>>>,
        alignments: Vec<Alignment>,
    },

    Form {
        action: String,
        method: String,
        items: Vec<FormItem>,
    },

    GopherBlock {
        header: String,
        lines: Vec<String>,
    },
}

#[derive(Debug)]
pub struct Document {
    pub content_type: String,
    pub meta: HashMap<String, MetaValue>,
    pub title: String,
    arena: Arena<NodeData>,
    root: NodeId,
}

impl Document {
    pub fn new(content_type: impl Into<String>) -> Self {
        let mut arena = Arena::new();
        let root = arena.new_node(NodeData::Root);
        Self {
            content_type: content_type.into(),
            meta: HashMap::new(),
            title: String::new(),
            arena,
            root,
        }
    }

    pub fn root(&self) -> NodeId {
        self.root
    }

    pub fn add_node(&mut self, data: NodeData) -> NodeId {
        let id = self.arena.new_node(data);
        self.root.append(id, &mut self.arena);
        id
    }

    pub fn add_child(&mut self, parent: NodeId, data: NodeData) -> NodeId {
        let id = self.arena.new_node(data);
        parent.append(id, &mut self.arena);
        id
    }

    pub fn add_paragraph(&mut self, text: impl Into<String>) -> NodeId {
        self.add_node(NodeData::Paragraph(vec![Span::raw(text)]))
    }

    pub fn children(&self, id: NodeId) -> Vec<(NodeId, &NodeData)> {
        id.children(&self.arena)
            .filter_map(|c| self.arena.get(c).map(|n| (c, n.get())))
            .collect()
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &NodeData)> + '_ {
        let arena = &self.arena;
        self.root
            .descendants(arena)
            .filter_map(move |id| arena.get(id).map(|n| (id, n.get())))
    }

    #[allow(unused)]
    pub fn get(&self, id: NodeId) -> Option<&NodeData> {
        self.arena.get(id).map(|n| n.get())
    }

    pub fn find_form_field(&self, form_id: NodeId, field_index: usize) -> Option<&FormField> {
        let node = self.arena.get(form_id)?;
        if let NodeData::Form { items, .. } = node.get() {
            return items
                .iter()
                .filter_map(|item| match item {
                    FormItem::Field(f) => Some(f),
                    _ => None,
                })
                .nth(field_index);
        }
        None
    }

    pub fn variables(&self) -> Option<&HashMap<String, String>> {
        match self.meta.get("variables") {
            Some(MetaValue::Map(m)) => Some(m),
            _ => None,
        }
    }

    pub fn string_meta(&self) -> impl Iterator<Item = (&String, &String)> {
        self.meta.iter().filter_map(|(k, v)| match v {
            MetaValue::Str(s) => Some((k, s)),
            _ => None,
        })
    }
}