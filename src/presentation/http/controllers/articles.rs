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
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::state::HttpState;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use serde::Deserialize;
use serde_json::json;

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct CreateArticleRequest {
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub publish: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateArticleRequest {
    pub title: Option<String>,
    pub body: Option<String>,
    pub publish: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct PublishRequest {
    pub publish: bool,
}

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
