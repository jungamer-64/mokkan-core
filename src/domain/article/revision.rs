#![allow(clippy::module_name_repetitions)]

// src/domain/article/revision.rs
use crate::domain::article::value_objects::{ArticleBody, ArticleId, ArticleSlug, ArticleTitle};
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ArticleRevision {
    pub article_id: ArticleId,
    pub version: i32,
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: UserId,
    pub edited_by: Option<UserId>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
#[must_use]
pub struct ArticleRevisionParts {
    pub article_id: ArticleId,
    pub version: i32,
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: UserId,
    pub edited_by: Option<UserId>,
    pub recorded_at: DateTime<Utc>,
}

impl From<ArticleRevisionParts> for ArticleRevision {
    fn from(parts: ArticleRevisionParts) -> Self {
        let ArticleRevisionParts {
            article_id,
            version,
            title,
            slug,
            body,
            published,
            published_at,
            author_id,
            edited_by,
            recorded_at,
        } = parts;

        Self {
            article_id,
            version,
            title,
            slug,
            body,
            published,
            published_at,
            author_id,
            edited_by,
            recorded_at,
        }
    }
}
