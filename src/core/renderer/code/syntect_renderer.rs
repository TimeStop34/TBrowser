use std::sync::OnceLock;

use ratatui::{
    style::{Color, Style},
    text::{Line, Span as RatSpan, Text},
};
use syntect::parsing::{
    ParseState, ScopeStack, SyntaxDefinition, SyntaxReference, SyntaxSet, SyntaxSetBuilder,
};

use crate::core::renderer::code::base::{CodeRenderResult, CodeRenderer};
use crate::core::renderer::code::theme::{canonical_lang, CodeTheme};

const RHAI_SYNTAX: &str = include_str!("syntaxes/rhai.sublime-syntax");
const TMKUP_SYNTAX: &str = include_str!("syntaxes/tmkup.sublime-syntax");

static SYNTAX_SET: OnceLock<SyntaxSet> = OnceLock::new();
static CUSTOM_SET: OnceLock<SyntaxSet> = OnceLock::new();

fn syntax_set() -> &'static SyntaxSet {
    SYNTAX_SET.get_or_init(two_face::syntax::extra_newlines)
}

fn custom_set() -> &'static SyntaxSet {
    CUSTOM_SET.get_or_init(|| {
        let mut builder = SyntaxSetBuilder::new();
        for (name, src) in [("Rhai", RHAI_SYNTAX), ("TMkUp", TMKUP_SYNTAX)] {
            match SyntaxDefinition::load_from_str(src, true, None) {
                Ok(def) => builder.add(def),
                Err(e) => eprintln!("Failed to load {} syntax: {}", name, e),
            }
        }
        builder.build()
    })
}

/// Ищет грамматику. Порядок: кастомные → two-face.
/// Возвращает `None`, если грамматики нет → fallback.
pub fn find_syntax(lang: &str) -> Option<(&'static SyntaxSet, &'static SyntaxReference)> {
    if lang.is_empty() {
        return None;
    }
    let canonical = canonical_lang(lang);
    let raw = lang.to_ascii_lowercase();

    let custom = custom_set();
    if !canonical.is_empty() {
        if let Some(s) = lookup(custom, canonical) {
            return Some((custom, s));
        }
    }
    if let Some(s) = lookup(custom, &raw) {
        return Some((custom, s));
    }

    let main = syntax_set();
    if !canonical.is_empty() {
        if let Some(s) = lookup(main, canonical) {
            return Some((main, s));
        }
    }
    lookup(main, &raw).map(|s| (main, s))
}

fn lookup(ss: &'static SyntaxSet, key: &str) -> Option<&'static SyntaxReference> {
    ss.find_syntax_by_token(key)
        .or_else(|| ss.find_syntax_by_extension(key))
        .or_else(|| {
            ss.syntaxes()
                .iter()
                .find(|s| s.name.eq_ignore_ascii_case(key))
        })
}

pub struct SyntectCodeRenderer {
    theme: CodeTheme,
}

impl SyntectCodeRenderer {
    pub fn new(theme: CodeTheme) -> Self {
        Self { theme }
    }
}

impl CodeRenderer for SyntectCodeRenderer {
    fn render(&self, lines: &[String], language: &str) -> CodeRenderResult {
        let label = if language.is_empty() {
            "code".to_string()
        } else {
            language.to_string()
        };

        let Some((ss, syntax)) = find_syntax(language) else {
            return fallback_result(lines, label);
        };

        let mut state = ParseState::new(syntax);
        let mut stack = ScopeStack::new();

        let mut rendered: Vec<Line<'static>> = Vec::with_capacity(lines.len());

        for line in lines {
            let with_nl = format!("{}\n", line);
            let ops = state.parse_line(&with_nl, ss).unwrap_or_default();
            let line_len = line.len();

            let mut spans: Vec<RatSpan<'static>> = Vec::new();
            let mut accum = String::new();
            let mut accum_style: Option<Style> = None;
            let mut prev = 0usize;

            for (off, op) in ops {
                let clamped = off.min(line_len);
                if clamped > prev {
                    let cat = scope_to_category(&stack);
                    let style = self.style_for(cat);
                    push_segment(
                        &mut spans,
                        &mut accum,
                        &mut accum_style,
                        &line[prev..clamped],
                        style,
                    );
                }
                let _ = stack.apply(&op);
                prev = clamped;
            }

            if prev < line_len {
                let cat = scope_to_category(&stack);
                let style = self.style_for(cat);
                push_segment(
                    &mut spans,
                    &mut accum,
                    &mut accum_style,
                    &line[prev..],
                    style,
                );
            }

            if let (Some(s), false) = (accum_style, accum.is_empty()) {
                spans.push(RatSpan::styled(accum, s));
            }

            rendered.push(Line::from(spans));
        }

        CodeRenderResult {
            text: Text::from(rendered),
            label,
            fg: Color::Gray,
            bg: Color::Black,
        }
    }
}

impl SyntectCodeRenderer {
    fn style_for(&self, cat: Category) -> Style {
        let fg = match cat {
            Category::Keyword => self.theme.keyword,
            Category::String => self.theme.string,
            Category::Comment => self.theme.comment,
            Category::Number => self.theme.number,
            Category::Function => self.theme.function,
            Category::Type => self.theme.type_,
            Category::Constant => self.theme.constant,
            Category::Operator => self.theme.operator,
            Category::Punctuation => self.theme.punctuation,
            Category::Plain => self.theme.default_fg,
        };
        Style::default().fg(fg)
    }
}

fn push_segment(
    spans: &mut Vec<RatSpan<'static>>,
    accum: &mut String,
    accum_style: &mut Option<Style>,
    text: &str,
    style: Style,
) {
    if text.is_empty() {
        return;
    }
    match *accum_style {
        Some(s) if s == style => {
            accum.push_str(text);
        }
        Some(s) => {
            if !accum.is_empty() {
                spans.push(RatSpan::styled(std::mem::take(accum), s));
            }
            accum.push_str(text);
            *accum_style = Some(style);
        }
        None => {
            accum.push_str(text);
            *accum_style = Some(style);
        }
    }
}

fn fallback_result(lines: &[String], label: String) -> CodeRenderResult {
    let rendered: Vec<Line<'static>> = lines.iter().map(|l| Line::from(l.clone())).collect();
    CodeRenderResult {
        text: Text::from(rendered),
        label,
        fg: Color::Gray,
        bg: Color::Black,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Constant,
    Operator,
    Punctuation,
    Plain,
}

fn scope_to_category(stack: &ScopeStack) -> Category {
    for scope in stack.as_slice().iter().rev() {
        let s = scope.build_string();
        match classify(&s) {
            Category::Plain => continue,
            other => return other,
        }
    }
    Category::Plain
}

fn classify(scope: &str) -> Category {
    // --- TMkUp ---
    if scope.starts_with("punctuation.definition.metadata") {
        return Category::Keyword;
    }
    if scope.starts_with("entity.name.tag.metadata") {
        return Category::Type;
    }
    if scope.starts_with("punctuation.definition.directive") {
        return Category::Keyword;
    }
    if scope.starts_with("keyword.control.gopher")
        || scope.starts_with("keyword.control.radio")
        || scope.starts_with("keyword.control.select")
    {
        return Category::Keyword;
    }
    if scope.starts_with("punctuation.definition.block") {
        return Category::Keyword;
    }
    if scope.starts_with("punctuation.definition.form") {
        return Category::Keyword;
    }
    if scope.starts_with("punctuation.definition.field") {
        return Category::Keyword;
    }
    if scope.starts_with("entity.name.label.field") {
        return Category::Type;
    }
    if scope.starts_with("entity.other.attribute-name") {
        return Category::Type;
    }
    if scope.starts_with("keyword.other.flag") {
        return Category::Constant;
    }
    if scope.starts_with("punctuation.definition.heading") {
        return Category::Keyword;
    }
    if scope.starts_with("entity.name.section") {
        return Category::Function;
    }
    if scope.starts_with("punctuation.definition.quote") {
        return Category::Keyword;
    }
    if scope.starts_with("punctuation.definition.table") {
        return Category::Punctuation;
    }
    if scope.starts_with("punctuation.definition.list") {
        return Category::Keyword;
    }
    if scope.starts_with("punctuation.definition.task") {
        return Category::Constant;
    }
    if scope.starts_with("punctuation.definition.footnote") {
        return Category::Keyword;
    }
    if scope.starts_with("entity.name.footnote") {
        return Category::Constant;
    }
    if scope.starts_with("punctuation.definition.image")
        || scope.starts_with("punctuation.definition.link")
    {
        return Category::Keyword;
    }
    if scope.starts_with("string.other.image.alt")
        || scope.starts_with("string.other.link.title")
        || scope.starts_with("string.other.link.description")
    {
        return Category::String;
    }
    if scope.starts_with("markup.underline.link") {
        return Category::Type;
    }
    if scope.starts_with("punctuation.definition.variable") {
        return Category::Keyword;
    }
    if scope.starts_with("variable.other") {
        return Category::Constant;
    }
    if scope.starts_with("support.constant.color") {
        return Category::Constant;
    }
    if scope.starts_with("punctuation.definition.color") {
        return Category::Keyword;
    }
    if scope.starts_with("markup.bold")
        || scope.starts_with("markup.italic")
        || scope.starts_with("markup.strikethrough")
        || scope.starts_with("markup.highlight")
        || scope.starts_with("markup.superscript")
        || scope.starts_with("markup.subscript")
    {
        return Category::Function;
    }
    if scope.starts_with("markup.heading") {
        return Category::Keyword;
    }
    if scope.starts_with("markup.quote") {
        return Category::Comment;
    }
    if scope.starts_with("markup.list") {
        return Category::Constant;
    }
    if scope.starts_with("markup.table") {
        return Category::Type;
    }
    if scope.starts_with("markup.fenced_code") {
        return Category::String;
    }
    if scope.starts_with("punctuation.definition.code") {
        return Category::Keyword;
    }
    if scope.starts_with("fenced_code.block.language") {
        return Category::Constant;
    }
    if scope.starts_with("punctuation.definition.raw") {
        return Category::Keyword;
    }
    if scope.starts_with("markup.inline.raw") {
        return Category::String;
    }
    if scope.starts_with("meta.separator") {
        return Category::Comment;
    }

    // --- generic ---
    if scope.starts_with("markup.heading") {
        return Category::Keyword;
    }
    if scope.starts_with("markup.bold") {
        return Category::Function;
    }
    if scope.starts_with("markup.italic") {
        return Category::Type;
    }
    if scope.starts_with("markup.raw") {
        return Category::String;
    }
    if scope.starts_with("markup.quote") {
        return Category::Comment;
    }
    if scope.starts_with("markup.list") {
        return Category::Constant;
    }
    if scope.starts_with("markup.underline") {
        return Category::Operator;
    }

    if scope.starts_with("comment") {
        return Category::Comment;
    }
    if scope.starts_with("string") {
        return Category::String;
    }
    if scope.starts_with("constant.numeric") {
        return Category::Number;
    }
    if scope.starts_with("constant.language")
        || scope.starts_with("constant.character.escape")
        || scope.starts_with("constant.other")
    {
        return Category::Constant;
    }
    if scope.starts_with("keyword.operator") {
        return Category::Operator;
    }
    if scope.starts_with("keyword") {
        return Category::Keyword;
    }
    if scope.starts_with("storage.type") {
        return Category::Keyword;
    }
    if scope.starts_with("storage.modifier") {
        return Category::Keyword;
    }
    if scope.starts_with("entity.name.function")
        || scope.starts_with("support.function")
        || scope.starts_with("entity.name.method")
    {
        return Category::Function;
    }
    if scope.starts_with("support.type")
        || scope.starts_with("support.class")
        || scope.starts_with("entity.name.type")
        || scope.starts_with("entity.name.class")
        || scope.starts_with("entity.name.struct")
        || scope.starts_with("entity.name.enum")
        || scope.starts_with("entity.name.trait")
        || scope.starts_with("entity.name.union")
        || scope.starts_with("entity.name.namespace")
    {
        return Category::Type;
    }
    if scope.starts_with("punctuation")
        || scope.starts_with("meta.brace")
        || scope.starts_with("meta.delimiter")
    {
        return Category::Punctuation;
    }
    if scope.starts_with("invalid") {
        return Category::Constant;
    }
    Category::Plain
}