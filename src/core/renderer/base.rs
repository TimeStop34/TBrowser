use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use indextree::NodeId;

use crate::core::document::Document;
use crate::core::renderer::meta::RenderedContent;
use crate::core::renderer::theme::RendererTheme;

#[async_trait]
pub trait Renderer: Send + Sync {
    #[allow(unused)]
    fn content_type(&self) -> &str;
    async fn render(
        &self,
        doc: &Document,
        base_url: &str,
        theme: &dyn RendererTheme,
        viewport_width: u16,
        state: &HashMap<NodeId, bool>,
    ) -> Result<RenderedContent>;
}