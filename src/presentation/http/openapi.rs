// src/presentation/http/openapi.rs
use crate::application::dto::{ArticleDto, CursorPage, UserDto};
use axum::{Router, routing::get};
use serde::{Deserialize, Serialize};
use utoipa::openapi::{
    Components,
    security::{Http, HttpAuthScheme, SecurityScheme},
    server::Server,
};
use utoipa::{Modify, OpenApi, ToSchema};
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
        if servers.is_empty() {
            servers.push(Server::new("http://localhost:3000"));
        }
    }
}

pub async fn serve_openapi() -> axum::Json<utoipa::openapi::OpenApi> {
    axum::Json(ApiDoc::openapi())
}

pub fn docs_router() -> Router {
    let openapi = ApiDoc::openapi();
    Router::new()
        .route("/openapi.json", get(serve_openapi))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", openapi))
}

pub fn write_openapi_snapshot() -> std::io::Result<()> {
    let spec = ApiDoc::openapi();
    let json = serde_json::to_string_pretty(&spec)?;
    std::fs::create_dir_all("spec")?;
    std::fs::write("spec/openapi.json", json)?;
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
