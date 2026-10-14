use crate::core::settings::{DefaultValue, def::{
    ItemDef, NodeDef, ParamDef, ParamKind, always_active,
}, general::keybind::NODE_KEYBIND};

static P_HOMEPAGE: ParamDef = ParamDef {
    key: "homepage",
    label: "Homepage",
    description: "The URL loaded at the start of the application.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str("tmutp://localhost/")
};

static P_MAX_TABS: ParamDef = ParamDef {
    key: "max_tabs",
    label: "Max tabs",
    description: "The maximum number of simultaneously open tabs.",
    kind: ParamKind::Int,
    active: always_active,
    default: DefaultValue::Int(20)
};

static P_AUTOLOAD_IMAGES: ParamDef = ParamDef {
    key: "autoload_images",
    label: "Autoload images",
    description: "Automatically load images when the page is opened",
    kind: ParamKind::Bool,
    active: always_active,
    default: DefaultValue::Bool(true)
};

mod keybind;

const GENERAL_CHILDREN: &[ItemDef] = &[
    ItemDef::Group(&NODE_KEYBIND),

    ItemDef::Param(&P_HOMEPAGE),
    ItemDef::Param(&P_MAX_TABS),
    ItemDef::Param(&P_AUTOLOAD_IMAGES),
];

pub static NODE_GENERAL: NodeDef = NodeDef {
    key: "general",
    label: "General",
    active: always_active,
    children: GENERAL_CHILDREN,
};