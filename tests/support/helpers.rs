// tests/support/helpers.rs
use std::sync::Arc;
use super::mocks as mocks;
use axum::body;
use axum::body::Body;
use axum::http::StatusCode;
use serde_json::Value;

pub async fn build_test_state() -> mokkan_core::presentation::http::state::HttpState {
    // Build services with mocks
    let user_repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> = Arc::new(mocks::DummyUserRepo);
    let article_write: Arc<dyn mokkan_core::domain::article::repository::ArticleWriteRepository> = Arc::new(mocks::DummyArticleWrite);
    let article_read: Arc<dyn mokkan_core::domain::article::repository::ArticleReadRepository> = Arc::new(mocks::DummyArticleRead);
    let article_rev: Arc<dyn mokkan_core::domain::article::repository::ArticleRevisionRepository> = Arc::new(mocks::DummyArticleRevision);
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> = Arc::new(mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> = Arc::new(mocks::DummyTokenManager);
    let audit_repo: Arc<dyn mokkan_core::domain::audit::repository::AuditLogRepository> = Arc::new(mocks::MockAuditRepo);
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> = Arc::new(mocks::DummyClock);
    let slugger: Arc<dyn mokkan_core::application::ports::util::SlugGenerator> = Arc::new(mocks::DummySlug);

    let services = Arc::new(mokkan_core::application::services::ApplicationServices::new(
        user_repo,
        article_write,
        article_read,
        article_rev,
        password_hasher,
        token_manager,
        audit_repo,
        clock,
        slugger,
    ));

    // PgPool: use lazy connect string so tests don't actually connect
    use sqlx::postgres::PgPoolOptions;
    let db_pool = PgPoolOptions::new().connect_lazy("postgres://localhost").expect("connect_lazy");

    mokkan_core::presentation::http::state::HttpState { services, db_pool }
}

pub async fn make_test_router() -> axum::Router {
    let state = build_test_state().await;
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

/// Build a test router but inject a custom audit repo (useful for E2E tests).
pub async fn make_test_router_with_audit_repo(audit_repo: std::sync::Arc<dyn mokkan_core::domain::audit::repository::AuditLogRepository>) -> axum::Router {
    use std::sync::Arc as _Arc;

    // Build services with the provided audit repo and default mocks for others
    let user_repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> = Arc::new(mocks::DummyUserRepo);
    let article_write: Arc<dyn mokkan_core::domain::article::repository::ArticleWriteRepository> = Arc::new(mocks::DummyArticleWrite);
    let article_read: Arc<dyn mokkan_core::domain::article::repository::ArticleReadRepository> = Arc::new(mocks::DummyArticleRead);
    let article_rev: Arc<dyn mokkan_core::domain::article::repository::ArticleRevisionRepository> = Arc::new(mocks::DummyArticleRevision);
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> = Arc::new(mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> = Arc::new(mocks::DummyTokenManager);
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> = Arc::new(mocks::DummyClock);
    let slugger: Arc<dyn mokkan_core::application::ports::util::SlugGenerator> = Arc::new(mocks::DummySlug);

    let services = Arc::new(mokkan_core::application::services::ApplicationServices::new(
        user_repo,
        article_write,
        article_read,
        article_rev,
        password_hasher,
        token_manager,
        // use provided audit repo
        audit_repo,
        clock,
        slugger,
    ));

    // PgPool: use lazy connect string so tests don't actually connect
    use sqlx::postgres::PgPoolOptions;
    let db_pool = PgPoolOptions::new().connect_lazy("postgres://localhost").expect("connect_lazy");

    let state = mokkan_core::presentation::http::state::HttpState { services, db_pool };
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

/// Assert that a response is an ErrorResponse JSON with the expected status and error string.
pub async fn assert_error_response(resp: axum::response::Response, expected_status: StatusCode, expected_error: &str) {
    // Check status first
    assert_eq!(resp.status(), expected_status);
    let (parts, body_stream) = resp.into_parts();
    let body_bytes = body::to_bytes(body_stream, 1024 * 1024).await.expect("read body");
    let ct = parts.headers.get("content-type").and_then(|v| v.to_str().ok()).unwrap_or("");
    assert!(ct.starts_with("application/json"), "unexpected content-type: {}", ct);
    let json: Value = serde_json::from_slice(&body_bytes).expect("expected valid json body for error");
    let err_field = json.get("error").and_then(|v| v.as_str()).unwrap_or("");
    let msg_field = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(err_field, expected_error, "unexpected error field: {}", err_field);
    assert!(!msg_field.is_empty(), "expected non-empty message field in ErrorResponse");
}
