// tests/support/helpers.rs
use std::sync::Arc;
use super::mocks;
use axum::body;
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::response::Response;
use axum::http::HeaderMap;
use serde_json::Value;

/// テスト用のHTTPステートを構築
pub async fn build_test_state() -> mokkan_core::presentation::http::state::HttpState {
    // モックでサービスを構築
    let user_repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> =
        Arc::new(mocks::DummyUserRepo);
    let article_write: Arc<dyn mokkan_core::domain::article::repository::ArticleWriteRepository> =
        Arc::new(mocks::DummyArticleWrite);
    let article_read: Arc<dyn mokkan_core::domain::article::repository::ArticleReadRepository> =
        Arc::new(mocks::DummyArticleRead);
    let article_rev: Arc<dyn mokkan_core::domain::article::repository::ArticleRevisionRepository> =
        Arc::new(mocks::DummyArticleRevision);
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> =
        Arc::new(mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> =
        Arc::new(mocks::DummyTokenManager);
    let audit_repo: Arc<dyn mokkan_core::domain::audit::repository::AuditLogRepository> =
        Arc::new(mocks::MockAuditRepo);
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> =
        Arc::new(mocks::DummyClock);
    let slugger: Arc<dyn mokkan_core::application::ports::util::SlugGenerator> =
        Arc::new(mocks::DummySlug);

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

    // PgPool: lazy connect文字列を使用してテストが実際に接続しないようにする
    use sqlx::postgres::PgPoolOptions;
    let db_pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost")
        .expect("connect_lazy");

    mokkan_core::presentation::http::state::HttpState { services, db_pool }
}

/// テスト用ルーターを作成
pub async fn make_test_router() -> axum::Router {
    let state = build_test_state().await;
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

/// カスタム監査リポジトリを注入したテストルーターを作成（E2Eテスト用）
pub async fn make_test_router_with_audit_repo(
    audit_repo: Arc<dyn mokkan_core::domain::audit::repository::AuditLogRepository>,
) -> axum::Router {
    // 提供された監査リポジトリと他のデフォルトモックでサービスを構築
    let user_repo: Arc<dyn mokkan_core::domain::user::repository::UserRepository> =
        Arc::new(mocks::DummyUserRepo);
    let article_write: Arc<dyn mokkan_core::domain::article::repository::ArticleWriteRepository> =
        Arc::new(mocks::DummyArticleWrite);
    let article_read: Arc<dyn mokkan_core::domain::article::repository::ArticleReadRepository> =
        Arc::new(mocks::DummyArticleRead);
    let article_rev: Arc<dyn mokkan_core::domain::article::repository::ArticleRevisionRepository> =
        Arc::new(mocks::DummyArticleRevision);
    let password_hasher: Arc<dyn mokkan_core::application::ports::security::PasswordHasher> =
        Arc::new(mocks::DummyPasswordHasher);
    let token_manager: Arc<dyn mokkan_core::application::ports::security::TokenManager> =
        Arc::new(mocks::DummyTokenManager);
    let clock: Arc<dyn mokkan_core::application::ports::time::Clock> =
        Arc::new(mocks::DummyClock);
    let slugger: Arc<dyn mokkan_core::application::ports::util::SlugGenerator> =
        Arc::new(mocks::DummySlug);

    let services = Arc::new(mokkan_core::application::services::ApplicationServices::new(
        user_repo,
        article_write,
        article_read,
        article_rev,
        password_hasher,
        token_manager,
        audit_repo, // 提供された監査リポジトリを使用
        clock,
        slugger,
    ));

    // PgPool: lazy connect文字列を使用してテストが実際に接続しないようにする
    use sqlx::postgres::PgPoolOptions;
    let db_pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost")
        .expect("connect_lazy");

    let state = mokkan_core::presentation::http::state::HttpState { services, db_pool };
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

/// レスポンスが期待されるステータスとエラー文字列を持つErrorResponse JSONであることをアサート
pub async fn assert_error_response(
    resp: axum::response::Response,
    expected_status: StatusCode,
    expected_error: &str,
) {
    // まずステータスをチェック
    assert_eq!(resp.status(), expected_status);
    
    let (parts, body_stream) = resp.into_parts();
    let body_bytes = body::to_bytes(body_stream, 1024 * 1024)
        .await
        .expect("read body");
    
    let ct = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(
        ct.starts_with("application/json"),
        "unexpected content-type: {}",
        ct
    );
    
    let json: Value = serde_json::from_slice(&body_bytes)
        .expect("expected valid json body for error");
    
    let err_field = json.get("error").and_then(|v| v.as_str()).unwrap_or("");
    let msg_field = json.get("message").and_then(|v| v.as_str()).unwrap_or("");
    
    assert_eq!(err_field, expected_error, "unexpected error field: {}", err_field);
    assert!(
        !msg_field.is_empty(),
        "expected non-empty message field in ErrorResponse"
    );
}

/// Convert an axum Response into its headers and parsed JSON body.
pub async fn to_json(resp: Response) -> (HeaderMap, serde_json::Value) {
    let (parts, body_stream) = resp.into_parts();
    let bytes = axum::body::to_bytes(body_stream, 1024 * 1024)
        .await
        .expect("read body");
    let json: serde_json::Value = serde_json::from_slice(&bytes).expect("parse json");
    (parts.headers, json)
}
