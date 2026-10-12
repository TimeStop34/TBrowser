use ratatui::{
    style::Color,
    text::{Line, Text},
};

use crate::core::renderer::code::base::{CodeRenderResult, CodeRenderer};

pub struct FallbackCodeRenderer;

impl CodeRenderer for FallbackCodeRenderer {
    fn render(&self, lines: &[String], language: &str) -> CodeRenderResult {
        let label = if language.is_empty() {
            "code".to_string()
        } else {
            language.to_string()
        };

        let rendered: Vec<Line<'static>> = lines
            .iter()
            .map(|l| Line::from(l.clone()))
            .collect();

        CodeRenderResult {
            text: Text::from(rendered),
            label,
            fg: Color::Gray,
            bg: Color::Black,
        }
    }
}