use crate::core::{Settings, settings::def::{NodeDef, Value}};

pub mod appearance;
pub mod general;
pub mod network;

pub const CATEGORIES: &[NodeDef] = &[
    general::NODE_GENERAL,
    appearance::NODE_APPEARANCE,
    network::NODE_NETWORK,
];

pub fn when_x_value(settings: &Settings, key: String, value: &Value) -> bool {
    match settings.get(&key) {
        Some(val) => value == val,
        _ => false,
    }
}