#![allow(clippy::multiple_crate_versions)]

use async_trait::async_trait;
use axum::{
    Extension, Router,
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::post,
};
use mokkan_core::{
    application::{
        dto::{AuthTokenDto, AuthenticatedUser, TokenSubject},
        ports::security::{PasswordHasher, TokenManager},
        services::{ApplicationDependencies, ApplicationRuntimeDependencies, ApplicationServices},
    },
    presentation::http::{
        extractors::Authenticated, middleware::require_capabilities, state::HttpState,
    },
};
use std::{
    collections::HashSet,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tower::util::ServiceExt as _;

mod support;

#[derive(Clone)]
struct CountingTokenManager {
    authenticate_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl TokenManager for CountingTokenManager {
    async fn issue(
        &self,
        _subject: TokenSubject,
    ) -> mokkan_core::application::ApplicationResult<AuthTokenDto> {
        unimplemented!("issue is not used in this test")
    }

    async fn authenticate(
        &self,
        token: &str,
    ) -> mokkan_core::application::ApplicationResult<AuthenticatedUser> {
        self.authenticate_calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(token, "counted-token");

        let now = chrono::Utc::now();
        Ok(AuthenticatedUser {
            id: mokkan_core::domain::user::value_objects::UserId::new(1).unwrap(),
            username: "counter".into(),
            role: mokkan_core::domain::user::value_objects::Role::Author,
            capabilities: HashSet::from([
                mokkan_core::domain::user::value_objects::Capability::new("articles", "create"),
            ]),
            issued_at: now,
            expires_at: now + chrono::Duration::hours(1),
            session_id: None,
            token_version: None,
        })
    }

    async fn public_jwk(&self) -> mokkan_core::application::ApplicationResult<serde_json::Value> {
        Ok(serde_json::json!({ "keys": [] }))
    }
}

async fn protected(Authenticated(_user): Authenticated) -> StatusCode {
    StatusCode::NO_CONTENT
}

fn lazy_pool() -> sqlx::Pool<sqlx::Postgres> {
    use sqlx::postgres::PgPoolOptions;
    PgPoolOptions::new()
        .connect_lazy("postgres://localhost/postgres")
        .expect("connect_lazy")
}

fn test_state(token_manager: Arc<dyn TokenManager>) -> HttpState {
    let deps = ApplicationDependencies {
        user_repo: Arc::new(support::mocks::DummyRepo),
        article_write_repo: Arc::new(support::mocks::DummyArticleWrite),
        article_read_repo: Arc::new(support::mocks::DummyArticleRead),
        article_revision_repo: Arc::new(support::mocks::DummyArticleRevision),
        audit_log_repo: Arc::new(support::mocks::MockAuditRepo),
    };

    let services = Arc::new(ApplicationServices::new(
        deps,
        ApplicationRuntimeDependencies {
            password_hasher: Arc::new(support::mocks::DummyPasswordHasher)
                as Arc<dyn PasswordHasher>,
            token_manager,
            refresh_token_codec: Arc::new(
                mokkan_core::infrastructure::security::refresh_token::HmacRefreshTokenCodec::new(
                    "test-refresh-secret",
                )
                .expect("refresh token codec"),
            ),
            session_revocation_store: Arc::new(
                mokkan_core::infrastructure::security::session_store::InMemorySessionRevocationStore::new(
                ),
            ),
            authorization_code_store: Arc::new(
                mokkan_core::infrastructure::security::authorization_code_store::InMemoryAuthorizationCodeStore::new(
                ),
            ),
            clock: Arc::new(support::mocks::DummyClock),
            slugger: Arc::new(support::mocks::DummySlug),
        },
    ));

    HttpState {
        services,
        db_pool: lazy_pool(),
    }
}

#[tokio::test]
async fn capability_middleware_reuses_authenticated_user_in_handler() {
    let calls = Arc::new(AtomicUsize::new(0));
    let state = test_state(Arc::new(CountingTokenManager {
        authenticate_calls: Arc::clone(&calls),
    }));

    let app = Router::new()
        .route(
            "/protected",
            post(protected).layer(axum::middleware::from_fn(move |req, next| {
                require_capabilities::require_capability(req, next, "articles", "create")
            })),
        )
        .layer(Extension(state));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/protected")
                .header(AUTHORIZATION, "Bearer counted-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}
