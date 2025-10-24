// tests/support/mocks/mod.rs
//! テストサポートモック再エクスポートモジュール
#![cfg(test)]
#![allow(dead_code)]
#![allow(unused_imports)]

pub mod time;
pub mod security;
pub mod audit;
pub mod repos;
pub mod util;
pub mod user_repo;
pub mod article_repos;

/* -------------------------------- 後方互換性のための再エクスポート -------------------------------- */

// 時刻関連
pub use time::fixed_now;

// セキュリティ関連
pub use security::{
    DummyPasswordHasher, DummyTokenManager, StrictPasswordHasher,
    EXPIRED_TOKEN, NO_AUDIT_TOKEN, TEST_TOKEN, SESSION_TOKEN,
};

// 監査ログ関連
pub use audit::{sample_audit, sample_audit_with};

// リポジトリ関連（監査ログ）
pub use repos::{CapturingAuditRepo, MockAuditRepo, MockRepo};

// ユーティリティ関連
pub use util::{DummyClock, DummySlug};

// ユーザーリポジトリ
pub use user_repo::DummyUserRepo;

// 記事リポジトリ
pub use article_repos::{DummyArticleRead, DummyArticleRevision, DummyArticleWrite};
