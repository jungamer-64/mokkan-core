// tests/support/mocks/security.rs
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;

/// テスト用トークン定数（タイポ防止とIDE補完のため）
pub const TEST_TOKEN: &str = "test-token";
pub const NO_AUDIT_TOKEN: &str = "no-audit";
pub const EXPIRED_TOKEN: &str = "expired-token";
pub const SESSION_TOKEN: &str = "session-token";

/* -------------------------------- TokenManager -------------------------------- */

#[derive(Clone, Debug, Default)]
pub struct DummyTokenManager;

#[async_trait]
impl mokkan_core::application::ports::security::TokenManager for DummyTokenManager {
    async fn issue(
        &self,
        _subject: mokkan_core::application::dto::TokenSubject,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthTokenDto> {
        // For tests return a deterministic token payload so exchange flows can be tested
        let now = super::time::fixed_now();
        let expires_at = now + chrono::Duration::hours(1);
        Ok(mokkan_core::application::dto::AuthTokenDto {
            token: format!("issued-{}", i64::from(_subject.user_id)),
            issued_at: now,
            expires_at,
            expires_in: (expires_at.signed_duration_since(now).num_seconds()),
            session_id: _subject.session_id.clone(),
            refresh_token: None,
        })
    }

    async fn authenticate(
        &self,
        token: &str,
    ) -> mokkan_core::application::ApplicationResult<mokkan_core::application::dto::AuthenticatedUser> {
        let now = super::time::fixed_now();
        match token {
            TEST_TOKEN => Ok(admin_audit_user(now)),
            SESSION_TOKEN => Ok(session_user(now)),
            NO_AUDIT_TOKEN => Ok(author_user(now)),
            // Expired tokens should be rejected at authentication time
            EXPIRED_TOKEN => Err(mokkan_core::application::error::ApplicationError::unauthorized("expired token")),
            _ => Err(mokkan_core::application::error::ApplicationError::unauthorized("invalid token")),
        }
    }

    async fn public_jwk(&self) -> mokkan_core::application::ApplicationResult<serde_json::Value> {
        // Return a minimal JWKS structure for tests. Real implementations will return
        // the actual public key material used to verify tokens.
        Ok(serde_json::json!({ "keys": [] }))
    }
}

fn admin_audit_user(now: DateTime<Utc>) -> mokkan_core::application::dto::AuthenticatedUser {
    // For tests, treat the admin test-token as having the Admin role's default capabilities
    // plus the audit:read capability so it can access audit endpoints used in tests.
    let mut caps = mokkan_core::domain::user::value_objects::Role::Admin.default_capabilities();
    caps.insert(mokkan_core::domain::user::value_objects::Capability::new("audit", "read"));

    mokkan_core::application::dto::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(1).expect("invalid user id"),
        username: "tester".into(),
        role: mokkan_core::domain::user::value_objects::Role::Admin,
        capabilities: caps,
        issued_at: now,
        expires_at: now + Duration::hours(1),
        session_id: None,
        token_version: None,
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
        session_id: None,
        token_version: None,
    }
}

fn session_user(now: DateTime<Utc>) -> mokkan_core::application::dto::AuthenticatedUser {
    mokkan_core::application::dto::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(4).expect("invalid user id"),
        username: "sessioned".into(),
        role: mokkan_core::domain::user::value_objects::Role::Author,
        capabilities: HashSet::from([
            mokkan_core::domain::user::value_objects::Capability::new("articles", "create"),
        ]),
        issued_at: now,
        expires_at: now + Duration::hours(1),
        session_id: Some("sid-1".into()),
        token_version: Some(1),
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
        session_id: None,
        token_version: None,
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