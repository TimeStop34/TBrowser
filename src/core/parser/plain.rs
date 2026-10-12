use async_trait::async_trait;

use crate::core::document::Document;
use crate::core::parser::base::Parser;

pub struct PlainParser;

#[async_trait]
impl Parser for PlainParser {
    fn content_type(&self) -> &str {
        "text/plain"
    }

    async fn parse(&self, raw: &str, content_type: &str) -> Document {
        let mut doc = Document::new(content_type);
        for line in raw.lines() {
            doc.add_paragraph(line.to_string());
        }
        doc
    }
}