use ratatui::{
    style::{Modifier, Style},
    text::{Line, Text},
};

use crate::core::document::Document;

pub fn build_info_text(url: &str, doc: Option<&Document>) -> Text<'static> {
    let mut lines = Vec::new();
    lines.push(Line::from(format!("URL: {}", url)));

    if let Some(doc) = doc {
        let scheme = url.split("://").next().unwrap_or("unknown");
        lines.push(Line::from(format!("Protocol: {}", scheme)));
        lines.push(Line::from(format!("Content Type: {}", doc.content_type)));

        let str_meta: Vec<(&String, &String)> = doc.string_meta().collect();
        if !str_meta.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "--- Metadata ---",
                Style::default().add_modifier(Modifier::BOLD),
            ));
            for (k, v) in str_meta {
                lines.push(Line::from(format!("{}: {}", k, v)));
            }
        }

        if let Some(vars) = doc.variables() {
            if !vars.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::styled(
                    "--- Variables ---",
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                for (k, v) in vars {
                    lines.push(Line::from(format!("  {}: {}", k, v)));
                }
            }
        }
    }

    Text::from(lines)
}