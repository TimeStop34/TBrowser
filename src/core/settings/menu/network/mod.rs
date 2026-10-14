use crate::core::settings::{DefaultValue, def::{
    ItemDef, NodeDef, ParamDef, ParamKind, always_active,
}};

static P_TIMEOUT: ParamDef = ParamDef {
    key: "timeout",
    label: "Timeout",
    description: "Timeout of network request in seconds",
    kind: ParamKind::Int,
    active: always_active,
    default: DefaultValue::Int(30)
};

static P_FOLLOW_REDIRECTS: ParamDef = ParamDef {
    key: "follow_redirects",
    label: "Follow redirects",
    description: "Automatically follow tmutp redirects",
    kind: ParamKind::Bool,
    active: always_active,
    default: DefaultValue::Bool(true)
};

const NETWORK_CHILDREN: &[ItemDef] = &[
    ItemDef::Param(&P_TIMEOUT),
    ItemDef::Param(&P_FOLLOW_REDIRECTS),
];

pub static NODE_NETWORK: NodeDef = NodeDef {
    key: "network",
    label: "Network",
    active: always_active,
    children: NETWORK_CHILDREN,
};