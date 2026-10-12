use crate::core::settings::{DefaultValue, def::{
    ItemDef, NodeDef, ParamDef, ParamKind, always_active,
}};

static P_BORDER_STYLE: ParamDef = ParamDef {
    key: "theme.border_style",
    label: "Border style",
    description: "The interface frame style.",
    kind: ParamKind::Enum(&["Single", "Double", "Rounded", "Thick"]),
    active: always_active,
    default: DefaultValue::EnumIndex(0)
};

static P_BTN_FORMAT: ParamDef = ParamDef {
    key: "theme.btn_format",
    label: "Button format",
    description: "The string used for the button format.\nMust contain {} — expands into the button name.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str("[ {} ]")
};

static P_TITLE_FORMAT: ParamDef = ParamDef {
    key: "theme.title_format",
    label: "Title format",
    description: "The string used for the title format.\nMust contain {} — expands into the title.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str(" {} ")
};

static P_KEYBIND_HINT_FORMAT: ParamDef = ParamDef {
    key: "theme.keybind_hint_format",
    label: "Keybind hint format",
    description: "Format string for the keybind hint. Must contain {} — expands into the action name.",
    kind: ParamKind::String,
    active: always_active,
    default: DefaultValue::Str("< {} >"),
};

static P_KEYBIND_HINT_DURATION: ParamDef = ParamDef {
    key: "theme.keybind_hint_duration",
    label: "Keybind hint duration",
    description: "How long the keybind hint is visible, in seconds.",
    kind: ParamKind::Float,
    active: always_active,
    default: DefaultValue::Float(1.0),
};

static P_URL_BAR_HOLD: ParamDef = ParamDef {
    key: "theme.url_bar_hold",
    label: "Url loading bar hold duration",
    description: "How long the url loading bar is visible, after finishing loading, in seconds.",
    kind: ParamKind::Float,
    active: always_active,
    default: DefaultValue::Float(0.5),
};


const LAYOUT_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_BORDER_STYLE),
    ItemDef::Param(&P_BTN_FORMAT),
    ItemDef::Param(&P_TITLE_FORMAT),
    ItemDef::Param(&P_KEYBIND_HINT_FORMAT),
    ItemDef::Param(&P_KEYBIND_HINT_DURATION),
    ItemDef::Param(&P_URL_BAR_HOLD)
];

pub static NODE_LAYOUT: NodeDef = NodeDef {
    key: "appearance.layout",
    label: "Layout",
    active: always_active,
    children: LAYOUT_CHILDREN,
};