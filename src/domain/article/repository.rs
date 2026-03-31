// src/domain/article/repository.rs
use crate::async_support::{BoxFuture, boxed};
use crate::domain::UserId;
use crate::domain::article::entity::{Article, ArticleUpdate, NewArticle};
use crate::domain::article::revision::Revision;
use crate::domain::article::value_objects::{ArticleId, ArticleListCursor, ArticleSlug};
use crate::domain::errors::DomainResult;

pub trait WriteRepo: Send + Sync {
    fn insert(&self, article: NewArticle) -> BoxFuture<'_, DomainResult<Article>>;
    fn update(&self, update: ArticleUpdate) -> BoxFuture<'_, DomainResult<Article>>;
    fn delete(&self, id: ArticleId) -> BoxFuture<'_, DomainResult<()>>;
}

pub trait ReadRepo: Send + Sync {
    fn find_by_id(&self, id: ArticleId) -> BoxFuture<'_, DomainResult<Option<Article>>>;
    fn find_by_slug<'a>(
        &'a self,
        slug: &'a ArticleSlug,
    ) -> BoxFuture<'a, DomainResult<Option<Article>>>;
    /// Existing page-oriented listing API. Keep for backward compatibility.
    fn list_page<'a>(
        &'a self,
        include_drafts: bool,
        limit: u32,
        cursor: Option<ArticleListCursor>,
        search: Option<&'a str>,
    ) -> BoxFuture<'a, DomainResult<(Vec<Article>, Option<ArticleListCursor>)>>;

    /// New builder-style query API. Default implementation delegates to
    /// `list_page` so existing implementations remain compatible.
    fn list(
        &self,
        query: ArticleQuery,
    ) -> BoxFuture<'_, DomainResult<(Vec<Article>, Option<ArticleListCursor>)>> {
        boxed(async move {
            // Convert Option<String> -> Option<&str> for the old API
            let search = query.search.as_deref();
            self.list_page(
                query.include_drafts,
                query.limit,
                query.cursor.clone(),
                search,
            )
            .await
        })
    }
}

/// Builder-style query for listing articles.
#[derive(Debug, Clone)]
#[must_use]
pub struct ArticleQuery {
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<ArticleListCursor>,
    pub search: Option<String>,
}

impl ArticleQuery {
    pub const fn new() -> Self {
        Self {
            include_drafts: false,
            limit: 20,
            cursor: None,
            search: None,
        }
    }

    pub const fn include_drafts(mut self, value: bool) -> Self {
        self.include_drafts = value;
        self
    }

    pub fn limit(mut self, value: u32) -> Self {
        self.limit = value.clamp(1, 100);
        self
    }

    pub const fn cursor(mut self, value: ArticleListCursor) -> Self {
        self.cursor = Some(value);
        self
    }

    pub fn search(mut self, value: impl Into<String>) -> Self {
        self.search = Some(value.into());
        self
    }
}

impl Default for ArticleQuery {
    fn default() -> Self {
        Self::new()
    }
}

pub trait RevisionRepo: Send + Sync {
    fn append<'a>(
        &'a self,
        article: &'a Article,
        edited_by: Option<UserId>,
    ) -> BoxFuture<'a, DomainResult<()>>;

    fn list_by_article(&self, article_id: ArticleId) -> BoxFuture<'_, DomainResult<Vec<Revision>>>;
}
