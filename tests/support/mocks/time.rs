// tests/support/mocks/time.rs
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;

/// テスト用の固定タイムスタンプ
static FIXED_NOW: Lazy<DateTime<Utc>> = Lazy::new(|| {
    DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .expect("invalid RFC3339 in tests/support/mocks/time.rs")
        .with_timezone(&Utc)
});

/// 決定論的なタイムスタンプを返す
pub fn fixed_now() -> DateTime<Utc> {
    FIXED_NOW.clone()
}
