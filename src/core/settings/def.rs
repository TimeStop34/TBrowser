use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

pub const CONFIG_VERSION: u32 = 1;
pub const TBROWSER_VERSION: &str = env!("CARGO_PKG_VERSION");
const HEADER: &str = "# TBrowser settings configuration\n\n";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub version: u32,
    pub tbrowser_version: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            tbrowser_version: TBROWSER_VERSION.to_string(),
        }
    }
}

impl Metadata {
    pub fn legacy() -> Self {
        Self {
            version: 0,
            tbrowser_version: String::new(),
        }
    }
}

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
    #[serde(default = "Metadata::legacy")]
    pub metadata: Metadata,
    pub values: HashMap<String, Value>,
}

impl Settings {
    pub fn new() -> Self {
        Self {
            metadata: Metadata::default(),
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

        let raw = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => {
                let s = Self::new();
                let _ = s.save();
                return s;
            }
        };

        let mut settings: Settings = match toml::from_str(&raw) {
            Ok(s) => s,
            Err(_) => return Self::reset_with_backup(&path, 0),
        };

        let from = settings.metadata.version;

        if from == CONFIG_VERSION {
            settings.metadata.tbrowser_version = TBROWSER_VERSION.to_string();
            Self::fill_missing(&mut settings);
            return settings;
        }

        if crate::core::settings::migration::migrate(from, &mut settings) {
            settings.metadata.version = CONFIG_VERSION;
            settings.metadata.tbrowser_version = TBROWSER_VERSION.to_string();
            Self::fill_missing(&mut settings);
            let _ = settings.save();
            return settings;
        }

        Self::reset_with_backup(&path, from)
    }

    fn fill_missing(settings: &mut Settings) {
        for (k, v) in collect_defaults() {
            settings.values.entry(k).or_insert(v);
        }
    }

    fn reset_with_backup(path: &Path, from: u32) -> Self {
        let bp = path.with_extension(format!("toml.v{}.backup", from));
        let _ = std::fs::copy(path, &bp);

        let s = Self::new();
        let _ = s.save();
        s
    }

    pub fn save(&self) -> Result<()> {
        let Some(path) = Self::config_path() else {
            anyhow::bail!("no config path");
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut out = String::from(HEADER);
        out.push_str(&toml::to_string_pretty(self)?);
        std::fs::write(&path, out)?;
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
                let full = if node.key.is_empty() {
                    p.key.to_string()
                } else {
                    format!("{}.{}", node.key, p.key)
                };
                out.insert(full, p.default.to_value());
            }
            ItemDef::Group(g) => collect_node(g, out),
        }
    }
}