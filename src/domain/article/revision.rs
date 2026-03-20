// src/domain/article/revision.rs
use crate::domain::UserId;
use crate::domain::article::value_objects::{ArticleBody, ArticleId, ArticleSlug, ArticleTitle};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Revision {
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
pub struct Parts {
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

impl From<Parts> for Revision {
    fn from(parts: Parts) -> Self {
        let Parts {
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
