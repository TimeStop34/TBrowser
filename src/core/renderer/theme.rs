pub trait RendererTheme: Send + Sync {}

pub struct NoTheme;

impl RendererTheme for NoTheme {}