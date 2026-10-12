use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "value")]
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    EnumIndex(usize),
}

#[derive(Debug, Clone, Copy)]
pub enum ParamKind {
    Bool,
    Int,
    Float,
    String,
    Keybind,
    Enum(&'static [&'static str]),
    Color(&'static [&'static str]),
}

pub type ActiveFn = fn(&Settings) -> bool;

pub fn always_active(_: &Settings) -> bool {
    true
}

#[derive(Debug, Clone, Copy)]
pub enum DefaultValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(&'static str),
    EnumIndex(usize),
}

impl DefaultValue {
    pub fn to_value(&self) -> Value {
        match self {
            DefaultValue::Bool(b) => Value::Bool(*b),
            DefaultValue::Int(i) => Value::Int(*i),
            DefaultValue::Float(f) => Value::Float(*f),
            DefaultValue::Str(s) => Value::Str((*s).to_string()),
            DefaultValue::EnumIndex(i) => Value::EnumIndex(*i),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParamDef {
    pub key: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub kind: ParamKind,
    pub active: ActiveFn,
    pub default: DefaultValue,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeDef {
    pub key: &'static str,
    pub label: &'static str,
    pub active: ActiveFn,
    pub children: &'static [ItemDef],
}

#[derive(Debug, Clone, Copy)]
pub enum ItemDef {
    Param(&'static ParamDef),
    Group(&'static NodeDef),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub values: HashMap<String, Value>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            values: collect_defaults(),
        }
    }

    pub fn config_path() -> Option<PathBuf> {
        let home = std::env::var("HOME").ok()?;
        Some(PathBuf::from(home).join(".config/tbrowser/settings.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::new();
        };
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(s) => toml::from_str::<Settings>(&s).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        };
        for (k, v) in collect_defaults() {
            settings.values.entry(k).or_insert(v);
        }
        settings
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::config_path() else {
            anyhow::bail!("no config path");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = toml::to_string_pretty(self)?;
        std::fs::write(&path, s)?;
        Ok(())
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        self.values.get_mut(key)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

pub fn collect_defaults() -> HashMap<String, Value> {
    let mut out = HashMap::new();
    for node in crate::core::settings::menu::CATEGORIES {
        collect_node(node, &mut out);
    }
    out
}

fn collect_node(node: &NodeDef, out: &mut HashMap<String, Value>) {
    for child in node.children {
        match child {
            ItemDef::Param(p) => {
                out.insert(p.key.to_string(), p.default.to_value());
            }
            ItemDef::Group(g) => collect_node(g, out),
        }
    }
}