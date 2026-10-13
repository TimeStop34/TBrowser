use crate::core::{ItemDef, NodeDef, ParamDef, ParamKind, Value, settings::{DefaultValue, appearance::COLORS, when_x_value}};

static P_BORDERS_FG_COLOR: ParamDef = ParamDef {
    key: "theme.borders_color",
    label: "Borders background color",
    description: "Background color of the borders",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "theme.borders_color_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

static P_URL_BAR_FG_COLOR: ParamDef = ParamDef {
    key: "theme.url_bar_color",
    label: "URL bar color",
    description: "Color for the loading progress bar",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "theme.url_bar_color_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

static P_SCROLLBAR_FG_COLOR: ParamDef = ParamDef {
    key: "theme.scrollbar_color",
    label: "Scrollbar color",
    description: "Color for the scrollbar",
    kind: ParamKind::Enum(COLORS),
    active: |s| when_x_value(s, "theme.scrollbar_color_type".to_string(), &Value::EnumIndex(2)),
    default: DefaultValue::EnumIndex(0)
};

pub static NODE_FGCOLORS: NodeDef = NodeDef {
    key: "appearance.fg_colors",
    label: "Foreground colors",
    active: |s| {
        when_x_value(s, "theme.borders_color_type".to_string(), &Value::EnumIndex(2)) ||
        when_x_value(s, "theme.url_bar_color_type".to_string(), &Value::EnumIndex(2)) ||
        when_x_value(s, "theme.scrollbar_color_type".to_string(), &Value::EnumIndex(2))
    },
    children: &[
        ItemDef::Param(&P_BORDERS_FG_COLOR),
        ItemDef::Param(&P_URL_BAR_FG_COLOR),
        ItemDef::Param(&P_SCROLLBAR_FG_COLOR),
    ],
};