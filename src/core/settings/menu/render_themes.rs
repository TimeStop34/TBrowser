use crate::core::renderer::tmkup_theme::NODE_TMKUP_THEME;
use crate::core::settings::def::{always_active, ItemDef, NodeDef};

const RENDER_THEMES_CHILDREN: &[ItemDef] = &[ItemDef::Group(&NODE_TMKUP_THEME)];

pub static NODE_RENDER_THEMES: NodeDef = NodeDef {
    key: "render_themes",
    label: "Render Themes",
    active: always_active,
    children: RENDER_THEMES_CHILDREN,
};