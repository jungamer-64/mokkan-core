// src/domain/article/repository.rs
use crate::domain::article::entity::{Article, ArticleUpdate, NewArticle};
use crate::domain::article::revision::ArticleRevision;
use crate::domain::article::value_objects::{ArticleId, ArticleListCursor, ArticleSlug};
use crate::domain::errors::DomainResult;
use crate::domain::user::UserId;
use async_trait::async_trait;

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
    /// Existing page-oriented listing API. Keep for backward compatibility.
    async fn list_page(
        &self,
        include_drafts: bool,
        limit: u32,
        cursor: Option<ArticleListCursor>,
        search: Option<&str>,
    ) -> DomainResult<(Vec<Article>, Option<ArticleListCursor>)>;

    /// New builder-style query API. Default implementation delegates to
    /// `list_page` so existing implementations remain compatible.
    async fn list(
        &self,
        query: ArticleQuery,
    ) -> DomainResult<(Vec<Article>, Option<ArticleListCursor>)> {
        // Convert Option<String> -> Option<&str> for the old API
        let search = query.search.as_deref();
        self.list_page(
            query.include_drafts,
            query.limit,
            query.cursor.clone(),
            search,
        )
        .await
    }
}

/// Builder-style query for listing articles.
#[derive(Debug, Clone)]
pub struct ArticleQuery {
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<ArticleListCursor>,
    pub search: Option<String>,
}

impl ArticleQuery {
    pub fn new() -> Self {
        Self {
            include_drafts: false,
            limit: 20,
            cursor: None,
            search: None,
        }
    }

    pub fn include_drafts(mut self, value: bool) -> Self {
        self.include_drafts = value;
        self
    }

    pub fn limit(mut self, value: u32) -> Self {
        self.limit = value.clamp(1, 100);
        self
    }

    pub fn cursor(mut self, value: ArticleListCursor) -> Self {
        self.cursor = Some(value);
        self
    }

    pub fn search(mut self, value: impl Into<String>) -> Self {
        self.search = Some(value.into());
        self
    }
}

#[async_trait]
pub trait ArticleRevisionRepository: Send + Sync {
    async fn append(&self, article: &Article, edited_by: Option<UserId>) -> DomainResult<()>;

    async fn list_by_article(&self, article_id: ArticleId) -> DomainResult<Vec<ArticleRevision>>;
}
