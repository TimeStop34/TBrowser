use crate::core::settings::Settings;

/// Миграция 0 → 1.
///
/// v0 — файл без `[metadata]`. Ключи уже в текущей схеме, добавить ничего не нужно.
pub fn migrate(_settings: &mut Settings) -> bool {
    true
}