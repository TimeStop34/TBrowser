use crate::core::settings::{DefaultValue, def::{
    ItemDef, NodeDef, ParamDef, ParamKind, always_active,
}};

pub static COLOR_TYPES: &'static[&'static str] = &["White", "Accent", "Custom"];

static P_BORDERS_FG: ParamDef = ParamDef {
    key: "theme.borders_color_type",
    label: "Border color type",
    description: "Color type for borders",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_SELECTED_BTN_BG: ParamDef = ParamDef {
    key: "theme.selected_btn_bg_type",
    label: "Selected buttons background color type",
    description: "Color type for the background color of the selected buttons",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_HEADER_BG: ParamDef = ParamDef {
    key: "theme.header_bg_type",
    label: "Header background color type",
    description: "Color type for the background color of the header",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_URL_BAR_FG: ParamDef = ParamDef {
    key: "theme.url_bar_color_type",
    label: "URL bar color type",
    description: "Color type for the loading progress bar",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(3)
};

static P_SCROLLBAR_FG: ParamDef = ParamDef {
    key: "theme.scrollbar_color_type",
    label: "Scrollbar color type",
    description: "Color type for the scrollbar.",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(3),
};

static P_SELECTION_BG: ParamDef = ParamDef {
    key: "theme.selection_bg_type",
    label: "Selection background color type",
    description: "Color type for the background color of the selection",
    kind: ParamKind::Enum(&COLOR_TYPES),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_KEYBIND_HINT_BG_TYPE: ParamDef = ParamDef {
    key: "theme.keybind_hint_bg_type",
    label: "Keybind hint bg type",
    description: "Background color type for the keybind hint overlay",
    kind: ParamKind::Enum(&["White", "Accent", "Custom"]),
    active: always_active,
    default: DefaultValue::EnumIndex(0),
};

static P_KEYBIND_HINT_FG_TYPE: ParamDef = ParamDef {
    key: "theme.keybind_hint_fg_type",
    label: "Keybind hint fg type",
    description: "Foreground color type for the keybind hint overlay",
    kind: ParamKind::Enum(&["White", "Accent", "Custom"]),
    active: always_active,
    default: DefaultValue::EnumIndex(0),
};

const ACCENTS_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_BORDERS_FG),
    ItemDef::Param(&P_SELECTED_BTN_BG),
    ItemDef::Param(&P_HEADER_BG),
    ItemDef::Param(&P_URL_BAR_FG),
    ItemDef::Param(&P_SCROLLBAR_FG),
    ItemDef::Param(&P_SELECTION_BG),
    ItemDef::Param(&P_KEYBIND_HINT_FG_TYPE),
    ItemDef::Param(&P_KEYBIND_HINT_BG_TYPE),
];

pub static NODE_INTERFACE: NodeDef = NodeDef {
    key: "appearance.interface",
    label: "Interface colors",
    active: always_active,
    children: ACCENTS_CHILDREN,
};