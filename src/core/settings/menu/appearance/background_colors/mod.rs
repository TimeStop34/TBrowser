use crate::{core::{ParamDef, ParamKind, Value, settings::{DefaultValue, appearance::COLORS, def::{
    ItemDef, NodeDef,
}, when_x_value}}};

static P_SELECTION_BG_COLOR: ParamDef = ParamDef {
    key: "selection_bg_color",
    label: "Selection background color",
    description: "Background color of the selection",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "appearance.interface.selection_bg_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

static P_HEADER_BG_COLOR: ParamDef = ParamDef {
    key: "header_bg_color",
    label: "Header background color",
    description: "Background color of the header",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "appearance.interface.header_bg_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

static P_SELECTED_BTN_BG_COLOR: ParamDef = ParamDef {
    key: "selected_btn_bg_color",
    label: "Selected buttons background color",
    description: "Background color of the selected buttons",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "appearance.interface.selected_btn_bg_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

static P_KEYBIND_HINT_BG: ParamDef = ParamDef {
    key: "keybind_hint_bg",
    label: "Keybind hint bg",
    description: "Background color for the keybind hint overlay (when type is Custom).",
    kind: ParamKind::Color(COLORS),
    active: |s| when_x_value(s, "appearance.interface.keybind_hint_bg_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0),
};

pub static NODE_BGCOLORS: NodeDef = NodeDef {
    key: "appearance.bg_colors",
    label: "Background colors",
    active: |s| {
        when_x_value(s, "appearance.interface.selection_bg_type".to_string(), &Value::EnumIndex(2)) ||
        when_x_value(s, "appearance.interface.header_bg_type".to_string(), &Value::EnumIndex(2)) ||
        when_x_value(s, "appearance.interface.selected_btn_bg_type".to_string(), &Value::EnumIndex(2)) ||
        when_x_value(s, "appearance.interface.keybind_hint_bg_type".to_string(), &Value::EnumIndex(2))
    },
    children: &[
        ItemDef::Param(&P_SELECTED_BTN_BG_COLOR),
        ItemDef::Param(&P_HEADER_BG_COLOR),
        ItemDef::Param(&P_SELECTION_BG_COLOR),
        ItemDef::Param(&P_KEYBIND_HINT_BG)
    ],
};