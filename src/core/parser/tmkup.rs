use std::collections::HashMap;

use async_trait::async_trait;
use ratatui::style::Color;

use crate::core::document::{
    Alignment, ColorBorder, Document, FieldKind, FormField, FormItem, InfoKind, LinkInfo,
    ListMarker, MetaValue, NodeData, RadioOption, SelectOption, Span, SpanStyle,
};
use crate::core::parser::base::Parser;

pub struct TmkupParser;

#[async_trait]
impl Parser for TmkupParser {
    fn content_type(&self) -> &str {
        "text/tmkup"
    }

    async fn parse(&self, raw: &str, content_type: &str) -> Document {
        let mut doc = Document::new(content_type);

        let (meta, variables, body) = extract_meta(raw);
        for (k, v) in &meta {
            doc.meta.insert(k.clone(), v.clone());
        }
        doc.meta
            .insert("variables".to_string(), MetaValue::Map(variables.clone()));

        // meta.title → doc.title (если задан)
        if let Some(MetaValue::Str(t)) = doc.meta.get("title") {
            doc.title = t.clone();
        }

        let lines = strip_comments(&body);
        let lines = trim_empty(&lines);
        parse_lines(&lines, &mut doc, &variables);

        if doc.title.is_empty() {
            let mut found: Option<String> = None;
            for (_, node) in doc.iter() {
                if let NodeData::Heading { level: 1, spans } = node {
                    found = Some(spans.iter().map(|s| s.text.as_str()).collect());
                    break;
                }
            }
            doc.title = found.unwrap_or_else(|| "TMkUp Document".to_string());
        }

        doc
    }
}

// ============================================================
// Meta
// ============================================================

fn extract_meta(raw: &str) -> (HashMap<String, MetaValue>, HashMap<String, String>, String) {
    let mut meta = HashMap::new();
    let mut variables = HashMap::new();

    let mut body = raw.to_string();

    let trimmed = raw.trim_start();
    if let Some(rest) = trimmed.strip_prefix("---tmk-meta") {
        if let Some(end) = rest.find("---tmk-meta") {
            let meta_block = &rest[..end];
            let after = &rest[end + "---tmk-meta".len()..];
            body = after.trim_start_matches('\n').to_string();

            let mut in_vars = false;
            let mut var_indent = 0usize;
            for line in meta_block.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let indent = line.len() - line.trim_start().len();

                if in_vars {
                    if indent > var_indent {
                        if let Some((k, v)) = line.trim().split_once(':') {
                            variables.insert(
                                k.trim().to_string(),
                                v.trim().trim_matches('"').to_string(),
                            );
                            continue;
                        }
                    } else {
                        in_vars = false;
                    }
                }

                if let Some((k, v)) = line.split_once(':') {
                    let k = k.trim().to_string();
                    let v = v.trim();
                    if v.is_empty() && k == "variables" {
                        in_vars = true;
                        var_indent = indent;
                    } else {
                        meta.insert(k, MetaValue::Str(v.trim_matches('"').to_string()));
                    }
                }
            }
        }
    }

    (meta, variables, body)
}

// ============================================================
// Комментарии и пустые строки
// ============================================================

fn strip_comments(raw: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut in_multiline = false;

    for line in raw.lines() {
        if in_multiline {
            if let Some(end) = line.find("*/") {
                in_multiline = false;
                let after = line[end + 2..].trim();
                if !after.is_empty() {
                    result.push(after.to_string());
                }
            }
            continue;
        }

        let current = line.to_string();

        if let Some(pos) = current.find("//") {
            let before = current[..pos].trim_end().to_string();
            if !before.is_empty() {
                result.push(before);
            }
            continue;
        }

        if let Some(start) = current.find("/*") {
            let before = current[..start].trim_end().to_string();
            if !before.is_empty() {
                result.push(before);
            }
            if let Some(end) = current[start + 2..].find("*/") {
                let after = current[start + 2 + end + 2..].trim().to_string();
                if !after.is_empty() {
                    result.push(after);
                }
            } else {
                in_multiline = true;
            }
            continue;
        }

        result.push(current);
    }

    result
}

fn trim_empty(lines: &[String]) -> Vec<String> {
    let mut start = 0;
    let mut end = lines.len();
    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    let mut result = Vec::new();
    let mut prev_empty = false;
    for line in &lines[start..end] {
        if line.trim().is_empty() {
            if !prev_empty {
                result.push(String::new());
                prev_empty = true;
            }
        } else {
            result.push(line.clone());
            prev_empty = false;
        }
    }
    result
}

// ============================================================
// Разбор строк в блоки
// ============================================================

fn parse_lines(lines: &[String], doc: &mut Document, vars: &HashMap<String, String>) {
    let mut i = 0;
    while i < lines.len() {
        let stripped = lines[i].trim();

        if stripped.is_empty() {
            doc.add_node(NodeData::Paragraph(Vec::new()));
            i += 1;
            continue;
        }

        if stripped.starts_with("!!!") {
            i = parse_info_block(lines, i, doc, vars);
        } else if stripped.starts_with("???") {
            i = parse_spoiler_block(lines, i, doc, vars);
        } else if stripped.starts_with("%%") {
            i = parse_color_block(lines, i, doc, vars);
        } else if stripped.starts_with('>') {
            i = parse_quote_block(lines, i, doc, vars);
        } else if stripped.starts_with("```") {
            i = parse_code_block(lines, i, doc);
        } else if stripped.starts_with('|') {
            i = parse_table_block(lines, i, doc, vars);
        } else if stripped.starts_with("@@gopher-type") {
            i = parse_gopher_block(lines, i, doc);
        } else if stripped.starts_with("@@") {
            i = parse_form_block(lines, i, doc, vars);
        } else if is_list_item(&lines[i]) {
            i = parse_list_block(lines, i, doc, vars);
        } else {
            i = parse_normal_line(lines, i, doc, vars);
        }
    }
}

// ============================================================
// Блоки
// ============================================================

fn parse_info_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let stripped = lines[start].trim();
    let rest = stripped.trim_start_matches('!').trim();
    let attrs = parse_attrs(rest);
    let kind = match attrs.get("type").map(String::as_str) {
        Some("warning") => InfoKind::Warning,
        Some("success") => InfoKind::Success,
        Some("error") => InfoKind::Error,
        _ => InfoKind::Info,
    };

    let node = doc.add_node(NodeData::InfoBlock { kind });

    let mut i = start + 1;
    while i < lines.len() {
        if lines[i].trim().starts_with("!!!") {
            return i + 1;
        }
        add_paragraph_to(doc, node, &lines[i], vars);
        i += 1;
    }
    i
}

fn parse_spoiler_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let stripped = lines[start].trim();
    let rest = stripped.trim_start_matches('?').trim();
    let attrs = parse_attrs(rest);
    let title = attrs.get("title").cloned();

    let node = doc.add_node(NodeData::Spoiler { title });

    let mut i = start + 1;
    while i < lines.len() {
        if lines[i].trim().starts_with("???") {
            return i + 1;
        }
        add_paragraph_to(doc, node, &lines[i], vars);
        i += 1;
    }
    i
}

fn parse_color_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let stripped = lines[start].trim();
    let rest = stripped.trim_start_matches('%').trim();
    let attrs = parse_attrs(rest);

    let color = attrs
        .get("color")
        .and_then(|c| color_by_name(c))
        .unwrap_or(Color::White);

    let border = match attrs.get("border").map(String::as_str) {
        Some("left") => ColorBorder::Left,
        _ => ColorBorder::Rounded,
    };

    let node = doc.add_node(NodeData::ColorBlock { color, border });

    let mut i = start + 1;
    while i < lines.len() {
        if lines[i].trim().starts_with("%%") {
            return i + 1;
        }
        add_paragraph_to(doc, node, &lines[i], vars);
        i += 1;
    }
    i
}

fn parse_quote_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let mut cite = None;
    let mut date = None;

    let first = lines[start].trim();
    let after_gt = first.trim_start_matches('>').trim();
    if after_gt.starts_with('{') && after_gt.contains('}') {
        let attrs = parse_attrs(after_gt);
        cite = attrs.get("cite").cloned();
        date = attrs.get("date").cloned();
    }

    let node = doc.add_node(NodeData::BlockQuote { cite, date });

    let mut i = start;
    while i < lines.len() && lines[i].trim().starts_with('>') {
        let line = lines[i].trim();
        let after = line.trim_start_matches('>').trim();

        if after.starts_with('{') && after.contains('}') && i == start {
            i += 1;
            continue;
        }

        let spans = parse_inline(after, vars);
        if !spans.is_empty() {
            doc.add_child(node, NodeData::Paragraph(spans));
        }
        i += 1;
    }
    i
}

fn parse_code_block(lines: &[String], start: usize, doc: &mut Document) -> usize {
    let header = lines[start].trim().trim_start_matches('`').trim();
    let language = if header.is_empty() {
        None
    } else {
        Some(header.to_string())
    };

    let mut content = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        if lines[i].trim().starts_with("```") {
            i += 1;
            break;
        }
        content.push(lines[i].clone());
        i += 1;
    }

    doc.add_node(NodeData::CodeBlock {
        language,
        text: content.join("\n"),
    });

    i
}

fn parse_table_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let mut table_lines = Vec::new();
    let mut i = start;
    while i < lines.len() && lines[i].trim().starts_with('|') {
        table_lines.push(lines[i].clone());
        i += 1;
    }

    if table_lines.len() < 2 {
        for line in &table_lines {
            let spans = parse_inline(line, vars);
            doc.add_node(NodeData::Paragraph(spans));
        }
        return i;
    }

    let split_row = |line: &str| -> Vec<String> {
        let stripped = line.trim().trim_start_matches('|').trim_end_matches('|');
        stripped.split('|').map(|c| c.trim().to_string()).collect()
    };

    let header_cells = split_row(&table_lines[0]);
    let sep_cells = split_row(&table_lines[1]);

    let alignments: Vec<Alignment> = sep_cells
        .iter()
        .map(|c| {
            let start_colon = c.starts_with(':');
            let end_colon = c.ends_with(':');
            match (start_colon, end_colon) {
                (true, true) => Alignment::Center,
                (false, true) => Alignment::Right,
                _ => Alignment::Left,
            }
        })
        .collect();

    let headers: Vec<Vec<Span>> = header_cells
        .iter()
        .map(|c| parse_inline(c, vars))
        .collect();

    let mut rows = Vec::new();
    for line in &table_lines[2..] {
        let cells = split_row(line);
        let row: Vec<Vec<Span>> = cells.iter().map(|c| parse_inline(c, vars)).collect();
        rows.push(row);
    }

    doc.add_node(NodeData::Table {
        headers,
        rows,
        alignments,
    });

    i
}

fn parse_gopher_block(lines: &[String], start: usize, doc: &mut Document) -> usize {
    let header = lines[start].trim().to_string();
    let mut content = Vec::new();
    let mut i = start + 1;
    while i < lines.len() {
        if lines[i].trim().starts_with("@@endgopher") {
            i += 1;
            break;
        }
        content.push(lines[i].clone());
        i += 1;
    }
    doc.add_node(NodeData::GopherBlock {
        header,
        lines: content,
    });
    // Съедаем одну пустую строку сразу после блока
    if i < lines.len() && lines[i].trim().is_empty() {
        i += 1;
    }
    i
}

fn parse_form_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let stripped = lines[start].trim();
    let rest = stripped.trim_start_matches('@').trim();
    let attrs = parse_attrs(rest);

    let action = attrs.get("action").cloned().unwrap_or_default();
    let method = attrs.get("method").cloned().unwrap_or_else(|| "post".to_string());

    let mut items: Vec<FormItem> = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let raw = lines[i].as_str();
        let line = raw.trim();
        if line.starts_with("@@") {
            i += 1;
            break;
        }

        if line.is_empty() {
            items.push(FormItem::Line(Vec::new()));
            i += 1;
            continue;
        }

        if let Some((field, consumed)) = try_parse_block_field(lines, i, vars) {
            items.push(FormItem::Field(field));
            i += consumed;
            continue;
        }

        if let Some(field) = try_parse_single_field(line, vars) {
            items.push(FormItem::Field(field));
            i += 1;
            continue;
        }

        let spans = parse_inline(line, vars);
        items.push(FormItem::Line(spans));
        i += 1;
    }

    doc.add_node(NodeData::Form {
        action,
        method,
        items,
    });

    i
}

// ============================================================
// Нормальные строки
// ============================================================

fn parse_normal_line(
    lines: &[String],
    i: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let line = &lines[i];
    let stripped = line.trim();

    if stripped == "---" {
        doc.add_node(NodeData::HorizontalRule { thick: false });
        return i + 1;
    }
    if stripped == "***" {
        doc.add_node(NodeData::HorizontalRule { thick: true });
        return i + 1;
    }

    if let Some(level) = heading_level(stripped) {
        let text = stripped.trim_start_matches('#').trim_end_matches('#').trim();
        let spans = parse_inline(text, vars);
        doc.add_node(NodeData::Heading { level, spans });
        return i + 1;
    }

    if let Some((name, text)) = try_footnote_def(stripped) {
        let sup = superscript_digits(&name);
        let mut spans = vec![Span {
            text: format!("⁽ {} ⁾", sup),
            style: SpanStyle {
                bold: true,
                fg: Some(Color::Blue),
                ..Default::default()
            },
            link: None,
        }];
        spans.push(Span::raw(" — "));
        spans.extend(parse_inline(&text, vars));
        doc.add_node(NodeData::Paragraph(spans));
        return i + 1;
    }

    if let Some(text) = stripped.strip_prefix(':') {
        let spans = parse_inline(text.trim(), vars);
        doc.add_node(NodeData::Definition { term: spans });
        return i + 1;
    }

    if let Some((alt, src, width, height)) = try_parse_image(stripped, vars) {
        doc.add_node(NodeData::Image {
            alt,
            src,
            width,
            height,
        });
        return i + 1;
    }

    let spans = parse_inline(stripped, vars);
    if !spans.is_empty() {
        doc.add_node(NodeData::Paragraph(spans));
    }
    i + 1
}

fn add_paragraph_to(
    doc: &mut Document,
    parent: indextree::NodeId,
    line: &str,
    vars: &HashMap<String, String>,
) {
    let stripped = line.trim();
    if stripped.is_empty() {
        return;
    }
    let spans = parse_inline(stripped, vars);
    if !spans.is_empty() {
        doc.add_child(parent, NodeData::Paragraph(spans));
    }
}

// ============================================================
// Вспомогательное для строк
// ============================================================

fn heading_level(line: &str) -> Option<u8> {
    let mut count = 0u8;
    for c in line.chars() {
        if c == '#' {
            count += 1;
        } else {
            break;
        }
    }
    if (1..=6).contains(&count) {
        Some(count)
    } else {
        None
    }
}

fn try_footnote_def(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("[^")?;
    let close = rest.find("]:")?;
    let name = rest[..close].to_string();
    let text = rest[close + 2..].trim_start().to_string();
    Some((name, text))
}

fn try_parse_image(
    line: &str,
    vars: &HashMap<String, String>,
) -> Option<(String, String, Option<u16>, Option<u16>)> {
    let rest = line.strip_prefix("![")?;
    let close = rest.find("](")?;
    let alt = substitute_vars(&rest[..close], vars);
    let after = &rest[close + 2..];
    let paren = after.find(')')?;
    let mut src_full = after[..paren].to_string();
    src_full = substitute_vars(&src_full, vars);

    let mut width = None;
    let mut height = None;

    if let Some(start) = src_full.find('{') {
        if let Some(end) = src_full.find('}') {
            let attr_str: String = src_full[start + 1..end].to_string();
            src_full = format!("{}{}", &src_full[..start], &src_full[end + 1..]);
            for part in attr_str.split_whitespace() {
                if let Some((k, v)) = part.split_once('=') {
                    let v = v.trim_matches('"');
                    match k.trim() {
                        "width" => width = v.parse().ok(),
                        "height" => height = v.parse().ok(),
                        _ => {}
                    }
                }
            }
        }
    }

    Some((alt, src_full.trim().to_string(), width, height))
}

// ============================================================
// Списки
// ============================================================

fn is_list_item(line: &str) -> bool {
    let s = line.trim_start();
    if s.starts_with("- ") || s.starts_with("* ") {
        return true;
    }
    is_ordered_item(s).is_some()
}

fn is_ordered_item(s: &str) -> Option<(String, String)> {
    let sep = s.find(". ")?;
    let num_part = &s[..sep];
    let rest = &s[sep + 2..];

    if num_part.is_empty() {
        return None;
    }
    if !num_part.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    if !num_part.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return None;
    }

    Some((num_part.to_string(), rest.to_string()))
}

fn strip_list_marker(s: &str) -> Option<&str> {
    if let Some(r) = s.strip_prefix("- ") {
        return Some(r);
    }
    if let Some(r) = s.strip_prefix("* ") {
        return Some(r);
    }
    if let Some((_, r)) = is_ordered_item(s) {
        let offset = s.len() - r.len();
        return Some(&s[offset..]);
    }
    None
}

fn parse_list_block(
    lines: &[String],
    start: usize,
    doc: &mut Document,
    vars: &HashMap<String, String>,
) -> usize {
    let mut end = start;
    while end < lines.len() && is_list_item(&lines[end]) {
        end += 1;
    }
    let block = &lines[start..end];
    if block.is_empty() {
        return start;
    }

    let first = block[0].trim_start();
    let ordered = is_ordered_item(first).is_some();

    let mut counters: Vec<u64> = Vec::new();

    for line in block {
        let indent_chars = line.len() - line.trim_start().len();
        let level = indent_chars / 2;
        let indent = level as u8;
        let stripped = line.trim_start();

        let content = match strip_list_marker(stripped) {
            Some(c) => c,
            None => continue,
        };

        counters.truncate(level + 1);
        while counters.len() <= level {
            counters.push(0);
        }
        counters[level] += 1;

        let (marker, text) = if ordered {
            let path: String = if level == 0 {
                format!("{}.", counters[0])
            } else {
                counters[..=level]
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            };
            (ListMarker::Ordered(path), content)
        } else {
            (ListMarker::Bullet, content)
        };

        let spans = parse_inline(text, vars);
        doc.add_node(NodeData::ListItem {
            indent,
            marker,
            spans,
        });
    }

    end
}

// ============================================================
// Атрибуты
// ============================================================

fn parse_attrs(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let s = text.trim();
    let s = s.trim_start_matches('{').trim_end_matches('}').trim();

    let mut chars = s.chars().peekable();
    let mut buf = String::new();
    let mut tokens: Vec<String> = Vec::new();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                buf.push('"');
                for nc in chars.by_ref() {
                    buf.push(nc);
                    if nc == '"' {
                        break;
                    }
                }
            }
            ' ' | '\t' => {
                if !buf.is_empty() {
                    tokens.push(std::mem::take(&mut buf));
                }
            }
            _ => buf.push(c),
        }
    }
    if !buf.is_empty() {
        tokens.push(buf);
    }

    for tok in tokens {
        if let Some((k, v)) = tok.split_once('=') {
            let v = v.trim_matches('"').to_string();
            map.insert(k.trim().to_string(), v);
        } else {
            map.insert(tok.trim().to_string(), "true".to_string());
        }
    }

    map
}

fn color_by_name(name: &str) -> Option<Color> {
    Some(match name {
        "black" => Color::Black,
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::Gray,
        "bblack" => Color::DarkGray,
        "bred" => Color::LightRed,
        "bgreen" => Color::LightGreen,
        "byellow" => Color::LightYellow,
        "bblue" => Color::LightBlue,
        "bmagenta" => Color::LightMagenta,
        "bcyan" => Color::LightCyan,
        "bwhite" => Color::White,
        _ => return None,
    })
}

// ============================================================
// Поля формы
// ============================================================

fn try_parse_block_field(
    lines: &[String],
    start: usize,
    vars: &HashMap<String, String>,
) -> Option<(FormField, usize)> {
    let stripped = lines[start].trim();

    if let Some(rest) = stripped.strip_prefix("[radio") {
        if let Some(name) = extract_attr_string(rest, "name") {
            let mut options = Vec::new();
            let mut i = start + 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l == "[/radio]" {
                    i += 1;
                    break;
                }
                if let Some((label, attrs)) = parse_option_line(l) {
                    let value = attrs.get("value").cloned().unwrap_or_else(|| label.clone());
                    let checked = attrs.contains_key("checked");
                    options.push(RadioOption {
                        label: substitute_vars(&label, vars),
                        value: substitute_vars(&value, vars),
                        checked,
                    });
                }
                i += 1;
            }
            return Some((FormField::Radio { name, options }, i - start));
        }
    }

    if let Some(rest) = stripped.strip_prefix("[select") {
        if let Some(name) = extract_attr_string(rest, "name") {
            let multiple = rest.contains("multiple");
            let mut options = Vec::new();
            let mut i = start + 1;
            while i < lines.len() {
                let l = lines[i].trim();
                if l == "[/select]" {
                    i += 1;
                    break;
                }
                if let Some((label, attrs)) = parse_option_line(l) {
                    let value = attrs.get("value").cloned().unwrap_or_else(|| label.clone());
                    let selected = attrs.contains_key("selected");
                    options.push(SelectOption {
                        label: substitute_vars(&label, vars),
                        value: substitute_vars(&value, vars),
                        selected,
                    });
                }
                i += 1;
            }
            return Some((
                FormField::Select {
                    name,
                    options,
                    multiple,
                },
                i - start,
            ));
        }
    }

    None
}

fn parse_option_line(line: &str) -> Option<(String, HashMap<String, String>)> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    let label = rest[..close].to_string();
    let after = rest[close + 1..].trim();
    let attrs = if after.starts_with('{') {
        parse_attrs(after)
    } else {
        HashMap::new()
    };
    Some((label, attrs))
}

fn extract_attr_string(s: &str, key: &str) -> Option<String> {
    let pattern = format!("{}=\"", key);
    let pos = s.find(&pattern)?;
    let after = &s[pos + pattern.len()..];
    let end = after.find('"')?;
    Some(after[..end].to_string())
}

fn try_parse_single_field(line: &str, vars: &HashMap<String, String>) -> Option<FormField> {
    let rest = line.strip_prefix('[')?;
    let close = rest.find(']')?;
    let label_raw = rest[..close].to_string();
    let after = rest[close + 1..].trim();

    if !after.starts_with('{') {
        return None;
    }

    let attrs = parse_attrs(after);

    let field_type = attrs.get("type").map(String::as_str).unwrap_or("text");

    let label = if label_raw.trim().is_empty() {
        attrs.get("label").cloned().unwrap_or_default()
    } else {
        label_raw.trim().to_string()
    };

    let name = attrs.get("name").cloned().unwrap_or_default();
    let placeholder = attrs.get("placeholder").cloned().unwrap_or_default();
    let value = attrs.get("value").cloned().unwrap_or_default();
    let required = attrs.contains_key("required");
    let checked = attrs.contains_key("checked");

    Some(match field_type {
        "email" => FormField::Text {
            label,
            name,
            placeholder,
            required,
            value,
            kind: FieldKind::Email,
        },
        "password" => FormField::Text {
            label,
            name,
            placeholder,
            required,
            value,
            kind: FieldKind::Password,
        },
        "textarea" => {
            let rows = attrs.get("rows").and_then(|s| s.parse().ok()).unwrap_or(3);
            FormField::Textarea {
                label,
                name,
                rows,
                placeholder,
                required,
                value,
            }
        }
        "checkbox" => FormField::Checkbox {
            label,
            name,
            checked,
            required,
        },
        "submit" => FormField::Submit { label },
        "reset" => FormField::Reset { label },
        "hidden" => FormField::Hidden { name, value },
        _ => FormField::Text {
            label,
            name,
            placeholder,
            required,
            value,
            kind: FieldKind::Text,
        },
    })
    .map(|f| substitute_field_vars(f, vars))
}

fn substitute_field_vars(field: FormField, vars: &HashMap<String, String>) -> FormField {
    match field {
        FormField::Text {
            label,
            name,
            placeholder,
            required,
            value,
            kind,
        } => FormField::Text {
            label: substitute_vars(&label, vars),
            name,
            placeholder: substitute_vars(&placeholder, vars),
            required,
            value: substitute_vars(&value, vars),
            kind,
        },
        FormField::Textarea {
            label,
            name,
            rows,
            placeholder,
            required,
            value,
        } => FormField::Textarea {
            label: substitute_vars(&label, vars),
            name,
            rows,
            placeholder: substitute_vars(&placeholder, vars),
            required,
            value: substitute_vars(&value, vars),
        },
        FormField::Checkbox {
            label,
            name,
            checked,
            required,
        } => FormField::Checkbox {
            label: substitute_vars(&label, vars),
            name,
            checked,
            required,
        },
        FormField::Radio { name, options } => FormField::Radio {
            name,
            options: options
                .into_iter()
                .map(|o| RadioOption {
                    label: substitute_vars(&o.label, vars),
                    value: o.value,
                    checked: o.checked,
                })
                .collect(),
        },
        FormField::Select {
            name,
            options,
            multiple,
        } => FormField::Select {
            name,
            options: options
                .into_iter()
                .map(|o| SelectOption {
                    label: substitute_vars(&o.label, vars),
                    value: o.value,
                    selected: o.selected,
                })
                .collect(),
            multiple,
        },
        FormField::Submit { label } => FormField::Submit {
            label: substitute_vars(&label, vars),
        },
        FormField::Reset { label } => FormField::Reset {
            label: substitute_vars(&label, vars),
        },
        FormField::Hidden { name, value } => FormField::Hidden {
            name,
            value: substitute_vars(&value, vars),
        },
    }
}

// ============================================================
// Переменные
// ============================================================

fn substitute_vars(text: &str, vars: &HashMap<String, String>) -> String {
    if vars.is_empty() || !text.contains("{{") {
        return text.to_string();
    }

    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = text.chars().collect();

    while i < chars.len() {
        if i + 1 < chars.len() && chars[i] == '{' && chars[i + 1] == '{' {
            let mut j = i + 2;
            let mut name = String::new();
            while j < chars.len() {
                if j + 1 < chars.len() && chars[j] == '}' && chars[j + 1] == '}' {
                    break;
                }
                name.push(chars[j]);
                j += 1;
            }
            if j + 1 < chars.len() {
                let key = name.trim().to_string();
                if let Some(v) = vars.get(&key) {
                    result.push_str(v);
                } else {
                    result.push_str("{{");
                    result.push_str(&name);
                    result.push_str("}}");
                }
                i = j + 2;
                continue;
            }
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

// ============================================================
// Инлайн-парсер
// ============================================================

fn parse_inline(text: &str, vars: &HashMap<String, String>) -> Vec<Span> {
    let substituted = substitute_vars(text, vars);
    let chars: Vec<char> = substituted.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut buf = String::new();
    let mut style = SpanStyle::default();
    let mut i = 0;

    while i < chars.len() {
        let rem = chars.len() - i;

        // [ ] [x] [~] — как текст, не как стили
        if rem >= 3 && chars[i] == '[' && chars[i + 2] == ']' {
            let mid = chars[i + 1];
            if mid == ' ' || mid == 'x' || mid == 'X' || mid == '~' {
                buf.push('[');
                buf.push(mid);
                buf.push(']');
                i += 3;
                continue;
            }
        }

        if rem >= 3 && chars[i] == '*' && chars[i + 1] == '*' && chars[i + 2] == '*' {
            flush(&mut spans, &mut buf, &style);
            style.bold = !style.bold;
            style.italic = !style.italic;
            i += 3;
            continue;
        }
        if rem >= 2 && chars[i] == '*' && chars[i + 1] == '*' {
            flush(&mut spans, &mut buf, &style);
            style.bold = !style.bold;
            i += 2;
            continue;
        }
        if rem >= 2 && chars[i] == '~' && chars[i + 1] == '~' {
            flush(&mut spans, &mut buf, &style);
            style.strikethrough = !style.strikethrough;
            i += 2;
            continue;
        }
        if rem >= 2 && chars[i] == '=' && chars[i + 1] == '=' {
            flush(&mut spans, &mut buf, &style);
            if style.bg.is_none() {
                style.bg = Some(Color::Yellow);
                style.fg = Some(Color::Black);
            } else {
                style.bg = None;
                style.fg = None;
            }
            i += 2;
            continue;
        }

        match chars[i] {
            '*' => {
                flush(&mut spans, &mut buf, &style);
                style.italic = !style.italic;
                i += 1;
            }
            '`' => {
                flush(&mut spans, &mut buf, &style);
                style.code = !style.code;
                if style.code {
                    style.fg = Some(Color::Yellow);
                } else if style.fg == Some(Color::Yellow) {
                    style.fg = None;
                }
                i += 1;
            }
            '^' => {
                flush(&mut spans, &mut buf, &style);
                style.superscript = !style.superscript;
                i += 1;
            }
            '~' => {
                flush(&mut spans, &mut buf, &style);
                style.subscript = !style.subscript;
                i += 1;
            }
            '{' => {
                if let Some((consumed, tag)) = parse_color_tag(&chars[i..]) {
                    flush(&mut spans, &mut buf, &style);
                    apply_color_tag(&mut style, tag);
                    i += consumed;
                } else {
                    buf.push('{');
                    i += 1;
                }
            }
            '[' => {
                if let Some((consumed, content)) = try_parse_link(&chars[i..]) {
                    flush(&mut spans, &mut buf, &style);
                    match content {
                        LinkContent::Link {
                            text,
                            url,
                            tooltip,
                        } => {
                            let inner = parse_inline(&text, &HashMap::new());
                            for mut s in inner {
                                s.style.bold |= style.bold;
                                s.style.italic |= style.italic;
                                s.style.strikethrough |= style.strikethrough;
                                s.style.superscript |= style.superscript;
                                s.style.subscript |= style.subscript;
                                if s.style.fg.is_none() {
                                    s.style.fg = style.fg;
                                }
                                if s.style.bg.is_none() {
                                    s.style.bg = style.bg;
                                }
                                if s.style.fg.is_none() {
                                    s.style.fg = Some(Color::Cyan);
                                }
                                s.link = Some(LinkInfo {
                                    url: url.clone(),
                                    tooltip: tooltip.clone(),
                                });
                                spans.push(s);
                            }
                        }
                        LinkContent::Footnote(name) => {
                            let mut s = style;
                            s.bold = true;
                            s.fg = Some(Color::Blue);
                            let sup = superscript_digits(&name);
                            spans.push(Span {
                                text: format!("⁽ {} ⁾", sup),
                                style: s,
                                link: None,
                            });
                        }
                    }
                    i += consumed;
                } else {
                    buf.push('[');
                    i += 1;
                }
            }
            '\\' if i + 1 < chars.len() => {
                buf.push(chars[i + 1]);
                i += 2;
            }
            _ => {
                buf.push(chars[i]);
                i += 1;
            }
        }
    }

    flush(&mut spans, &mut buf, &style);
    spans
}

fn flush(spans: &mut Vec<Span>, buf: &mut String, style: &SpanStyle) {
    if buf.is_empty() {
        return;
    }
    spans.push(Span {
        text: std::mem::take(buf),
        style: *style,
        link: None,
    });
}

// ============================================================
// Цвета
// ============================================================

enum ColorTag {
    Fg(Color),
    Bg(Color),
    ResetAll,
    ResetFg,
    ResetBg,
}

fn parse_color_tag(chars: &[char]) -> Option<(usize, ColorTag)> {
    let mut i = 1;
    let mut name = String::new();
    while i < chars.len() && chars[i] != '}' {
        name.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() {
        return None;
    }
    let consumed = i + 1;
    let name = name.trim();

    if name == "/" {
        return Some((consumed, ColorTag::ResetAll));
    }
    if let Some(rest) = name.strip_prefix('/') {
        if rest == "bg" || rest.starts_with("bg-") {
            return Some((consumed, ColorTag::ResetBg));
        }
        return Some((consumed, ColorTag::ResetFg));
    }
    if let Some(rest) = name.strip_prefix("bg-") {
        if let Some(c) = color_by_name(rest) {
            return Some((consumed, ColorTag::Bg(c)));
        }
    }
    if let Some(c) = color_by_name(name) {
        return Some((consumed, ColorTag::Fg(c)));
    }

    None
}

fn apply_color_tag(style: &mut SpanStyle, tag: ColorTag) {
    match tag {
        ColorTag::Fg(c) => style.fg = Some(c),
        ColorTag::Bg(c) => style.bg = Some(c),
        ColorTag::ResetAll => {
            let keep_bold = style.bold;
            let keep_italic = style.italic;
            *style = SpanStyle::default();
            style.bold = keep_bold;
            style.italic = keep_italic;
        }
        ColorTag::ResetFg => style.fg = None,
        ColorTag::ResetBg => style.bg = None,
    }
}

// ============================================================
// Ссылки и сноски
// ============================================================

enum LinkContent {
    Link {
        text: String,
        url: String,
        tooltip: Option<String>,
    },
    Footnote(String),
}

fn try_parse_link(chars: &[char]) -> Option<(usize, LinkContent)> {
    if chars.len() < 2 || chars[0] != '[' {
        return None;
    }

    if chars[1] == '^' {
        let mut i = 2;
        let mut name = String::new();
        while i < chars.len() && chars[i] != ']' {
            name.push(chars[i]);
            i += 1;
        }
        if i >= chars.len() {
            return None;
        }
        return Some((i + 1, LinkContent::Footnote(name)));
    }

    let mut i = 1;
    let mut text = String::new();
    let mut depth = 0;
    while i < chars.len() {
        match chars[i] {
            '[' => {
                depth += 1;
                text.push('[');
            }
            ']' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                text.push(']');
            }
            c => text.push(c),
        }
        i += 1;
    }
    if i >= chars.len() || chars[i] != ']' {
        return None;
    }
    i += 1;
    if i >= chars.len() || chars[i] != '(' {
        return None;
    }
    i += 1;

    let mut url = String::new();
    while i < chars.len() && chars[i] != ')' && chars[i] != ' ' && chars[i] != '"' {
        url.push(chars[i]);
        i += 1;
    }
    while i < chars.len() && chars[i] == ' ' {
        i += 1;
    }

    let mut tooltip = None;
    if i < chars.len() && chars[i] == '"' {
        i += 1;
        let mut t = String::new();
        while i < chars.len() && chars[i] != '"' {
            t.push(chars[i]);
            i += 1;
        }
        if i < chars.len() {
            i += 1;
        }
        tooltip = Some(t);
    }

    if i >= chars.len() || chars[i] != ')' {
        return None;
    }
    i += 1;

    Some((
        i,
        LinkContent::Link {
            text,
            url,
            tooltip,
        },
    ))
}

// ============================================================
// Сноски: суперскрипт
// ============================================================

fn superscript_digits(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            '+' => '⁺',
            '-' => '⁻',
            other => other,
        })
        .collect()
}