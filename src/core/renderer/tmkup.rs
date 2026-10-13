use anyhow::Result;
use async_trait::async_trait;
use indextree::NodeId;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span as RatSpan, Text},
};
use url::Url;

use crate::core::document::{
    Alignment, ColorBorder, Document, FieldKind, FormField, FormItem, InfoKind, ListMarker,
    NodeData, Span, SpanStyle,
};
use crate::core::renderer::base::Renderer;
use crate::core::renderer::code::get_code_renderer;
use crate::core::renderer::meta::{
    FieldValue, InteractiveFieldKind, LineMeta, Link, RenderState, RenderedContent,
};
use crate::core::renderer::theme::RendererTheme;

pub struct TmkupRenderer;

struct Out {
    lines: Vec<Line<'static>>,
    meta: Vec<LineMeta>,
}

impl Out {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            meta: Vec::new(),
        }
    }

    fn push(&mut self, line: Line<'static>, meta: LineMeta) {
        self.lines.push(line);
        self.meta.push(meta);
    }

    fn push_normal(&mut self, line: Line<'static>) {
        self.push(line, LineMeta::normal());
    }

    fn extend_normal(&mut self, lines: Vec<Line<'static>>) {
        for l in lines {
            self.push_normal(l);
        }
    }
}

#[async_trait]
impl Renderer for TmkupRenderer {
    fn content_type(&self) -> &str {
        "text/tmkup"
    }

    async fn render(
        &self,
        doc: &Document,
        base_url: &str,
        _theme: &dyn RendererTheme,
        viewport_width: u16,
        state: &RenderState,
    ) -> Result<RenderedContent> {
        let mut out = Out::new();
        render_children(doc, doc.root(), &mut out, viewport_width, state, base_url);
        Ok(RenderedContent::new(Text::from(out.lines), out.meta))
    }
}

fn resolve_href(base_url: &str, href: &str) -> String {
    if href.is_empty() {
        return base_url.to_string();
    }
    if let Ok(base) = Url::parse(base_url) {
        if let Ok(joined) = base.join(href) {
            return joined.to_string();
        }
    }
    href.to_string()
}

fn collect_links(spans: &[Span], base_url: &str) -> Vec<Link> {
    spans
        .iter()
        .filter_map(|s| {
            s.link.as_ref().map(|l| Link {
                url: resolve_href(base_url, &l.url),
                tooltip: l.tooltip.clone(),
            })
        })
        .collect()
}

fn render_children(
    doc: &Document,
    parent: NodeId,
    out: &mut Out,
    vw: u16,
    state: &RenderState,
    base_url: &str,
) {
    for (id, node) in doc.children(parent) {
        render_node(doc, id, node, out, vw, state, base_url);
    }
}

fn render_node(
    doc: &Document,
    id: NodeId,
    node: &NodeData,
    out: &mut Out,
    vw: u16,
    state: &RenderState,
    base_url: &str,
) {
    match node {
        NodeData::Root => {}

        NodeData::Paragraph(spans) => {
            if spans.is_empty() {
                out.push_normal(Line::from(""));
            } else {
                let links = collect_links(spans, base_url);
                out.push(
                    Line::from(spans_to_ratatui(spans)),
                    LineMeta::with_links(links),
                );
            }
        }

        NodeData::Heading { spans, .. } => {
            let style = Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD);
            let rat: Vec<RatSpan<'static>> = spans
                .iter()
                .map(|s| RatSpan::styled(sanitize(&s.text), style))
                .collect();
            out.push_normal(Line::from(rat));
        }

        NodeData::ListItem {
            indent,
            marker,
            spans,
        } => {
            let m = match marker {
                ListMarker::Bullet => "•".to_string(),
                ListMarker::Ordered(path) => path.clone(),
            };
            let prefix = format!("{}{} ", "  ".repeat(*indent as usize), m);
            let links = collect_links(spans, base_url);
            let mut rat = vec![RatSpan::raw(prefix)];
            rat.extend(spans_to_ratatui(spans));
            out.push(Line::from(rat), LineMeta::with_links(links));
        }

        NodeData::Definition { term } => {
            let italic = Style::default().add_modifier(Modifier::ITALIC);
            let mut rat = vec![RatSpan::raw("    — ")];
            for s in term {
                rat.push(RatSpan::styled(sanitize(&s.text), italic));
            }
            out.push_normal(Line::from(rat));
        }

        NodeData::HorizontalRule { thick } => {
            let ch = if *thick { '━' } else { '─' };
            let width = ((vw as usize) * 3 / 4).max(1);
            out.push_normal(Line::from(ch.to_string().repeat(width)));
        }

        NodeData::CodeBlock { language, text } => {
            let lang = language.as_deref().unwrap_or("");
            let lines: Vec<String> = text.lines().map(|l| l.to_string()).collect();

            let renderer = get_code_renderer(lang);
            let result = renderer.render(&lines, lang);

            let content: Vec<Vec<Span>> = result
                .text
                .lines
                .into_iter()
                .map(|line| {
                    line.spans
                        .into_iter()
                        .map(|rs| Span {
                            text: rs.content.into_owned(),
                            style: crate::core::document::SpanStyle {
                                fg: rs.style.fg,
                                bg: rs.style.bg,
                                bold: rs.style.add_modifier.contains(Modifier::BOLD),
                                italic: rs.style.add_modifier.contains(Modifier::ITALIC),
                                ..Default::default()
                            },
                            link: None,
                        })
                        .collect()
                })
                .collect();

            let frame = build_frame(
                &result.label,
                &content,
                Color::White,
                result.bg,
                result.fg,
                FrameBorder::Rounded,
                None,
            );
            out.extend_normal(frame);
        }

        NodeData::Image { alt, .. } => {
            out.push_normal(Line::from(format!("[{}]", alt)));
        }

        NodeData::BlockQuote { cite, date } => {
            let mut content: Vec<Vec<Span>> = Vec::new();
            for (_, child) in doc.children(id) {
                if let NodeData::Paragraph(spans) = child {
                    content.push(spans.clone());
                }
            }

            if let Some(c) = cite {
                let author_text = match date {
                    Some(d) => format!("— {} ({})", c, d),
                    None => format!("— {}", c),
                };

                let mut max_len = 0usize;
                for spans in &content {
                    let len: usize = spans.iter().map(|s| s.text.chars().count()).sum();
                    if len > max_len {
                        max_len = len;
                    }
                }
                let author_len = author_text.chars().count();
                let padding = max_len.saturating_sub(author_len);
                let padded = format!("{}{}", " ".repeat(padding), author_text);

                content.push(vec![Span::raw(padded)]);
            }

            let frame = build_frame(
                "",
                &content,
                Color::White,
                Color::White,
                Color::Black,
                FrameBorder::Left,
                None,
            );
            out.extend_normal(frame);
        }

        NodeData::InfoBlock { kind } => {
            let (label, color) = match kind {
                InfoKind::Info => ("INFO", Color::Blue),
                InfoKind::Warning => ("WARN", Color::Yellow),
                InfoKind::Success => (" OK ", Color::Green),
                InfoKind::Error => ("ERR ", Color::Red),
            };
            let mut content: Vec<Vec<Span>> = Vec::new();
            for (_, child) in doc.children(id) {
                if let NodeData::Paragraph(spans) = child {
                    content.push(spans.clone());
                }
            }
            let frame = build_frame(
                label,
                &content,
                color,
                color,
                Color::White,
                FrameBorder::Rounded,
                Some(color),
            );
            out.extend_normal(frame);
        }

        NodeData::ColorBlock { color, border } => {
            let mut content: Vec<Vec<Span>> = Vec::new();
            for (_, child) in doc.children(id) {
                if let NodeData::Paragraph(spans) = child {
                    content.push(spans.clone());
                }
            }
            let border_style = match border {
                ColorBorder::Left => FrameBorder::Left,
                ColorBorder::Rounded => FrameBorder::Rounded,
            };
            let frame = build_frame(
                "",
                &content,
                *color,
                *color,
                Color::Black,
                border_style,
                Some(*color),
            );
            out.extend_normal(frame);
        }

        NodeData::Spoiler { title } => {
            let expanded = state.spoilers.get(&id).copied().unwrap_or(false);
            let t = title.as_deref().unwrap_or("Спойлер");
            let marker = if expanded { "▼" } else { "▶" };
            let header = format!("[{}] {}", marker, t);

            let mut content: Vec<Vec<Span>> = Vec::new();
            content.push(vec![Span::raw(header)]);

            if expanded {
                content.push(vec![Span::raw("")]);
                for (_, child) in doc.children(id) {
                    if let NodeData::Paragraph(spans) = child {
                        content.push(spans.clone());
                    }
                }
            }

            let frame = build_frame(
                "SPOILER",
                &content,
                Color::White,
                Color::White,
                Color::Black,
                FrameBorder::Rounded,
                None,
            );

            for (i, line) in frame.into_iter().enumerate() {
                if i == 1 {
                    out.push(line, LineMeta::spoiler(id, expanded));
                } else {
                    out.push_normal(line);
                }
            }
        }

        NodeData::Table {
            headers,
            rows,
            alignments,
        } => {
            render_table(headers, rows, alignments, out);
        }

        NodeData::Form { items, .. } => {
            render_form(id, items, out, state);
        }

        NodeData::GopherBlock { .. } => {}
    }
}

fn render_form(form_id: NodeId, items: &[FormItem], out: &mut Out, state: &RenderState) {
    let mut content: Vec<Vec<Span>> = Vec::new();
    let mut metas: Vec<LineMeta> = Vec::new();
    let mut field_index = 0usize;

    for item in items {
        match item {
            FormItem::Line(spans) => {
                content.push(spans.clone());
                metas.push(LineMeta::normal());
            }
            FormItem::Field(field) => {
                let (lines, line_metas) = field_to_lines(field, state, form_id, field_index);
                content.extend(lines);
                metas.extend(line_metas);
                field_index += 1;
            }
        }
    }

    let frame = build_frame(
        " FORM ",
        &content,
        Color::White,
        Color::White,
        Color::Black,
        FrameBorder::Rounded,
        None,
    );

    let total = frame.len();
    for (i, line) in frame.into_iter().enumerate() {
        if i == 0 || i == total - 1 {
            out.push_normal(line);
        } else {
            let content_idx = i - 1;
            let meta = metas
                .get(content_idx)
                .cloned()
                .unwrap_or_else(LineMeta::normal);
            out.push(line, meta);
        }
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

enum FrameBorder {
    Rounded,
    Left,
}

fn build_frame(
    label: &str,
    content: &[Vec<Span>],
    border_color: Color,
    label_bg: Color,
    label_fg: Color,
    border: FrameBorder,
    content_fg: Option<Color>,
) -> Vec<Line<'static>> {
    if matches!(border, FrameBorder::Left) {
        let mut out = Vec::new();
        for spans in content {
            let mut rat = vec![RatSpan::styled("┃ ", Style::default().fg(border_color))];
            for s in spans {
                let mut style = span_style(s);
                if s.style.fg.is_none() {
                    if let Some(fg) = content_fg {
                        style = style.fg(fg);
                    }
                }
                rat.push(RatSpan::styled(sanitize(&s.text), style));
            }
            out.push(Line::from(rat));
        }
        return out;
    }

    let mut content_width = 0usize;
    for spans in content {
        let len: usize = spans.iter().map(|s| s.text.chars().count()).sum();
        content_width = content_width.max(len);
    }

    let label_fmt = if label.is_empty() {
        String::new()
    } else {
        format!(" {} ", label)
    };
    let label_len = label_fmt.chars().count();

    let width = content_width + 6;
    let inner = content_width;
    if width < 4 {
        return Vec::new();
    }

    let border_style = Style::default().fg(border_color);
    let mut out = Vec::new();

    let mut top: Vec<RatSpan<'static>> = Vec::new();
    top.push(RatSpan::styled("┌─".to_string(), border_style));
    if !label.is_empty() {
        top.push(RatSpan::styled(
            label_fmt.clone(),
            Style::default().fg(label_fg).bg(label_bg),
        ));
        let after_len = width - 2 - label_len;
        let after = format!("{}┐", "─".repeat(after_len + 1));
        top.push(RatSpan::styled(after, border_style));
    } else {
        let after = format!("{}┐", "─".repeat(width - 1));
        top.push(RatSpan::styled(after, border_style));
    }
    out.push(Line::from(top));

    for spans in content {
        let mut rat: Vec<RatSpan<'static>> = Vec::new();
        rat.push(RatSpan::styled("│   ".to_string(), border_style));
        let mut used = 0usize;
        for s in spans {
            let mut style = span_style(s);
            if s.style.fg.is_none() {
                if let Some(fg) = content_fg {
                    style = style.fg(fg);
                }
            }
            rat.push(RatSpan::styled(sanitize(&s.text), style));
            used += s.text.chars().count();
        }
        if used < inner {
            rat.push(RatSpan::raw(" ".repeat(inner - used)));
        }
        rat.push(RatSpan::styled("   │".to_string(), border_style));
        out.push(Line::from(rat));
    }

    let bottom = format!("└{}┘", "─".repeat(width));
    out.push(Line::from(RatSpan::styled(bottom, border_style)));

    out
}

fn render_table(
    headers: &[Vec<Span>],
    rows: &[Vec<Vec<Span>>],
    alignments: &[Alignment],
    out: &mut Out,
) {
    let header_texts: Vec<String> = headers
        .iter()
        .map(|c| c.iter().map(|s| sanitize(&s.text)).collect())
        .collect();

    let row_texts: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| c.iter().map(|s| sanitize(&s.text)).collect::<String>())
                .collect()
        })
        .collect();

    let cols = headers
        .len()
        .max(row_texts.iter().map(|r| r.len()).max().unwrap_or(0));
    if cols == 0 {
        return;
    }

    let mut widths = vec![0usize; cols];
    for (i, h) in header_texts.iter().enumerate() {
        widths[i] = widths[i].max(h.chars().count());
    }
    for row in &row_texts {
        for (i, c) in row.iter().enumerate() {
            widths[i] = widths[i].max(c.chars().count());
        }
    }

    let make_border = |l: char, m: char, r: char, fill: char| -> String {
        let parts: Vec<String> = widths
            .iter()
            .map(|w| fill.to_string().repeat(w + 2))
            .collect();
        format!("{}{}{}", l, parts.join(&m.to_string()), r)
    };

    let row_str = |cells: &[String]| -> String {
        let mut parts = Vec::new();
        for i in 0..cols {
            let c = cells.get(i).map(String::as_str).unwrap_or("");
            let w = widths[i];
            let visible = c.chars().count();
            let pad = w.saturating_sub(visible);
            let align = alignments.get(i).copied().unwrap_or(Alignment::Left);
            let padded = match align {
                Alignment::Left => format!("{}{}", c, " ".repeat(pad)),
                Alignment::Right => format!("{}{}", " ".repeat(pad), c),
                Alignment::Center => {
                    let left = pad / 2;
                    let right = pad - left;
                    format!("{}{}{}", " ".repeat(left), c, " ".repeat(right))
                }
            };
            parts.push(format!(" {} ", padded));
        }
        format!("│{}│", parts.join("│"))
    };

    out.push_normal(Line::from(make_border('╔', '╦', '╗', '═')));
    if !header_texts.is_empty() {
        out.push_normal(Line::from(row_str(&header_texts)));
        out.push_normal(Line::from(make_border('╠', '╬', '╣', '═')));
    }
    for row in &row_texts {
        out.push_normal(Line::from(row_str(row)));
    }
    out.push_normal(Line::from(make_border('╚', '╩', '╝', '═')));
}

fn required_marker(form_id: NodeId, field_index: usize, state: &RenderState) -> Span {
    let has_err = state.field_errors.contains(&(form_id, field_index));
    Span {
        text: "*".to_string(),
        style: SpanStyle {
            reversed: has_err,
            ..Default::default()
        },
        link: None,
    }
}

fn field_to_lines(
    field: &FormField,
    state: &RenderState,
    form_id: NodeId,
    field_index: usize,
) -> (Vec<Vec<Span>>, Vec<LineMeta>) {
    let key = (form_id, field_index);

    match field {
        FormField::Text {
            label,
            placeholder,
            required,
            value,
            kind,
            ..
        } => {
            let display = match state.fields.get(&key) {
                Some(FieldValue::Text(t)) => {
                    if t.is_empty() {
                        if !placeholder.is_empty() {
                            placeholder.clone()
                        } else {
                            "____".to_string()
                        }
                    } else {
                        match kind {
                            FieldKind::Password => "*".repeat(t.chars().count()),
                            _ => t.clone(),
                        }
                    }
                }
                _ => {
                    if !value.is_empty() {
                        match kind {
                            FieldKind::Password => "*".repeat(value.chars().count()),
                            _ => value.clone(),
                        }
                    } else if !placeholder.is_empty() {
                        placeholder.clone()
                    } else {
                        "____".to_string()
                    }
                }
            };

            let has_err = state.field_errors.contains(&key);

            let mut spans: Vec<Span> = Vec::new();
            if matches!(kind, FieldKind::Email) && has_err {
                spans.push(Span::raw(format!("{}: ", label)));
                spans.push(Span {
                    text: display,
                    style: SpanStyle {
                        reversed: true,
                        ..Default::default()
                    },
                    link: None,
                });
                if *required {
                    spans.push(Span::raw(" "));
                    spans.push(required_marker(form_id, field_index, state));
                }
                spans.push(Span::raw(" (Needs to be an email)"));
            } else {
                spans.push(Span::raw(format!("{}: {} ", label, display)));
                if *required {
                    spans.push(required_marker(form_id, field_index, state));
                }
            }

            let ifk = match kind {
                FieldKind::Email => InteractiveFieldKind::Email,
                FieldKind::Password => InteractiveFieldKind::Password,
                FieldKind::Text => InteractiveFieldKind::Text,
            };
            (
                vec![spans],
                vec![LineMeta::form_text(form_id, field_index, ifk)],
            )
        }
        FormField::Textarea {
            label,
            required,
            placeholder,
            value,
            ..
        } => {
            let display = match state.fields.get(&key) {
                Some(FieldValue::Text(t)) if !t.is_empty() => t.clone(),
                _ => {
                    if value.is_empty() {
                        placeholder.clone()
                    } else {
                        value.clone()
                    }
                }
            };

            let mut content: Vec<Vec<Span>> = Vec::new();
            let mut metas: Vec<LineMeta> = Vec::new();

            let mut head = vec![Span::raw(format!("{}:", label))];
            if *required {
                head.push(Span::raw(" "));
                head.push(required_marker(form_id, field_index, state));
            }
            content.push(head);
            metas.push(LineMeta::form_text(
                form_id,
                field_index,
                InteractiveFieldKind::Textarea,
            ));

            if display.is_empty() {
                content.push(vec![Span::raw(String::new())]);
                metas.push(LineMeta::form_text(
                    form_id,
                    field_index,
                    InteractiveFieldKind::Textarea,
                ));
            } else {
                for line in display.split('\n') {
                    content.push(vec![Span::raw(format!("    {}", line))]);
                    metas.push(LineMeta::form_text(
                        form_id,
                        field_index,
                        InteractiveFieldKind::Textarea,
                    ));
                }
            }

            (content, metas)
        }
        FormField::Checkbox {
            label,
            checked,
            required,
            ..
        } => {
            let display_checked = match state.fields.get(&key) {
                Some(FieldValue::Checked(b)) => *b,
                _ => *checked,
            };
            let mark = if display_checked { "[x]" } else { "[ ]" };
            let mut spans = vec![Span::raw(format!("{} {}", mark, label))];
            if *required {
                spans.push(Span::raw(" "));
                spans.push(required_marker(form_id, field_index, state));
            }
            (
                vec![spans],
                vec![LineMeta::form_checkbox(form_id, field_index)],
            )
        }
        FormField::Radio { options, .. } => {
            let selected_idx = match state.fields.get(&key) {
                Some(FieldValue::Index(i)) => *i,
                _ => options.iter().position(|o| o.checked).unwrap_or(0),
            };
            let mut lines = Vec::new();
            let mut metas = Vec::new();
            for (i, opt) in options.iter().enumerate() {
                let mark = if i == selected_idx { "(x)" } else { "( )" };
                lines.push(vec![Span::raw(format!("{} {}", mark, opt.label))]);
                metas.push(LineMeta::form_radio_option(form_id, field_index, i));
            }
            (lines, metas)
        }
        FormField::Select { options, .. } => {
            let selected_idx = match state.fields.get(&key) {
                Some(FieldValue::Index(i)) => *i,
                _ => options.iter().position(|o| o.selected).unwrap_or(0),
            };
            let selected = options
                .get(selected_idx)
                .map(|o| o.label.clone())
                .unwrap_or_default();

            let is_open = state.select_open.contains(&key);

            if is_open {
                let mut lines =
                    vec![vec![Span::raw(format!("[ {} ▲ ]", selected))]];
                let mut metas = vec![LineMeta::form_select_header(form_id, field_index)];
                for (i, opt) in options.iter().enumerate() {
                    let marker = if i == selected_idx { ">" } else { " " };
                    lines.push(vec![Span::raw(format!(
                        "  {} {}",
                        marker, opt.label
                    ))]);
                    metas.push(LineMeta::form_select_option(form_id, field_index, i));
                }
                (lines, metas)
            } else {
                (
                    vec![vec![Span::raw(format!("[ {} ▼ ]", selected))]],
                    vec![LineMeta::form_select_collapsed(form_id, field_index)],
                )
            }
        }
        FormField::Submit { label } => {
            let mut lines = vec![vec![Span::raw(format!("[ {} ]", label))]];
            let mut metas = vec![LineMeta::form_submit(form_id)];
            if let Some(err) = state.form_errors.get(&form_id) {
                lines.push(vec![Span {
                    text: err.clone(),
                    style: SpanStyle {
                        fg: Some(Color::Red),
                        ..Default::default()
                    },
                    link: None,
                }]);
                metas.push(LineMeta::normal());
            }
            (lines, metas)
        }
        FormField::Reset { label } => (
            vec![vec![Span::raw(format!("[ {} ]", label))]],
            vec![LineMeta::form_reset(form_id)],
        ),
        FormField::Hidden { .. } => (vec![], vec![]),
    }
}

fn spans_to_ratatui(spans: &[Span]) -> Vec<RatSpan<'static>> {
    spans
        .iter()
        .map(|s| RatSpan::styled(sanitize(&s.text), span_style(s)))
        .collect()
}

fn span_style(s: &Span) -> Style {
    let mut st = Style::default();
    if s.style.bold {
        st = st.add_modifier(Modifier::BOLD);
    }
    if s.style.italic {
        st = st.add_modifier(Modifier::ITALIC);
    }
    if s.style.strikethrough {
        st = st.add_modifier(Modifier::CROSSED_OUT);
    }
    if s.style.superscript {
        st = st.add_modifier(Modifier::BOLD);
    }
    if s.style.subscript {
        st = st.add_modifier(Modifier::DIM);
    }
    if s.style.reversed {
        st = st.add_modifier(Modifier::REVERSED);
    }
    if s.style.code {
        st = st.fg(Color::Yellow);
    }
    if let Some(fg) = s.style.fg {
        st = st.fg(fg);
    }
    if let Some(bg) = s.style.bg {
        st = st.bg(bg);
    }
    if s.link.is_some() && s.style.fg.is_none() {
        st = st.fg(Color::Blue).add_modifier(Modifier::UNDERLINED);
    }
    st
}