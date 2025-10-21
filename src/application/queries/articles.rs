// src/application/queries/articles.rs
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser, PaginatedResult},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{ArticleId, ArticleReadRepository, ArticleSlug},
};
use std::sync::Arc;

pub struct ListArticlesQuery {
    pub include_drafts: bool,
    pub page: u32,
    pub page_size: u32,
}

pub struct SearchArticlesQuery {
    pub query: String,
    pub include_drafts: bool,
    pub page: u32,
    pub page_size: u32,
}

pub struct GetArticleBySlugQuery {
    pub slug: String,
}

pub struct ArticleQueryService {
    read_repo: Arc<dyn ArticleReadRepository>,
}

impl ArticleQueryService {
    pub fn new(read_repo: Arc<dyn ArticleReadRepository>) -> Self {
        Self { read_repo }
    }

    pub async fn list_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: ListArticlesQuery,
    ) -> ApplicationResult<PaginatedResult<ArticleDto>> {
        let (include_drafts, page, page_size) =
            self.normalize_listing(actor, query.include_drafts, query.page, query.page_size)?;

        let (records, total) = self
            .read_repo
            .list_paginated(include_drafts, page, page_size, None)
            .await?;

        let dtos = records.into_iter().map(Into::into).collect();
        Ok(PaginatedResult::new(dtos, total, page, page_size))
    }

    pub async fn search_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: SearchArticlesQuery,
    ) -> ApplicationResult<PaginatedResult<ArticleDto>> {
        if query.query.trim().is_empty() {
            return self
                .list_articles(
                    actor,
                    ListArticlesQuery {
                        include_drafts: query.include_drafts,
                        page: query.page,
                        page_size: query.page_size,
                    },
                )
                .await;
        }

        let (include_drafts, page, page_size) =
            self.normalize_listing(actor, query.include_drafts, query.page, query.page_size)?;

        let (records, total) = self
            .read_repo
            .list_paginated(include_drafts, page, page_size, Some(query.query.as_str()))
            .await?;

        let dtos = records.into_iter().map(Into::into).collect();
        Ok(PaginatedResult::new(dtos, total, page, page_size))
    }

    pub async fn get_article_by_slug(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: GetArticleBySlugQuery,
    ) -> ApplicationResult<ArticleDto> {
        let slug = ArticleSlug::new(query.slug)?;
        let article = self
            .read_repo
            .find_by_slug(&slug)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        if !article.published {
            let actor = actor.ok_or_else(|| ApplicationError::not_found("article not found"))?;
            if !actor.has_capability("articles", "view:drafts") && actor.id != article.author_id {
                return Err(ApplicationError::not_found("article not found"));
            }
        }

        Ok(article.into())
    }

    pub async fn get_article_by_id(&self, id: i64) -> ApplicationResult<ArticleDto> {
        let id = ArticleId::new(id)?;
        let article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;
        Ok(article.into())
    }

    fn normalize_listing(
        &self,
        actor: Option<&AuthenticatedUser>,
        include_drafts: bool,
        page: u32,
        page_size: u32,
    ) -> ApplicationResult<(bool, u32, u32)> {
        let include_drafts = if include_drafts {
            let actor = actor.ok_or_else(|| {
                ApplicationError::forbidden("authentication required for draft access")
            })?;
            if !actor.has_capability("articles", "view:drafts") {
                return Err(ApplicationError::forbidden(
                    "missing capability articles:view:drafts",
                ));
            }
            true
        } else {
            false
        };

        const DEFAULT_PAGE: u32 = 1;
        const DEFAULT_PAGE_SIZE: u32 = 20;
        const MAX_PAGE_SIZE: u32 = 100;

        let page = if page == 0 { DEFAULT_PAGE } else { page };
        let page_size = if page_size == 0 {
            DEFAULT_PAGE_SIZE
        } else {
            page_size.min(MAX_PAGE_SIZE)
        };

        Ok((include_drafts, page, page_size))
    }
}
