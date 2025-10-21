// src/presentation/http/routes.rs
use crate::presentation::http::state::HttpState;
use crate::presentation::http::{
    controllers::{articles, auth},
    openapi::{self, StatusResponse},
};
use axum::{
    Extension, Router,
    http::Method,
    routing::{get, patch, post, put},
};
use std::time::Duration;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub fn build_router(state: HttpState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
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

    Router::new()
        .merge(openapi::docs_router())
        .route("/health", get(health))
        .route("/api/v1/auth/register", post(auth::register))
        .route("/api/v1/auth/login", post(auth::login))
        .route("/api/v1/auth/me", get(auth::profile))
        .route("/api/v1/users", get(auth::list_users))
        .route("/api/v1/users/:id", patch(auth::update_user))
        .route(
            "/api/v1/users/:id/change-password",
            post(auth::change_password),
        )
        .route(
            "/api/v1/articles",
            get(articles::list_articles).post(articles::create_article),
        )
        .route(
            "/api/v1/articles/by-slug/:slug",
            get(articles::get_article_by_slug),
        )
        .route(
            "/api/v1/articles/:id",
            put(articles::update_article).delete(articles::delete_article),
        )
        .route(
            "/api/v1/articles/:id/revisions",
            get(articles::list_article_revisions),
        )
        .route(
            "/api/v1/articles/:id/publish",
            post(articles::set_publish_state),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(Extension(state))
}

#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health check.", body = crate::presentation::http::openapi::StatusResponse)
    ),
    tag = "System"
)]
pub async fn health() -> axum::Json<StatusResponse> {
    axum::Json(StatusResponse {
        status: "ok".into(),
    })
}
