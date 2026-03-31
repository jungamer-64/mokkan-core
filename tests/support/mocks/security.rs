// tests/support/mocks/security.rs
use chrono::{DateTime, Duration, Utc};
use mokkan_core::async_support::{BoxFuture, boxed};
use std::collections::HashSet;

/// テスト用トークン定数（タイポ防止とIDE補完のため）
pub const TEST_TOKEN: &str = "test-token";
pub const NO_AUDIT_TOKEN: &str = "no-audit";
pub const EXPIRED_TOKEN: &str = "expired-token";
pub const SESSION_TOKEN: &str = "session-token";

/* -------------------------------- TokenManager -------------------------------- */

#[derive(Clone, Debug, Default)]
pub struct DummyTokenManager;

impl mokkan_core::application::ports::security::TokenManager for DummyTokenManager {
    fn issue(
        &self,
        _subject: mokkan_core::application::TokenSubject,
    ) -> BoxFuture<'_, mokkan_core::application::AppResult<mokkan_core::application::AuthTokenDto>>
    {
        boxed(async move {
            // For tests return a deterministic token payload so exchange flows can be tested
            let now = super::time::fixed_now();
            let expires_at = now + chrono::Duration::hours(1);
            Ok(mokkan_core::application::AuthTokenDto {
                token: format!("issued-{}", i64::from(_subject.user_id)),
                issued_at: now,
                expires_at,
                expires_in: expires_at.signed_duration_since(now).num_seconds(),
                session_id: _subject.session_id,
                refresh_token: None,
            })
        })
    }

    fn authenticate<'a>(
        &'a self,
        token: &'a str,
    ) -> BoxFuture<
        'a,
        mokkan_core::application::AppResult<mokkan_core::application::AuthenticatedUser>,
    > {
        boxed(async move {
            let now = super::time::fixed_now();
            match token {
                TEST_TOKEN => Ok(admin_audit_user(now)),
                SESSION_TOKEN => Ok(session_user(now)),
                NO_AUDIT_TOKEN => Ok(author_user(now)),
                // Expired tokens should be rejected at authentication time
                EXPIRED_TOKEN => Err(mokkan_core::application::error::AppError::unauthorized(
                    "expired token",
                )),
                _ => Err(mokkan_core::application::error::AppError::unauthorized(
                    "invalid token",
                )),
            }
        })
    }

    fn public_jwk(&self) -> BoxFuture<'_, mokkan_core::application::AppResult<serde_json::Value>> {
        boxed(async move {
            // Return a minimal JWKS structure for tests. Real implementations will return
            // the actual public key material used to verify tokens.
            Ok(serde_json::json!({ "keys": [] }))
        })
    }
}

fn admin_audit_user(now: DateTime<Utc>) -> mokkan_core::application::AuthenticatedUser {
    // For tests, treat the admin test-token as having the Admin role's default capabilities
    // plus the audit:read capability so it can access audit endpoints used in tests.
    let mut caps = mokkan_core::domain::user::value_objects::Role::Admin.default_capabilities();
    caps.insert(mokkan_core::domain::user::value_objects::Capability::new(
        "audit", "read",
    ));

    mokkan_core::application::AuthenticatedUser {
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

fn author_user(now: DateTime<Utc>) -> mokkan_core::application::AuthenticatedUser {
    mokkan_core::application::AuthenticatedUser {
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

fn session_user(now: DateTime<Utc>) -> mokkan_core::application::AuthenticatedUser {
    mokkan_core::application::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(4).expect("invalid user id"),
        username: "sessioned".into(),
        role: mokkan_core::domain::user::value_objects::Role::Author,
        capabilities: HashSet::from([mokkan_core::domain::user::value_objects::Capability::new(
            "articles", "create",
        )]),
        issued_at: now,
        expires_at: now + Duration::hours(1),
        session_id: Some("sid-1".into()),
        token_version: Some(1),
    }
}

#[allow(dead_code)]
fn expired_admin_audit_user(now: DateTime<Utc>) -> mokkan_core::application::AuthenticatedUser {
    mokkan_core::application::AuthenticatedUser {
        id: mokkan_core::domain::user::value_objects::UserId::new(3).expect("invalid user id"),
        username: "expired".into(),
        role: mokkan_core::domain::user::value_objects::Role::Admin,
        capabilities: HashSet::from([mokkan_core::domain::user::value_objects::Capability::new(
            "audit", "read",
        )]),
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

impl mokkan_core::application::ports::security::PasswordHasher for DummyPasswordHasher {
    fn hash<'a>(
        &'a self,
        _password: &'a str,
    ) -> BoxFuture<'a, mokkan_core::application::AppResult<String>> {
        boxed(async move { Ok("hash".into()) })
    }

    fn verify<'a>(
        &'a self,
        _password: &'a str,
        _expected_hash: &'a str,
    ) -> BoxFuture<'a, mokkan_core::application::AppResult<()>> {
        boxed(async move { Ok(()) })
    }
}

/// 厳密なパスワードハッシャー（ネガティブパステスト用）
#[derive(Clone, Debug, Default)]
pub struct StrictPasswordHasher;

impl mokkan_core::application::ports::security::PasswordHasher for StrictPasswordHasher {
    fn hash<'a>(
        &'a self,
        password: &'a str,
    ) -> BoxFuture<'a, mokkan_core::application::AppResult<String>> {
        boxed(async move { Ok(format!("hash::{password}")) })
    }

    fn verify<'a>(
        &'a self,
        password: &'a str,
        expected_hash: &'a str,
    ) -> BoxFuture<'a, mokkan_core::application::AppResult<()>> {
        boxed(async move {
            if format!("hash::{password}") == expected_hash {
                Ok(())
            } else {
                Err(mokkan_core::application::error::AppError::unauthorized(
                    "bad password",
                ))
            }
        })
    }
}
