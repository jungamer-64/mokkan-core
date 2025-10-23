// tests/support/mocks/mod.rs
//! test support mocks re-export module
#![cfg(any(test, feature = "test-utils"))]
#![allow(dead_code)]

pub mod time;
pub mod security;
pub mod audit;
pub mod repos;
pub mod util;

// Re-export common items for compatibility with existing imports that used
// `tests::support::mocks::*` or referenced symbols directly from the file.
pub use time::fixed_now;

pub use security::{DummyTokenManager, TEST_TOKEN, NO_AUDIT_TOKEN, EXPIRED_TOKEN, DummyPasswordHasher, StrictPasswordHasher};

pub use audit::{sample_audit, sample_audit_with};

pub use repos::{MockRepo, MockAuditRepo, CapturingAuditRepo};

pub use util::{DummyClock, DummySlug};
