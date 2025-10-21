// src/presentation/http/openapi.rs
use crate::application::dto::{ArticleDto, UserDto};
use axum::Json;
use serde::Serialize;
use utoipa::openapi::{
    Components,
    security::{Http, HttpAuthScheme, SecurityScheme},
};
use utoipa::{Modify, OpenApi, ToSchema};

#[derive(Debug, Serialize, ToSchema)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserListResponse {
    pub items: Vec<UserDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, ToSchema)]
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
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Components::default);
        let mut http = Http::new(HttpAuthScheme::Bearer);
        http.bearer_format = Some("JWT".into());
        components.add_security_scheme("bearerAuth", SecurityScheme::Http(http));
    }
}

pub async fn serve_openapi() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
