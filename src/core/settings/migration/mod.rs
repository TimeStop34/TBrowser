mod v1;
// mod v2;  ← сюда добавляешь по мере появления новой версии

use crate::core::settings::{Settings, CONFIG_VERSION};

type Migrator = fn(&mut Settings) -> bool;

/// Таблица миграторов. Ключ — версия, в которую переводит мигратор.
/// `v1::migrate` переводит 0 → 1, `v2::migrate` переводит 1 → 2 и т.д.
const MIGRATORS: &[(u32, Migrator)] = &[
    (1, v1::migrate),
    // (2, v2::migrate),
];

/// Прогоняет цепочку миграций с `from` до текущей `CONFIG_VERSION`.
/// Возвращает `true`, если дошли до конца (или миграция не нужна).
pub fn migrate(from: u32, settings: &mut Settings) -> bool {
    if from == CONFIG_VERSION {
        return true;
    }
    if from > CONFIG_VERSION {
        return false;
    }

    let mut v = from;
    while v < CONFIG_VERSION {
        let target = v + 1;
        let Some((_, m)) = MIGRATORS.iter().find(|(ver, _)| *ver == target) else {
            return false;
        };
        if !m(settings) {
            return false;
        }
        v = target;
    }
    true
}