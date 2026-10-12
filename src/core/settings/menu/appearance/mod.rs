use crate::core::settings::DefaultValue;
use crate::core::{ParamDef, ParamKind};
use crate::core::settings::def::{always_active, ItemDef, NodeDef};

pub mod interface;
pub mod layout;
pub mod background_colors;
pub mod foreground_colors;

use self::interface::NODE_INTERFACE;
use self::layout::NODE_LAYOUT;
use self::background_colors::NODE_BGCOLORS;
use self::foreground_colors::NODE_FGCOLORS;

pub const COLORS: &[&'static str] = &[
    "Blue",
    "Green",
    "Magenta",
    "Cyan",
    "Yellow",
    "Red",
];

pub static P_ACCENT: ParamDef = ParamDef {
    key: "theme.accent",
    label: "Accent color",
    description: "Accent color of the interface.",
    kind: ParamKind::Color(COLORS),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_SELECTED_TAB_COLOR: ParamDef = ParamDef {
    key: "theme.selected_tab_color",
    label: "Selected tab color",
    description: "The color of the selected tab in the tab menu.",
    kind: ParamKind::Color(COLORS),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

const APPEARANCE_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_ACCENT),
    ItemDef::Param(&P_SELECTED_TAB_COLOR),

    ItemDef::Group(&NODE_LAYOUT),
    ItemDef::Group(&NODE_INTERFACE),
    ItemDef::Group(&NODE_FGCOLORS),
    ItemDef::Group(&NODE_BGCOLORS),
];

pub static NODE_APPEARANCE: NodeDef = NodeDef {
    key: "appearance",
    label: "Appearance",
    active: always_active,
    children: APPEARANCE_CHILDREN,
};