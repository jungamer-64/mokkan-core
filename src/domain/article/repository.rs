// src/domain/article/repository.rs
use crate::domain::article::entity::{Article, ArticleUpdate, NewArticle};
use crate::domain::article::value_objects::{ArticleId, ArticleListCursor, ArticleSlug};
use crate::domain::errors::DomainResult;
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
    async fn list_page(
        &self,
        include_drafts: bool,
        limit: u32,
        cursor: Option<ArticleListCursor>,
        search: Option<&str>,
    ) -> DomainResult<(Vec<Article>, Option<ArticleListCursor>)>;
}
