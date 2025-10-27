use crate::domain::article::{Article, ArticleRevision};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::serde_time;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArticleDto {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub published: bool,
    #[serde(default, with = "serde_time::option")]
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: i64,
    #[serde(with = "serde_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "serde_time")]
    pub updated_at: DateTime<Utc>,
}

impl From<Article> for ArticleDto {
    fn from(article: Article) -> Self {
        Self {
            id: article.id.into(),
            title: article.title.into_inner(),
            slug: article.slug.into_inner(),
            body: article.body.into_inner(),
            published: article.published,
            published_at: article.published_at,
            author_id: article.author_id.into(),
            created_at: article.created_at,
            updated_at: article.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArticleRevisionDto {
    pub version: i32,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub published: bool,
    #[serde(default, with = "serde_time::option")]
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: i64,
    #[serde(default)]
    pub edited_by: Option<i64>,
    #[serde(with = "serde_time")]
    pub recorded_at: DateTime<Utc>,
}

impl From<ArticleRevision> for ArticleRevisionDto {
    fn from(revision: ArticleRevision) -> Self {
        Self {
            version: revision.version,
            title: revision.title.into_inner(),
            slug: revision.slug.into_inner(),
            body: revision.body.into_inner(),
            published: revision.published,
            published_at: revision.published_at,
            author_id: revision.author_id.into(),
            edited_by: revision.edited_by.map(Into::into),
            recorded_at: revision.recorded_at,
        }
    }
}
