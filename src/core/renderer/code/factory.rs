use crate::core::renderer::code::base::CodeRenderer;
use crate::core::renderer::code::fallback::FallbackCodeRenderer;

pub fn get_code_renderer(_language: &str) -> Box<dyn CodeRenderer> {
    Box::new(FallbackCodeRenderer)
}