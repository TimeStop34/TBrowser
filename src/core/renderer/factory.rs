use crate::core::renderer::base::Renderer;
use crate::core::renderer::plain::PlainRenderer;
use crate::core::renderer::tmkup::TmkupRenderer;
use crate::core::settings::Settings;

pub fn get_renderer(content_type: &str, settings: &Settings) -> Box<dyn Renderer> {
    let ct = content_type.to_ascii_lowercase();
    if ct.contains("tmkup") {
        Box::new(TmkupRenderer::new(settings))
    } else {
        Box::new(PlainRenderer)
    }
}