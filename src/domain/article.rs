use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::user::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArticleId(pub i64);

impl ArticleId {
    pub fn new(id: i64) -> DomainResult<Self> {
        if id <= 0 {
            Err(DomainError::Validation(
                "article id must be positive".into(),
            ))
        } else {
            Ok(Self(id))
        }
    }
}

impl From<ArticleId> for i64 {
    fn from(value: ArticleId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleTitle(String);

impl ArticleTitle {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("title cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArticleTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ArticleTitle> for String {
    fn from(value: ArticleTitle) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleSlug(String);

impl ArticleSlug {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("slug cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArticleSlug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ArticleSlug> for String {
    fn from(value: ArticleSlug) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArticleBody(String);

impl ArticleBody {
    pub fn new(value: impl Into<String>) -> DomainResult<Self> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(DomainError::Validation("body cannot be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ArticleBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<ArticleBody> for String {
    fn from(value: ArticleBody) -> Self {
        value.0
    }
}

#[derive(Debug, Clone)]
pub struct Article {
    pub id: ArticleId,
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Article {
    pub fn publish(mut self, published: bool, updated_at: DateTime<Utc>) -> Self {
        self.published = published;
        self.updated_at = updated_at;
        self
    }
}

#[derive(Debug, Clone)]
pub struct NewArticle {
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ArticleUpdate {
    pub id: ArticleId,
    pub title: Option<ArticleTitle>,
    pub slug: Option<ArticleSlug>,
    pub body: Option<ArticleBody>,
    pub published: Option<bool>,
    pub updated_at: DateTime<Utc>,
}

impl ArticleUpdate {
    pub fn new(id: ArticleId, updated_at: DateTime<Utc>) -> Self {
        Self {
            id,
            title: None,
            slug: None,
            body: None,
            published: None,
            updated_at,
        }
    }

    pub fn with_title(mut self, title: ArticleTitle) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_slug(mut self, slug: ArticleSlug) -> Self {
        self.slug = Some(slug);
        self
    }

    pub fn with_body(mut self, body: ArticleBody) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_published(mut self, published: bool) -> Self {
        self.published = Some(published);
        self
    }
}

#[async_trait]
pub trait ArticleWriteRepository: Send + Sync {
    async fn insert(&self, article: NewArticle) -> DomainResult<Article>;
    async fn update(&self, update: ArticleUpdate) -> DomainResult<Article>;
    async fn delete(&self, id: ArticleId) -> DomainResult<()>;
}

#[async_trait]
pub trait ArticleReadRepository: Send + Sync {
    async fn find_by_id(&self, id: ArticleId) -> DomainResult<Option<Article>>;
    async fn find_by_slug(&self, slug: &ArticleSlug) -> DomainResult<Option<Article>>;
    async fn list(&self, include_drafts: bool) -> DomainResult<Vec<Article>>;
}
