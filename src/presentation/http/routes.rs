// src/presentation/http/routes.rs
use crate::presentation::http::state::HttpState;
use crate::presentation::http::{
    controllers::{articles, auth},
    middleware::rate_limit,
    openapi::{self, StatusResponse},
};
use crate::presentation::http::controllers::audit;
use axum::{
    Extension, Router,
    http::{Method, header::HeaderValue},
    routing::{get, patch, post, put},
};
use tower_http::cors::AllowOrigin;
use std::time::Duration;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router_with_rate_limiter(state: HttpState, enable_rate_limiter: bool) -> Router {
    // prefer reading allowed origins from env directly so tests don't have to provide BISCUIT key
    let origins = crate::config::AppConfig::allowed_origins_from_env();

    let cors = if origins.iter().any(|o| o == "*") {
        CorsLayer::new().allow_origin(tower_http::cors::Any)
    } else {
        let origin_list: Vec<_> = origins
            .iter()
            .filter_map(|s| s.parse::<HeaderValue>().ok())
            .collect();
        CorsLayer::new().allow_origin(AllowOrigin::list(origin_list))
    }
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
        .max_age(Duration::from_secs(3600));

    let mut router = Router::new()
        .merge(openapi::docs_router())
        .merge(system_routes())
        .merge(auth_routes())
        .merge(user_routes())
        .merge(audit_routes())
        .merge(article_routes())
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(Extension(state));

    // apply rate limiter only when requested. Tests can call the alternative constructor
    // and pass `false` to avoid the governor dependency on real remote addresses.
    if enable_rate_limiter {
        router = router.layer(rate_limit::rate_limit_layer());
    }

    router
}

/// Backwards-compatible wrapper that reads the `DISABLE_RATE_LIMIT` env var to decide
/// whether to enable the governor rate limiter. Production code can continue to call
/// `build_router(state)`.
pub fn build_router(state: HttpState) -> Router {
    let disable = std::env::var("DISABLE_RATE_LIMIT").as_deref() == Ok("1");
    build_router_with_rate_limiter(state, !disable)
}

fn audit_routes() -> Router {
    Router::new()
        .route(
            "/api/v1/audit-logs",
            get(audit::list_audit_logs),
        )
        .route(
            "/api/v1/audit-logs/user/{id}",
            get(audit::list_audit_logs_by_user),
        )
        .route(
            "/api/v1/audit-logs/resource/{type}/{id}",
            get(audit::list_audit_logs_by_resource),
        )
}

fn system_routes() -> Router {
    Router::new().route("/health", get(health))
}

fn auth_routes() -> Router {
    Router::new()
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/refresh", post(auth::refresh_token))
        .route("/api/v1/auth/me", get(auth::profile))
}

fn user_routes() -> Router {
    Router::new()
        .route("/api/v1/users", get(auth::list_users))
        .route("/api/v1/users/{id}", patch(auth::update_user))
        .route(
            "/api/v1/users/{id}/change-password",
            post(auth::change_password),
        )
}

fn article_routes() -> Router {
    Router::new()
        .route(
            "/api/v1/articles",
            get(articles::list_articles).post(articles::create_article),
        )
        .route(
            "/api/v1/articles/by-slug/{slug}",
            get(articles::get_article_by_slug),
        )
        .route(
            "/api/v1/articles/{id}",
            put(articles::update_article).delete(articles::delete_article),
        )
        .route(
            "/api/v1/articles/{id}/revisions",
            get(articles::list_article_revisions),
        )
        .route(
            "/api/v1/articles/{id}/publish",
            post(articles::set_publish_state),
        )
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health check.", body = crate::presentation::http::openapi::StatusResponse)
    ),
    security([]),
    tag = "System"
)]
pub async fn health() -> axum::Json<StatusResponse> {
    axum::Json(StatusResponse {
        status: "ok".into(),
    })
}
