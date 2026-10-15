use crate::core::renderer::code::base::CodeRenderer;
use crate::core::renderer::code::fallback::FallbackCodeRenderer;
use crate::core::renderer::code::syntect_renderer::{find_syntax, SyntectCodeRenderer};
use crate::core::renderer::code::theme::CodeTheme;
use crate::core::settings::Settings;

pub fn get_code_renderer(language: &str, settings: &Settings) -> Box<dyn CodeRenderer> {
    if find_syntax(language).is_some() {
        let theme = CodeTheme::from_settings(settings);
        Box::new(SyntectCodeRenderer::new(theme))
    } else {
        Box::new(FallbackCodeRenderer)
    }
}