// tests/support/helpers.rs
use std::sync::Arc;
use super::mocks;

// Macros that expand at the call site so panics report the test caller location.
// Named with `_async` to clarify these return an async block that must be awaited.
#[macro_export]
macro_rules! assert_error_response_async {
    ($resp:expr, $expected_status:expr, $expected_error:expr) => {{
        let __resp = $resp;
        async move {
            // ステータスをチェック
            assert_eq!(__resp.status(), $expected_status);

            let (parts, body_stream) = __resp.into_parts();
            let body_bytes = axum::body::to_bytes(body_stream, 1024 * 1024)
                .await
                .expect("read body");

            let ct = parts
                .headers
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            assert!(
                ct.starts_with("application/json"),
                "unexpected content-type: {}",
                ct
            );

            let json: serde_json::Value = serde_json::from_slice(&body_bytes)
                .expect("expected valid json body for error");

            let err_field = json.get("error").and_then(|v| v.as_str()).unwrap_or("");
            let msg_field = json.get("message").and_then(|v| v.as_str()).unwrap_or("");

            assert_eq!(err_field, $expected_error, "unexpected error field: {}", err_field);
            assert!(
                !msg_field.is_empty(),
                "expected non-empty message field in ErrorResponse"
            );
        }
    }};
}

#[macro_export]
macro_rules! to_json_async {
    ($resp:expr) => {{
        let __resp = $resp;
        async move {
            let (parts, body_stream) = __resp.into_parts();
            let bytes = axum::body::to_bytes(body_stream, 1024 * 1024)
                .await
                .expect("read body");
            let json: serde_json::Value = serde_json::from_slice(&bytes).expect("parse json");
            (parts.headers, json)
        }
    }};
}

// Short aliases to keep function signatures compact for static analysis tools
type AuditRepo =
    dyn mokkan_core::domain::audit::repository::AuditLogRepository + Send + Sync + 'static;
type UserRepo =
    dyn mokkan_core::domain::user::repository::UserRepository + Send + Sync + 'static;
type ArticleWriteRepo =
    dyn mokkan_core::domain::article::repository::ArticleWriteRepository + Send + Sync + 'static;
type ArticleReadRepo =
    dyn mokkan_core::domain::article::repository::ArticleReadRepository + Send + Sync + 'static;
type ArticleRevisionRepo =
    dyn mokkan_core::domain::article::repository::ArticleRevisionRepository + Send + Sync + 'static;
type PasswordHasherPort =
    dyn mokkan_core::application::ports::security::PasswordHasher + Send + Sync + 'static;
type TokenManagerPort =
    dyn mokkan_core::application::ports::security::TokenManager + Send + Sync + 'static;
type ClockPort =
    dyn mokkan_core::application::ports::time::Clock + Send + Sync + 'static;
type SlugGeneratorPort =
    dyn mokkan_core::application::ports::util::SlugGenerator + Send + Sync + 'static;

/// テスト用のHTTPステートを構築
fn default_dependencies() -> (
    Arc<UserRepo>,
    Arc<ArticleWriteRepo>,
    Arc<ArticleReadRepo>,
    Arc<ArticleRevisionRepo>,
    Arc<PasswordHasherPort>,
    Arc<TokenManagerPort>,
    Arc<ClockPort>,
    Arc<SlugGeneratorPort>,
)
{
    (
        Arc::new(mocks::DummyUserRepo),
        Arc::new(mocks::DummyArticleWrite),
        Arc::new(mocks::DummyArticleRead),
        Arc::new(mocks::DummyArticleRevision),
        Arc::new(mocks::DummyPasswordHasher),
        Arc::new(mocks::DummyTokenManager),
        Arc::new(mocks::DummyClock),
        Arc::new(mocks::DummySlug),
    )
}

fn make_services(audit_repo: Arc<AuditRepo>) -> Arc<mokkan_core::application::services::ApplicationServices>
{
    let (
        user_repo,
        article_write,
        article_read,
        article_rev,
        password_hasher,
        token_manager,
        clock,
        slugger,
    ) = default_dependencies();

    Arc::new(mokkan_core::application::services::ApplicationServices::new(
        user_repo,
        article_write,
        article_read,
        article_rev,
        password_hasher,
        token_manager,
        audit_repo,
        clock,
        slugger,
    ))
}

/// Create a lazily-connected PgPool for tests.
fn lazy_pool() -> sqlx::Pool<sqlx::Postgres> {
    use sqlx::postgres::PgPoolOptions;
    PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("connect_lazy")
}

/// テスト用のHTTPステートを構築
pub async fn build_test_state() -> mokkan_core::presentation::http::state::HttpState {
    let services = make_services(Arc::new(mocks::MockAuditRepo));

    // PgPool: use shared helper
    let db_pool = lazy_pool();

    mokkan_core::presentation::http::state::HttpState { services, db_pool }
}

/// テスト用ルーターを作成
pub async fn make_test_router() -> axum::Router {
    let state = build_test_state().await;
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

/// カスタム監査リポジトリを注入したテストルーターを作成（E2Eテスト用）
pub async fn make_test_router_with_audit_repo(audit_repo: Arc<AuditRepo>) -> axum::Router {
    let services = make_services(audit_repo);

    // PgPool: use shared helper
    let db_pool = lazy_pool();

    let state = mokkan_core::presentation::http::state::HttpState { services, db_pool };
    mokkan_core::presentation::http::routes::build_router_with_rate_limiter(state, false)
}

// NOTE: assert_error_response is provided as a macro `assert_error_response_async!` that expands
// at the test call site so failure locations point to the test instead of this helper module.

// NOTE: to_json is provided as a macro `to_json_async!` that expands at the test call site.
