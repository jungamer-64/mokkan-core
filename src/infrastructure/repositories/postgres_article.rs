// src/infrastructure/repositories/postgres_article.rs
use super::map_sqlx;
use crate::domain::article::{
    Article, ArticleBody, ArticleId, ArticleListCursor, ArticleReadRepository, ArticleSlug,
    ArticleTitle, ArticleUpdate, ArticleWriteRepository, NewArticle,
};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, QueryBuilder};

#[derive(Clone)]
pub struct PostgresArticleWriteRepository {
    pool: PgPool,
}

impl PostgresArticleWriteRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(Clone)]
pub struct PostgresArticleReadRepository {
    pool: PgPool,
}

impl PostgresArticleReadRepository {
    pub fn new(pool: PgPool) -> Self {
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
        .fetch_one(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Article::try_from(row)
    }

    async fn update(&self, update: ArticleUpdate) -> DomainResult<Article> {
        let ArticleUpdate {
            id,
            title,
            slug,
            body,
            publish_state,
            original_updated_at,
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
        builder.push(" AND updated_at = ");
        builder.push_bind(original_updated_at);
        builder.push(
            " RETURNING id, title, slug, body, published, published_at, author_id, created_at, updated_at",
        );

        let maybe_row = builder
            .build_query_as::<ArticleRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_sqlx)?;

        let row = maybe_row
            .ok_or_else(|| DomainError::Conflict("article update conflict, please retry".into()))?;

        Article::try_from(row)
    }

    async fn delete(&self, id: ArticleId) -> DomainResult<()> {
        let result = sqlx::query("DELETE FROM articles WHERE id = $1")
            .bind(i64::from(id))
            .execute(&self.pool)
            .await
            .map_err(map_sqlx)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("article not found".into()));
        }
        Ok(())
    }
}

enum SearchMode<'q> {
    None,
    FullText(&'q str),
    Trigram(&'q str),
}

impl PostgresArticleReadRepository {
    fn apply_conditions<'a>(
        builder: &mut QueryBuilder<'a, Postgres>,
        include_drafts: bool,
        cursor: Option<&'a ArticleListCursor>,
        mode: &SearchMode<'a>,
    ) {
        let mut has_where = false;
        if !include_drafts {
            builder.push(" WHERE published = TRUE");
            has_where = true;
        }

        match mode {
            SearchMode::FullText(query) => {
                if has_where {
                    builder.push(" AND ");
                } else {
                    builder.push(" WHERE ");
                    has_where = true;
                }
                builder.push("search @@ plainto_tsquery('simple', ");
                builder.push_bind(*query);
                builder.push(")");
            }
            SearchMode::Trigram(pattern) => {
                if has_where {
                    builder.push(" AND (");
                } else {
                    builder.push(" WHERE (");
                    has_where = true;
                }
                builder.push("title ILIKE ");
                builder.push_bind(*pattern);
                builder.push(" OR body ILIKE ");
                builder.push_bind(*pattern);
                builder.push(")");
            }
            SearchMode::None => {}
        }

        if let Some(cursor) = cursor {
            if has_where {
                builder.push(" AND ");
            } else {
                builder.push(" WHERE ");
            }
            builder.push("(created_at, id) < (");
            builder.push_bind(cursor.created_at);
            builder.push(", ");
            builder.push_bind(i64::from(cursor.article_id));
            builder.push(")");
        }
    }

    fn apply_ordering<'a>(builder: &mut QueryBuilder<'a, Postgres>, mode: &SearchMode<'a>) {
        match mode {
            SearchMode::FullText(query) => {
                builder.push(" ORDER BY ts_rank(search, plainto_tsquery('simple', ");
                builder.push_bind(*query);
                builder.push(")) DESC, created_at DESC, id DESC");
            }
            _ => {
                builder.push(" ORDER BY created_at DESC, id DESC");
            }
        }
    }

    async fn fetch_page(
        &self,
        include_drafts: bool,
        limit: u32,
        cursor: Option<&ArticleListCursor>,
        mode: SearchMode<'_>,
    ) -> DomainResult<(Vec<Article>, Option<ArticleListCursor>)> {
        let limit = limit.clamp(1, 100);
        let fetch_limit = (limit as i64) + 1;

        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "SELECT id, title, slug, body, published, published_at, author_id, created_at, updated_at FROM articles",
        );
        Self::apply_conditions(&mut builder, include_drafts, cursor, &mode);
        Self::apply_ordering(&mut builder, &mode);
        builder.push(" LIMIT ");
        builder.push_bind(fetch_limit);

        let rows = builder
            .build_query_as::<ArticleRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;

        let mut articles = rows
            .into_iter()
            .map(Article::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        let mut next_cursor = None;
        if articles.len() > limit as usize {
            articles.pop();
            if let Some(last) = articles.last() {
                next_cursor = Some(ArticleListCursor::from_parts(last.created_at, last.id));
            }
        }

        Ok((articles, next_cursor))
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
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        row.map(Article::try_from).transpose()
    }

    async fn find_by_slug(&self, slug: &ArticleSlug) -> DomainResult<Option<Article>> {
        let row = sqlx::query_as::<_, ArticleRow>(
            "SELECT id, title, slug, body, published, published_at, author_id, created_at, updated_at
             FROM articles WHERE slug = $1",
        )
        .bind(slug.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_sqlx)?;

        row.map(Article::try_from).transpose()
    }

    async fn list_page(
        &self,
        include_drafts: bool,
        limit: u32,
        cursor: Option<ArticleListCursor>,
        search: Option<&str>,
    ) -> DomainResult<(Vec<Article>, Option<ArticleListCursor>)> {
        let cursor_ref = cursor.as_ref();

        if let Some(query) = search.map(str::trim).filter(|s| !s.is_empty()) {
            let (articles, next_cursor) = self
                .fetch_page(
                    include_drafts,
                    limit,
                    cursor_ref,
                    SearchMode::FullText(query),
                )
                .await?;

            if !articles.is_empty() {
                return Ok((articles, next_cursor));
            }

            let pattern = format!("%{}%", query);
            return self
                .fetch_page(
                    include_drafts,
                    limit,
                    cursor_ref,
                    SearchMode::Trigram(&pattern),
                )
                .await;
        }

        self.fetch_page(include_drafts, limit, cursor_ref, SearchMode::None)
            .await
    }
}
