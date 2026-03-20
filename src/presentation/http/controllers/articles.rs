// src/presentation/http/controllers/articles.rs
use crate::application::{
    ArticleDto, ArticleRevisionDto,
    commands::articles::{
        CreateArticleCommand, DeleteArticleCommand, SetPublishStateCommand, UpdateArticleCommand,
    },
    queries::articles::{
        GetArticleBySlugQuery, ListArticleRevisionsQuery, ListArticlesQuery, SearchArticlesQuery,
    },
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::openapi::{ArticleListResponse, StatusResponse};
use crate::presentation::http::state::HttpContext;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use serde::Deserialize;
use utoipa::IntoParams;

const fn default_limit() -> u32 {
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
        (status = 400, description = "Invalid query parameters.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security([]),
    tag = "Articles"
)]
/// List articles visible to the caller.
///
/// # Errors
///
/// Returns an error if query validation fails, draft access is forbidden, or
/// the article query service fails.
pub async fn list(
    Extension(state): Extension<HttpContext>,
    actor: MaybeAuthenticated,
    Query(params): Query<ArticleListParams>,
) -> HttpResult<Json<ArticleListResponse>> {
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

    Ok(Json(ArticleListResponse::from(result)))
}

#[utoipa::path(
    get,
    path = "/api/v1/articles/by-slug/{slug}",
    params(
        ("slug" = String, Path, description = "Article slug")
    ),
    responses(
        (status = 200, description = "Article by slug.", body = ArticleDto),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "Article not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security([]),
    tag = "Articles"
)]
/// Load a single article by slug.
///
/// # Errors
///
/// Returns an error if the slug is invalid, the article is missing, or the
/// caller cannot view an unpublished article.
pub async fn get_by_slug(
    Extension(state): Extension<HttpContext>,
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
/// Create a new article.
///
/// # Errors
///
/// Returns an error if authentication or authorization fails, the payload is
/// invalid, or the command service fails.
pub async fn create(
    Extension(state): Extension<HttpContext>,
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "Article not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
/// Update an existing article.
///
/// # Errors
///
/// Returns an error if authentication or authorization fails, the payload is
/// invalid, the article is missing, or the command service fails.
pub async fn update(
    Extension(state): Extension<HttpContext>,
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
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "Article not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
/// Delete an article.
///
/// # Errors
///
/// Returns an error if authentication or authorization fails, the article is
/// missing, or the command service fails.
pub async fn delete(
    Extension(state): Extension<HttpContext>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> HttpResult<Json<StatusResponse>> {
    state
        .services
        .article_commands
        .delete_article(&user, DeleteArticleCommand { id })
        .await
        .into_http()?;

    Ok(Json(StatusResponse {
        status: "deleted".into(),
    }))
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "Article not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
/// Change an article's published state.
///
/// # Errors
///
/// Returns an error if authentication or authorization fails, the payload is
/// invalid, the article is missing, or the command service fails.
pub async fn set_publish_state(
    Extension(state): Extension<HttpContext>,
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
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "Article not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Articles"
)]
/// List revision history for an article.
///
/// # Errors
///
/// Returns an error if authentication or authorization fails, the article is
/// missing, or the query service fails.
pub async fn list_revisions(
    Extension(state): Extension<HttpContext>,
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
