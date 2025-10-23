// tests/support/mocks/util.rs
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct DummyClock;

impl mokkan_core::application::ports::time::Clock for DummyClock {
    fn now(&self) -> DateTime<Utc> {
        // Use fixed time for deterministic tests
        crate::time::fixed_now()
    }
}

#[derive(Clone)]
pub struct DummySlug;

impl mokkan_core::application::ports::util::SlugGenerator for DummySlug {
    fn slugify(&self, s: &str) -> String {
        s.to_string()
    }
}
