mod base;
pub use base::{CodeRenderResult, CodeRenderer};

mod fallback;
#[allow(unused)]
pub use fallback::FallbackCodeRenderer;

pub mod theme;

mod syntect_renderer;

mod factory;
pub use factory::get_code_renderer;