// src/presentation/http/openapi.rs
use crate::application::dto::{ArticleDto, CursorPage, UserDto};
use axum::{Router, response::Redirect, routing::get};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs::File, io::BufWriter, path::Path};
use utoipa::openapi::{
    Components,
    security::{Http, HttpAuthScheme, SecurityScheme},
    server::Server,
};
use utoipa::{Modify, OpenApi, ToSchema};
use utoipa_redoc::{Redoc, Servable};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ArticleListResponse {
    pub items: Vec<ArticleDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::presentation::http::controllers::auth::register,
        crate::presentation::http::controllers::auth::login,
        crate::presentation::http::controllers::auth::refresh_token,
        crate::presentation::http::controllers::auth::profile,
        crate::presentation::http::controllers::auth::list_users,
        crate::presentation::http::controllers::auth::update_user,
        crate::presentation::http::controllers::auth::change_password,
        crate::presentation::http::controllers::articles::list_articles,
        crate::presentation::http::controllers::articles::get_article_by_slug,
        crate::presentation::http::controllers::articles::create_article,
        crate::presentation::http::controllers::articles::update_article,
        crate::presentation::http::controllers::articles::delete_article,
        crate::presentation::http::controllers::articles::list_article_revisions,
        crate::presentation::http::controllers::articles::set_publish_state,
        super::routes::health
    ),
    components(
        schemas(
            StatusResponse,
            UserListResponse,
            ArticleListResponse,
            crate::presentation::http::error::ErrorResponse,
            crate::presentation::http::controllers::auth::RegisterRequest,
            crate::presentation::http::controllers::auth::LoginRequest,
            crate::presentation::http::controllers::auth::LoginResponse,
            crate::presentation::http::controllers::auth::RefreshTokenRequest,
            crate::presentation::http::controllers::auth::ListUsersParams,
            crate::presentation::http::controllers::auth::UpdateUserRequest,
            crate::presentation::http::controllers::auth::ChangePasswordRequest,
            crate::presentation::http::controllers::articles::ArticleListParams,
            crate::presentation::http::controllers::articles::CreateArticleRequest,
            crate::presentation::http::controllers::articles::UpdateArticleRequest,
            crate::presentation::http::controllers::articles::PublishRequest,
            crate::application::dto::UserDto,
            crate::application::dto::UserProfileDto,
            crate::application::dto::AuthTokenDto,
            crate::application::dto::CapabilityView,
            crate::application::dto::ArticleDto,
            crate::application::dto::ArticleRevisionDto
        )
    ),
    tags(
        (name = "Auth", description = "Authentication and session endpoints"),
        (name = "Users", description = "User management endpoints"),
        (name = "Articles", description = "Article management endpoints"),
        (name = "System", description = "System level endpoints")
    ),
    modifiers(&ApiDocCustomizer),
    security(("bearerAuth" = [])),
    info(
        title = "Mokkan API",
        description = "Headless CMS backend",
        version = "0.1.0"
    )
)]
pub struct ApiDoc;

struct ApiDocCustomizer;

impl Modify for ApiDocCustomizer {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::default);
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("JWT".into());
        components.add_security_scheme("bearerAuth", SecurityScheme::Http(http));

        let servers = openapi.servers.get_or_insert_with(Vec::new);
        servers.clear();

        let mut urls: Vec<String> = env::var("PUBLIC_API_URLS")
            .ok()
            .map(|value| {
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|segment| !segment.is_empty())
                    .map(|segment| segment.trim_end_matches('/').to_string())
                    .collect()
            })
            .unwrap_or_default();

        if urls.is_empty() {
            if let Ok(url) = env::var("PUBLIC_API_URL") {
                let sanitized = url.trim().trim_end_matches('/').to_string();
                if !sanitized.is_empty() {
                    urls.push(sanitized);
                }
            }
        }

        if !urls.iter().any(|url| url == "http://localhost:3000") {
            urls.push("http://localhost:3000".to_string());
        }

        let mut seen = HashSet::new();
        for url in urls {
            if seen.insert(url.clone()) {
                servers.push(Server::new(url));
            }
        }
    }
}

pub async fn serve_openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

pub fn docs_router() -> Router {
    let openapi = ApiDoc::openapi();
    let swagger = SwaggerUi::new("/docs").url("/openapi.json", openapi.clone());
    let redoc = Redoc::with_url("/redoc", openapi);
    Router::new()
        .route("/openapi.json", get(serve_openapi))
        .merge(swagger)
        .merge(redoc)
        .route("/", get(|| async { Redirect::permanent("/docs") }))
}

pub fn write_openapi_snapshot() -> std::io::Result<()> {
    let spec = ApiDoc::openapi();
    let output_path = env::var("OPENAPI_SNAPSHOT_PATH")
        .unwrap_or_else(|_| "backend/spec/openapi.json".to_string());
    let path = Path::new(&output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &spec)?;
    Ok(())
}

impl From<CursorPage<UserDto>> for UserListResponse {
    fn from(page: CursorPage<UserDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}

impl From<CursorPage<ArticleDto>> for ArticleListResponse {
    fn from(page: CursorPage<ArticleDto>) -> Self {
        Self {
            items: page.items,
            next_cursor: page.next_cursor,
            has_more: page.has_more,
        }
    }
}
