use crate::core::document::{Document, InfoKind, NodeData, Span};

pub fn build_error_page(url: &str, error: &str) -> Document {
    let mut doc = Document::new("text/tmkup");
    doc.title = "Error".to_string();

    let info = doc.add_node(NodeData::InfoBlock {
        kind: InfoKind::Error,
    });

    doc.add_child(
        info,
        NodeData::Paragraph(vec![Span::raw("Failed to load page")]),
    );
    doc.add_child(info, NodeData::Paragraph(vec![]));

    doc.add_child(info, NodeData::Paragraph(vec![Span::raw("URL:")]));
    doc.add_child(
        info,
        NodeData::Paragraph(vec![Span::raw(url.to_string())]),
    );
    doc.add_child(info, NodeData::Paragraph(vec![]));

    doc.add_child(info, NodeData::Paragraph(vec![Span::raw("Error:")]));
    doc.add_child(
        info,
        NodeData::Paragraph(vec![Span::raw(error.to_string())]),
    );
    doc.add_child(info, NodeData::Paragraph(vec![]));

    doc.add_child(info, NodeData::Paragraph(vec![Span::raw("Actions:")]));
    doc.add_child(
        info,
        NodeData::Paragraph(vec![Span::raw(
            "r — reload, o — open another URL, b — back",
        )]),
    );

    doc
}