use crate::domain::article::{
    Article, ArticleBody, ArticleId, ArticleReadRepository, ArticleSlug, ArticleTitle,
    ArticleUpdate, ArticleWriteRepository, NewArticle,
};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use std::sync::Arc;

fn map_error(err: sqlx::Error) -> DomainError {
    DomainError::Persistence(err.to_string())
}

#[derive(Clone)]
pub struct SqliteArticleWriteRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteArticleWriteRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct SqliteArticleReadRepository {
    pool: Arc<SqlitePool>,
}

impl SqliteArticleReadRepository {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct ArticleRow {
    id: i64,
    title: String,
    slug: String,
    body: String,
    published: i64,
    author_id: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<ArticleRow> for Article {
    type Error = DomainError;

    fn try_from(row: ArticleRow) -> Result<Self, Self::Error> {
        Ok(Article {
            id: ArticleId::new(row.id)?,
            title: ArticleTitle::new(row.title)?,
            slug: ArticleSlug::new(row.slug)?,
            body: ArticleBody::new(row.body)?,
            published: row.published != 0,
            author_id: UserId::new(row.author_id)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait]
impl ArticleWriteRepository for SqliteArticleWriteRepository {
    async fn insert(&self, article: NewArticle) -> DomainResult<Article> {
        let NewArticle {
            title,
            slug,
            body,
            published,
            author_id,
            created_at,
            updated_at,
        } = article;

        let row = sqlx::query_as::<_, ArticleRow>(
            "INSERT INTO articles (title, slug, body, published, author_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id, title, slug, body, published, author_id, created_at, updated_at",
        )
        .bind(title.as_str())
        .bind(slug.as_str())
        .bind(body.as_str())
        .bind(if published { 1 } else { 0 })
        .bind(i64::from(author_id))
        .bind(created_at)
        .bind(updated_at)
        .fetch_one(&*self.pool)
        .await
        .map_err(map_error)?;

        Article::try_from(row)
    }

    async fn update(&self, update: ArticleUpdate) -> DomainResult<Article> {
        let ArticleUpdate {
            id,
            title,
            slug,
            body,
            published,
            updated_at,
        } = update;

        let row = sqlx::query_as::<_, ArticleRow>(
            "UPDATE articles SET title = COALESCE(?, title), slug = COALESCE(?, slug), body = COALESCE(?, body), published = COALESCE(?, published), updated_at = ? WHERE id = ? RETURNING id, title, slug, body, published, author_id, created_at, updated_at",
        )
        .bind(title.as_ref().map(|t| t.as_str()))
        .bind(slug.as_ref().map(|s| s.as_str()))
        .bind(body.as_ref().map(|b| b.as_str()))
        .bind(published.map(|p| if p { 1 } else { 0 }))
        .bind(updated_at)
        .bind(i64::from(id))
        .fetch_one(&*self.pool)
        .await
        .map_err(map_error)?;

        Article::try_from(row)
    }

    async fn delete(&self, id: ArticleId) -> DomainResult<()> {
        sqlx::query("DELETE FROM articles WHERE id = ?")
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await
            .map_err(map_error)?;
        Ok(())
    }
}

#[async_trait]
impl ArticleReadRepository for SqliteArticleReadRepository {
    async fn find_by_id(&self, id: ArticleId) -> DomainResult<Option<Article>> {
        let row = sqlx::query_as::<_, ArticleRow>(
            "SELECT id, title, slug, body, published, author_id, created_at, updated_at FROM articles WHERE id = ?",
        )
        .bind(i64::from(id))
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_error)?;

        row.map(Article::try_from).transpose()
    }

    async fn find_by_slug(&self, slug: &ArticleSlug) -> DomainResult<Option<Article>> {
        let row = sqlx::query_as::<_, ArticleRow>(
            "SELECT id, title, slug, body, published, author_id, created_at, updated_at FROM articles WHERE slug = ?",
        )
        .bind(slug.as_str())
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_error)?;

        row.map(Article::try_from).transpose()
    }

    async fn list(&self, include_drafts: bool) -> DomainResult<Vec<Article>> {
        let rows = if include_drafts {
            sqlx::query_as::<_, ArticleRow>(
                "SELECT id, title, slug, body, published, author_id, created_at, updated_at FROM articles ORDER BY created_at DESC",
            )
            .fetch_all(&*self.pool)
            .await
        } else {
            sqlx::query_as::<_, ArticleRow>(
                "SELECT id, title, slug, body, published, author_id, created_at, updated_at FROM articles WHERE published = 1 ORDER BY created_at DESC",
            )
            .fetch_all(&*self.pool)
            .await
        }
        .map_err(map_error)?;

        rows.into_iter().map(Article::try_from).collect()
    }
}
