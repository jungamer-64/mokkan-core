// src/infrastructure/repositories/postgres_article.rs
use crate::domain::article::{
    Article, ArticleBody, ArticleId, ArticleReadRepository, ArticleSlug, ArticleTitle,
    ArticleUpdate, ArticleWriteRepository, NewArticle,
};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};
use std::sync::Arc;

fn map_error(err: sqlx::Error) -> DomainError {
    DomainError::Persistence(err.to_string())
}

#[derive(Clone)]
pub struct PostgresArticleWriteRepository {
    pool: Arc<PgPool>,
}

impl PostgresArticleWriteRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct PostgresArticleReadRepository {
    pool: Arc<PgPool>,
}

impl PostgresArticleReadRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

#[derive(Debug, FromRow)]
struct ArticleRow {
    id: i64,
    title: String,
    slug: String,
    body: String,
    published: bool,
    published_at: Option<DateTime<Utc>>,
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
            published: row.published,
            published_at: row.published_at,
            author_id: UserId::new(row.author_id)?,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[async_trait]
impl ArticleWriteRepository for PostgresArticleWriteRepository {
    async fn insert(&self, article: NewArticle) -> DomainResult<Article> {
        let NewArticle {
            title,
            slug,
            body,
            published,
            published_at,
            author_id,
            created_at,
            updated_at,
        } = article;

        let row = sqlx::query_as::<_, ArticleRow>(
            "INSERT INTO articles (title, slug, body, published, published_at, author_id, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, title, slug, body, published, published_at, author_id, created_at, updated_at",
        )
        .bind(title.as_str())
        .bind(slug.as_str())
        .bind(body.as_str())
        .bind(published)
        .bind(published_at)
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
            publish_state,
            updated_at,
        } = update;

        let mut builder: QueryBuilder<Postgres> =
            QueryBuilder::new("UPDATE articles SET updated_at = ");
        builder.push_bind(updated_at);

        if let Some(title) = title {
            let title_str: String = title.into();
            builder.push(", title = ");
            builder.push_bind(title_str);
        }

        if let Some(slug) = slug {
            let slug_str: String = slug.into();
            builder.push(", slug = ");
            builder.push_bind(slug_str);
        }

        if let Some(body) = body {
            let body_str: String = body.into();
            builder.push(", body = ");
            builder.push_bind(body_str);
        }

        if let Some(state) = publish_state {
            builder.push(", published = ");
            builder.push_bind(state.published);
            builder.push(", published_at = ");
            builder.push_bind(state.published_at);
        }

        builder.push(" WHERE id = ");
        builder.push_bind(i64::from(id));
        builder.push(
            " RETURNING id, title, slug, body, published, published_at, author_id, created_at, updated_at",
        );

        let row = builder
            .build_query_as::<ArticleRow>()
            .fetch_one(&*self.pool)
            .await
            .map_err(map_error)?;

        Article::try_from(row)
    }

    async fn delete(&self, id: ArticleId) -> DomainResult<()> {
        sqlx::query("DELETE FROM articles WHERE id = $1")
            .bind(i64::from(id))
            .execute(&*self.pool)
            .await
            .map_err(map_error)?;
        Ok(())
    }
}

#[async_trait]
impl ArticleReadRepository for PostgresArticleReadRepository {
    async fn find_by_id(&self, id: ArticleId) -> DomainResult<Option<Article>> {
        let row = sqlx::query_as::<_, ArticleRow>(
            "SELECT id, title, slug, body, published, published_at, author_id, created_at, updated_at
             FROM articles WHERE id = $1",
        )
        .bind(i64::from(id))
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_error)?;

        row.map(Article::try_from).transpose()
    }

    async fn find_by_slug(&self, slug: &ArticleSlug) -> DomainResult<Option<Article>> {
        let row = sqlx::query_as::<_, ArticleRow>(
            "SELECT id, title, slug, body, published, published_at, author_id, created_at, updated_at
             FROM articles WHERE slug = $1",
        )
        .bind(slug.as_str())
        .fetch_optional(&*self.pool)
        .await
        .map_err(map_error)?;

        row.map(Article::try_from).transpose()
    }

    async fn list_paginated(
        &self,
        include_drafts: bool,
        page: u32,
        page_size: u32,
        search: Option<&str>,
    ) -> DomainResult<(Vec<Article>, u64)> {
        let page = page.max(1);
        let page_size = page_size.max(1);
        let offset = ((page - 1) as i64) * page_size as i64;
        let search_pattern = search
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s));

        fn apply_conditions<'a>(
            builder: &mut QueryBuilder<'a, Postgres>,
            include_drafts: bool,
            search_pattern: Option<&'a str>,
        ) {
            let mut has_where = false;
            if !include_drafts {
                builder.push(" WHERE published = TRUE");
                has_where = true;
            }

            if let Some(pattern) = search_pattern {
                if has_where {
                    builder.push(" AND (");
                } else {
                    builder.push(" WHERE (");
                }
                builder.push("title ILIKE ");
                builder.push_bind(pattern);
                builder.push(" OR body ILIKE ");
                builder.push_bind(pattern);
                builder.push(")");
            }
        }

        let mut list_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, title, slug, body, published, published_at, author_id, created_at, updated_at FROM articles",
        );
        apply_conditions(&mut list_builder, include_drafts, search_pattern.as_deref());
        list_builder.push(" ORDER BY created_at DESC LIMIT ");
        list_builder.push_bind(page_size as i64);
        list_builder.push(" OFFSET ");
        list_builder.push_bind(offset);

        let rows = list_builder
            .build_query_as::<ArticleRow>()
            .fetch_all(&*self.pool)
            .await
            .map_err(map_error)?;

        let mut count_builder: QueryBuilder<Postgres> =
            QueryBuilder::new("SELECT COUNT(1) as count FROM articles");
        apply_conditions(
            &mut count_builder,
            include_drafts,
            search_pattern.as_deref(),
        );

        let total: i64 = count_builder
            .build_query_scalar()
            .fetch_one(&*self.pool)
            .await
            .map_err(map_error)?;

        let articles = rows
            .into_iter()
            .map(Article::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        Ok((articles, total as u64))
    }
}
