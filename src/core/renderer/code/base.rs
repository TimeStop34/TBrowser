use ratatui::{
    style::Color,
    text::Text,
};

pub struct CodeRenderResult {
    pub text: Text<'static>,
    pub label: String,
    pub fg: Color,
    pub bg: Color,
}

pub trait CodeRenderer: Send + Sync {
    fn render(&self, lines: &[String], language: &str) -> CodeRenderResult;
}