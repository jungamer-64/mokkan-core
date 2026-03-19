// src/infrastructure/repositories/articles/revision.rs
use super::super::map_sqlx;
use crate::domain::article::{
    Article, ArticleBody, ArticleId, ArticleRevision, ArticleRevisionParts,
    ArticleRevisionRepository, ArticleSlug, ArticleTitle,
};
use crate::domain::errors::DomainResult;
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

#[derive(Clone)]
#[must_use]
pub struct PostgresArticleRevisionRepository {
    pool: PgPool,
}

impl PostgresArticleRevisionRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct ArticleRevisionRow {
    article_id: i64,
    version: i32,
    title: String,
    slug: String,
    body: String,
    published: bool,
    published_at: Option<DateTime<Utc>>,
    author_id: i64,
    edited_by: Option<i64>,
    recorded_at: DateTime<Utc>,
}

impl TryFrom<ArticleRevisionRow> for ArticleRevision {
    type Error = crate::domain::errors::DomainError;

    fn try_from(row: ArticleRevisionRow) -> Result<Self, Self::Error> {
        Ok(ArticleRevisionParts {
            article_id: ArticleId::new(row.article_id)?,
            version: row.version,
            title: ArticleTitle::new(row.title)?,
            slug: ArticleSlug::new(row.slug)?,
            body: ArticleBody::new(row.body)?,
            published: row.published,
            published_at: row.published_at,
            author_id: UserId::new(row.author_id)?,
            edited_by: row.edited_by.map(UserId::new).transpose()?,
            recorded_at: row.recorded_at,
        }
        .into())
    }
}

#[async_trait]
impl ArticleRevisionRepository for PostgresArticleRevisionRepository {
    async fn append(&self, article: &Article, edited_by: Option<UserId>) -> DomainResult<()> {
        let edited_by = edited_by.map(i64::from);

        sqlx::query(
            r"
            WITH next_version AS (
                SELECT COALESCE(MAX(version) + 1, 1) AS version
                FROM article_revisions
                WHERE article_id = $1
            )
            INSERT INTO article_revisions (
                article_id, version, title, slug, body, published, published_at,
                author_id, edited_by
            )
            SELECT
                $1,
                next_version.version,
                $2, $3, $4, $5, $6,
                $7, $8
            FROM next_version
            ",
        )
        .bind(i64::from(article.id))
        .bind(article.title.as_str())
        .bind(article.slug.as_str())
        .bind(article.body.as_str())
        .bind(article.published)
        .bind(article.published_at)
        .bind(i64::from(article.author_id))
        .bind(edited_by)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(())
    }

    async fn list_by_article(&self, article_id: ArticleId) -> DomainResult<Vec<ArticleRevision>> {
        let rows = sqlx::query_as::<_, ArticleRevisionRow>(
            r"
            SELECT article_id, version, title, slug, body, published, published_at,
                   author_id, edited_by, recorded_at
            FROM article_revisions
            WHERE article_id = $1
            ORDER BY version DESC
            ",
        )
        .bind(i64::from(article_id))
        .fetch_all(&self.pool)
        .await
        .map_err(map_sqlx)?;

        rows.into_iter()
            .map(ArticleRevision::try_from)
            .collect::<Result<Vec<_>, _>>()
    }
}
