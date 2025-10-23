// tests/support/helpers.rs
use std::sync::Arc;
use crate::support::mocks as mocks;

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
