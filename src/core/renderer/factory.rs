use crate::core::renderer::base::Renderer;
use crate::core::renderer::plain::PlainRenderer;
use crate::core::renderer::tmkup::TmkupRenderer;

pub fn get_renderer(content_type: &str) -> Box<dyn Renderer> {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("tmkup") {
        Box::new(TmkupRenderer)
    } else {
        Box::new(PlainRenderer)
    }
}