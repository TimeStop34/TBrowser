use async_trait::async_trait;

use crate::core::document::Document;

#[async_trait]
pub trait Parser: Send + Sync {
    #[allow(unused)]
    fn content_type(&self) -> &str;
    async fn parse(&self, raw: &str, content_type: &str) -> Document;
}