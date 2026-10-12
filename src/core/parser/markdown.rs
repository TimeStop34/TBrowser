use std::collections::HashMap;

use async_trait::async_trait;
use pulldown_cmark::{
    CodeBlockKind, Event, HeadingLevel, Options, Parser as CmParser, Tag, TagEnd,
};

use crate::core::document::{Document, MetaValue, NodeData, Span, SpanStyle};
use crate::core::parser::base::Parser;

pub struct MarkdownParser;

#[async_trait]
impl Parser for MarkdownParser {
    fn content_type(&self) -> &str {
        "text/markdown"
    }

    async fn parse(&self, raw: &str, content_type: &str) -> Document {
        let mut doc = Document::new(content_type);

        let (meta, body) = extract_front_matter(raw);
        for (k, v) in meta {
            doc.meta.insert(k, v);
        }

        parse_events(body, &mut doc);

        if doc.title.is_empty() {
            let mut found: Option<String> = None;
            for (_, node) in doc.iter() {
                if let NodeData::Heading { level: 1, spans } = node {
                    found = Some(spans.iter().map(|s| s.text.as_str()).collect());
                    break;
                }
            }
            if let Some(t) = found {
                doc.title = t;
            }
        }

        doc
    }
}

#[derive(Default)]
struct State {
    spans: Vec<Span>,
    style: SpanStyle,
    link: Option<String>,
    list_depth: u8,
    ordered: Vec<bool>,
    counters: Vec<u64>,
    quote_depth: u8,
    pending_heading: Option<u8>,
    in_codeblock: bool,
    code_lang: Option<String>,
    code_text: String,
}

fn parse_events(body: &str, doc: &mut Document) {
    let mut s = State::default();

    for event in CmParser::new_ext(body, Options::all()) {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                s.spans.clear();
                s.pending_heading = Some(heading_level_to_u8(level));
            }
            Event::End(TagEnd::Heading(_)) => {
                let spans = std::mem::take(&mut s.spans);
                let level = s.pending_heading.take().unwrap_or(1);
                doc.add_node(NodeData::Heading { level, spans });
            }
            Event::Start(Tag::Paragraph) => {
                s.spans.clear();
            }
            Event::End(TagEnd::Paragraph) => {
                if s.list_depth == 0 && s.quote_depth == 0 {
                    let spans = std::mem::take(&mut s.spans);
                    if !spans.is_empty() {
                        doc.add_node(NodeData::Paragraph(spans));
                    }
                }
            }
            Event::Start(Tag::Emphasis) => s.style.italic = true,
            Event::End(TagEnd::Emphasis) => s.style.italic = false,
            Event::Start(Tag::Strong) => s.style.bold = true,
            Event::End(TagEnd::Strong) => s.style.bold = false,
            Event::Start(Tag::Strikethrough) => s.style.strikethrough = true,
            Event::End(TagEnd::Strikethrough) => s.style.strikethrough = false,
            Event::Start(Tag::Link { dest_url, .. }) => {
                s.link = Some(dest_url.to_string());
            }
            Event::End(TagEnd::Link) => s.link = None,
            Event::Start(Tag::CodeBlock(kind)) => {
                s.in_codeblock = true;
                s.code_lang = match kind {
                    CodeBlockKind::Fenced(lang) => {
                        let l = lang.to_string();
                        if l.is_empty() { None } else { Some(l) }
                    }
                    CodeBlockKind::Indented => None,
                };
                s.code_text.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                s.in_codeblock = false;
                doc.add_node(NodeData::CodeBlock {
                    language: s.code_lang.take(),
                    text: std::mem::take(&mut s.code_text),
                });
            }
            Event::Start(Tag::List(start)) => {
                s.list_depth = s.list_depth.saturating_add(1);
                s.ordered.push(start.is_some());
                s.counters.push(start.unwrap_or(1));
            }
            Event::End(TagEnd::List(_)) => {
                s.list_depth = s.list_depth.saturating_sub(1);
                s.ordered.pop();
                s.counters.pop();
            }
            Event::Start(Tag::Item) => {
                s.spans.clear();
            }
            Event::End(TagEnd::Item) => {
                let spans = std::mem::take(&mut s.spans);
                let indent = s.list_depth.saturating_sub(1);
                let marker = if s.ordered.last().copied().unwrap_or(false) {
                    let counter = s.counters.last_mut().unwrap();
                    let m = format!("{}.", counter);
                    *counter += 1;
                    m
                } else {
                    "•".to_string()
                };
                doc.add_node(NodeData::ListItem {
                    indent,
                    marker,
                    spans,
                });
            }
            Event::Start(Tag::BlockQuote(_)) => {
                s.quote_depth = s.quote_depth.saturating_add(1);
                s.spans.clear();
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                s.quote_depth = s.quote_depth.saturating_sub(1);
                let spans = std::mem::take(&mut s.spans);
                if !spans.is_empty() {
                    doc.add_node(NodeData::BlockQuote(spans));
                }
            }
            Event::Rule => {
                doc.add_node(NodeData::HorizontalRule);
            }
            Event::Text(t) => {
                if s.in_codeblock {
                    s.code_text.push_str(&t);
                } else {
                    s.spans.push(Span {
                        text: t.to_string(),
                        style: s.style,
                        link: s.link.clone(),
                    });
                }
            }
            Event::Code(t) => {
                let mut style = s.style;
                style.code = true;
                s.spans.push(Span {
                    text: t.to_string(),
                    style,
                    link: s.link.clone(),
                });
            }
            Event::Html(t) | Event::InlineHtml(t) => {
                s.spans.push(Span {
                    text: t.to_string(),
                    style: s.style,
                    link: s.link.clone(),
                });
            }
            Event::SoftBreak | Event::HardBreak => {
                if s.in_codeblock {
                    s.code_text.push('\n');
                } else {
                    s.spans.push(Span {
                        text: " ".to_string(),
                        style: s.style,
                        link: s.link.clone(),
                    });
                }
            }
            _ => {}
        }
    }
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn extract_front_matter(raw: &str) -> (HashMap<String, MetaValue>, &str) {
    if !raw.starts_with("---\n") {
        return (HashMap::new(), raw);
    }
    let rest = &raw[4..];
    let Some(end) = rest.find("\n---\n") else {
        return (HashMap::new(), raw);
    };
    let meta_block = &rest[..end];
    let body = &rest[end + 5..];

    let mut meta: HashMap<String, MetaValue> = HashMap::new();
    let mut current_map: Option<(String, HashMap<String, String>, usize)> = None;

    for line in meta_block.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.len() - line.trim_start().len();

        if let Some((key, map, base_indent)) = &mut current_map {
            if indent > *base_indent {
                if let Some((k, v)) = line.trim().split_once(':') {
                    map.insert(
                        k.trim().to_string(),
                        v.trim().trim_matches('"').to_string(),
                    );
                }
                continue;
            } else {
                let (k, m, _) = current_map.take().unwrap();
                meta.insert(k, MetaValue::Map(m));
            }
        }

        if let Some((k, v)) = line.split_once(':') {
            let k = k.trim().to_string();
            let v = v.trim();
            if v.is_empty() {
                current_map = Some((k, HashMap::new(), indent));
            } else {
                meta.insert(k, MetaValue::Str(v.trim_matches('"').to_string()));
            }
        }
    }
    if let Some((k, m, _)) = current_map.take() {
        meta.insert(k, MetaValue::Map(m));
    }

    (meta, body)
}