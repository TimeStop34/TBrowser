mod v1;

use crate::core::settings::{Settings, CONFIG_VERSION};

pub type Migrator = fn(from: u32, settings: &mut Settings) -> bool;

fn current_migrator() -> Option<Migrator> {
    match CONFIG_VERSION {
        1 => Some(v1::migrate),
        _ => None,
    }
}

pub fn migrate(from: u32, settings: &mut Settings) -> bool {
    if from == CONFIG_VERSION {
        return true;
    }
    if from == 0 || from > CONFIG_VERSION {
        return false;
    }
    let Some(m) = current_migrator() else {
        return false;
    };
    m(from, settings)
}