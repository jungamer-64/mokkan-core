// src/presentation/http/controllers/articles.rs
use crate::application::{
    commands::articles::{
        CreateArticleCommand, DeleteArticleCommand, SetPublishStateCommand, UpdateArticleCommand,
    },
    dto::{ArticleDto, ArticleRevisionDto, CursorPage},
    queries::articles::{
        GetArticleBySlugQuery, ListArticleRevisionsQuery, ListArticlesQuery, SearchArticlesQuery,
    },
};
use crate::presentation::http::error::{ErrorResponse, HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::openapi::{ArticleListResponse, StatusResponse};
use crate::presentation::http::state::HttpState;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use serde::Deserialize;
use serde_json::json;
use utoipa::IntoParams;

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize, IntoParams, utoipa::ToSchema)]
pub struct ArticleListParams {
    #[serde(default)]
    pub include_drafts: bool,
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateArticleRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub publish: bool,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub publish: Option<bool>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct PublishRequest {
    pub publish: bool,
}

#[utoipa::path(
    get,
    path = "/api/v1/articles",
    params(ArticleListParams),
    responses(
        (status = 200, description = "List articles.", body = ArticleListResponse),
        (status = 400, description = "Invalid query parameters.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    tag = "Articles"
)]
pub async fn list_articles(
    Extension(state): Extension<HttpState>,
    actor: MaybeAuthenticated,
    Query(params): Query<ArticleListParams>,
) -> HttpResult<Json<CursorPage<ArticleDto>>> {
    let include_drafts = params.include_drafts;
    let limit = params.limit;
    let cursor = params.cursor.clone();

    let result = if let Some(query) = params.q.clone() {
        state
            .services
            .article_queries
            .search_articles(
                actor.0.as_ref(),
                SearchArticlesQuery {
                    query,
                    include_drafts,
                    limit,
                    cursor: cursor.clone(),
                },
            )
            .await
            .into_http()?
    } else {
        state
            .services
            .article_queries
            .list_articles(
                actor.0.as_ref(),
                ListArticlesQuery {
                    include_drafts,
                    limit,
                    cursor,
                },
            )
            .await
            .into_http()?
    };

    Ok(Json(result))
}

#[utoipa::path(
    get,
    path = "/api/v1/articles/by-slug/{slug}",
    params(
        ("slug" = String, Path, description = "Article slug")
    ),
    responses(
        (status = 200, description = "Article by slug.", body = ArticleDto),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 404, description = "Article not found.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    tag = "Articles"
)]
pub async fn get_article_by_slug(
    Extension(state): Extension<HttpState>,
    actor: MaybeAuthenticated,
    Path(slug): Path<String>,
) -> HttpResult<Json<ArticleDto>> {
    state
        .services
        .article_queries
        .get_article_by_slug(actor.0.as_ref(), GetArticleBySlugQuery { slug })
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/articles",
    request_body = CreateArticleRequest,
    responses(
        (status = 200, description = "Article created.", body = ArticleDto),
        (status = 400, description = "Invalid input.", body = ErrorResponse),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
pub async fn create_article(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Json(payload): Json<CreateArticleRequest>,
) -> HttpResult<Json<ArticleDto>> {
    let command = CreateArticleCommand {
        title: payload.title,
        body: payload.body,
        publish: payload.publish,
    };

    state
        .services
        .article_commands
        .create_article(&user, command)
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    put,
    path = "/api/v1/articles/{id}",
    params(
        ("id" = i64, Path, description = "Article identifier")
    ),
    request_body = UpdateArticleRequest,
    responses(
        (status = 200, description = "Article updated.", body = ArticleDto),
        (status = 400, description = "Invalid input.", body = ErrorResponse),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 404, description = "Article not found.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
pub async fn update_article(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateArticleRequest>,
) -> HttpResult<Json<ArticleDto>> {
    let command = UpdateArticleCommand {
        id,
        title: payload.title,
        body: payload.body,
        publish: payload.publish,
    };

    state
        .services
        .article_commands
        .update_article(&user, command)
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    delete,
    path = "/api/v1/articles/{id}",
    params(
        ("id" = i64, Path, description = "Article identifier")
    ),
    responses(
        (status = 200, description = "Article deleted.", body = StatusResponse),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 404, description = "Article not found.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
pub async fn delete_article(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> HttpResult<Json<serde_json::Value>> {
    state
        .services
        .article_commands
        .delete_article(&user, DeleteArticleCommand { id })
        .await
        .into_http()?;

    Ok(Json(json!({ "status": "deleted" })))
}

#[utoipa::path(
    post,
    path = "/api/v1/articles/{id}/publish",
    params(
        ("id" = i64, Path, description = "Article identifier")
    ),
    request_body = PublishRequest,
    responses(
        (status = 200, description = "Article publish state updated.", body = ArticleDto),
        (status = 400, description = "Invalid input.", body = ErrorResponse),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 404, description = "Article not found.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
pub async fn set_publish_state(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<PublishRequest>,
) -> HttpResult<Json<ArticleDto>> {
    let command = SetPublishStateCommand {
        id,
        publish: payload.publish,
    };

    state
        .services
        .article_commands
        .set_publish_state(&user, command)
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/articles/{id}/revisions",
    params(
        ("id" = i64, Path, description = "Article identifier")
    ),
    responses(
        (status = 200, description = "Article revision history.", body = [ArticleRevisionDto]),
        (status = 401, description = "Unauthorized.", body = ErrorResponse),
        (status = 403, description = "Forbidden.", body = ErrorResponse),
        (status = 404, description = "Article not found.", body = ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
pub async fn list_article_revisions(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> HttpResult<Json<Vec<ArticleRevisionDto>>> {
    state
        .services
        .article_queries
        .list_revisions(&user, ListArticleRevisionsQuery { article_id: id })
        .await
        .into_http()
        .map(Json)
}
