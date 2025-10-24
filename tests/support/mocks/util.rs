// tests/support/mocks/util.rs
use chrono::{DateTime, Utc};

/* -------------------------------- Clock -------------------------------- */

/// 決定論的なクロック実装（テスト用）
#[derive(Clone, Debug, Default)]
pub struct DummyClock;

impl mokkan_core::application::ports::time::Clock for DummyClock {
    fn now(&self) -> DateTime<Utc> {
        super::time::fixed_now()
    }
}

/* -------------------------------- SlugGenerator -------------------------------- */

/// ダミーのスラグジェネレーター（入力をそのまま返す）
#[derive(Clone, Debug, Default)]
pub struct DummySlug;

impl mokkan_core::application::ports::util::SlugGenerator for DummySlug {
    fn slugify(&self, s: &str) -> String {
        s.to_string()
    }
}