use anyhow::Result;
use async_trait::async_trait;
use ratatui::text::{Line, Text};

use crate::core::document::{Document, NodeData};
use crate::core::renderer::base::Renderer;
use crate::core::renderer::theme::RendererTheme;

pub struct PlainRenderer;

#[async_trait]
impl Renderer for PlainRenderer {
    fn content_type(&self) -> &str {
        "text/plain"
    }

    async fn render(
        &self,
        doc: &Document,
        _base_url: &str,
        _theme: &dyn RendererTheme,
        _viewport_width: u16,
    ) -> Result<Text<'static>> {
        let mut lines = Vec::new();
        for (_, node) in doc.iter() {
            if let NodeData::Paragraph(spans) = node {
                let text: String = spans.iter().map(|s| s.text.as_str()).collect();
                lines.push(Line::from(text));
            }
        }
        Ok(Text::from(lines))
    }
}