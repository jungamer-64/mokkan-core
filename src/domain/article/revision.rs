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

impl ArticleRevision {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        article_id: ArticleId,
        version: i32,
        title: ArticleTitle,
        slug: ArticleSlug,
        body: ArticleBody,
        published: bool,
        published_at: Option<DateTime<Utc>>,
        author_id: UserId,
        edited_by: Option<UserId>,
        recorded_at: DateTime<Utc>,
    ) -> Self {
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
