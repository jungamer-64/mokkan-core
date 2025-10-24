// tests/support/mocks/security.rs
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;

/// テスト用トークン定数（タイポ防止とIDE補完のため）
pub const TEST_TOKEN: &str = "test-token";
pub const NO_AUDIT_TOKEN: &str = "no-audit";
pub const EXPIRED_TOKEN: &str = "expired-token";

/* -------------------------------- TokenManager -------------------------------- */

#[derive(Clone, Debug, Default)]
pub struct DummyTokenManager;

#[async_trait]
impl mokkan_core::application::ports::security::TokenManager for DummyTokenManager {
    async fn issue(
        &self,
        _subject: mokkan_core::application::dto::TokenSubject,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto> {
        Err(mokkan_core::application::error::ApplicationError::infrastructure("not implemented"))
    }

    async fn authenticate(
        &self,
        token: &str,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthenticatedUser> {
        let now = super::time::fixed_now();
        match token {
            TEST_TOKEN => Ok(admin_audit_user(now)),
            NO_AUDIT_TOKEN => Ok(author_user(now)),
            // Expired tokens should be rejected at authentication time
            EXPIRED_TOKEN => Err(mokkan_core::application::error::ApplicationError::unauthorized("expired token")),
            _ => Err(mokkan_core::application::error::ApplicationError::unauthorized("invalid token")),
        }
    }
}

fn admin_audit_user(now: DateTime<Utc>) -> mokkan_core::application::dto::AuthenticatedUser {
    mokkan_core::application::dto::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(1).expect("invalid user id"),
        username: "tester".into(),
        role: mokkan_core::domain::user::value_objects::Role::Admin,
        capabilities: HashSet::from([
            mokkan_core::domain::user::value_objects::Capability::new("audit", "read"),
        ]),
        issued_at: now,
        expires_at: now + Duration::hours(1),
    }
}

fn author_user(now: DateTime<Utc>) -> mokkan_core::application::dto::AuthenticatedUser {
    mokkan_core::application::dto::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(2).expect("invalid user id"),
        username: "noaudit".into(),
        role: mokkan_core::domain::user::value_objects::Role::Author,
        capabilities: HashSet::new(),
        issued_at: now,
        expires_at: now + Duration::hours(1),
    }
}

#[allow(dead_code)]
fn expired_admin_audit_user(now: DateTime<Utc>) -> mokkan_core::application::dto::AuthenticatedUser {
    mokkan_core::application::dto::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(3).expect("invalid user id"),
        username: "expired".into(),
        role: mokkan_core::domain::user::value_objects::Role::Admin,
        capabilities: HashSet::from([
            mokkan_core::domain::user::value_objects::Capability::new("audit", "read"),
        ]),
        issued_at: now - Duration::hours(2),
        expires_at: now - Duration::hours(1),
    }
}

/* -------------------------------- PasswordHasher -------------------------------- */

/// 寛容なパスワードハッシャー（大半のテストで使用）
#[derive(Clone, Debug, Default)]
pub struct DummyPasswordHasher;

#[async_trait]
impl mokkan_core::application::ports::security::PasswordHasher for DummyPasswordHasher {
    async fn hash(&self, _password: &str) -> mokkan_core::application::ApplicationResult<String> {
        Ok("hash".into())
    }

    async fn verify(&self, _password: &str, _expected_hash: &str) -> mokkan_core::application::ApplicationResult<()> {
        Ok(())
    }
}

/// 厳密なパスワードハッシャー（ネガティブパステスト用）
#[derive(Clone, Debug, Default)]
pub struct StrictPasswordHasher;

#[async_trait]
impl mokkan_core::application::ports::security::PasswordHasher for StrictPasswordHasher {
    async fn hash(&self, password: &str) -> mokkan_core::application::ApplicationResult<String> {
        Ok(format!("hash::{}", password))
    }

    async fn verify(&self, password: &str, expected_hash: &str) -> mokkan_core::application::ApplicationResult<()> {
        if format!("hash::{}", password) == expected_hash {
            Ok(())
        } else {
            Err(mokkan_core::application::error::ApplicationError::unauthorized("bad password"))
        }
    }
}