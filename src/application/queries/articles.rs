use crate::{
    application::{
        dto::{ArticleDto, ArticleRevisionDto, AuthenticatedUser, CursorPage},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::specifications::{ArticleSpecification, CanUpdateArticleSpec},
    domain::article::{
        ArticleId, ArticleListCursor, ArticleReadRepository, ArticleRevisionRepository, ArticleSlug,
    },
    domain::errors::DomainError,
};
use std::sync::Arc;

pub struct ListArticlesQuery {
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}

pub struct SearchArticlesQuery {
    pub query: String,
    pub include_drafts: bool,
    pub limit: u32,
    pub cursor: Option<String>,
}

pub struct GetArticleBySlugQuery {
    pub slug: String,
}

pub struct ListArticleRevisionsQuery {
    pub article_id: i64,
}

pub struct ArticleQueryService {
    read_repo: Arc<dyn ArticleReadRepository>,
    revision_repo: Arc<dyn ArticleRevisionRepository>,
}

impl ArticleQueryService {
    pub fn new(
        read_repo: Arc<dyn ArticleReadRepository>,
        revision_repo: Arc<dyn ArticleRevisionRepository>,
    ) -> Self {
        Self {
            read_repo,
            revision_repo,
        }
    }

    pub async fn list_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: ListArticlesQuery,
    ) -> ApplicationResult<CursorPage<ArticleDto>> {
        let (include_drafts, limit) =
            self.normalize_listing(actor, query.include_drafts, query.limit)?;
        let cursor = self.decode_cursor(query.cursor.as_deref())?;

        let (records, next_cursor) = self
            .read_repo
            .list_page(include_drafts, limit, cursor, None)
            .await?;

        let items = records.into_iter().map(Into::into).collect();
        Ok(CursorPage::new(
            items,
            next_cursor.map(|cursor| cursor.encode()),
        ))
    }

    pub async fn search_articles(
        &self,
        actor: Option<&AuthenticatedUser>,
        query: SearchArticlesQuery,
    ) -> ApplicationResult<CursorPage<ArticleDto>> {
        let trimmed = query.query.trim();
        if trimmed.is_empty() {
            return self
                .list_articles(
                    actor,
                    ListArticlesQuery {
                        include_drafts: query.include_drafts,
                        limit: query.limit,
                        cursor: query.cursor,
                    },
                )
                .await;
        }

        let (include_drafts, limit) =
            self.normalize_listing(actor, query.include_drafts, query.limit)?;
        let cursor = self.decode_cursor(query.cursor.as_deref())?;

        let (records, next_cursor) = self
            .read_repo
            .list_page(include_drafts, limit, cursor, Some(trimmed))
            .await?;

        let items = records.into_iter().map(Into::into).collect();
        Ok(CursorPage::new(
            items,
            next_cursor.map(|cursor| cursor.encode()),
        ))
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

    pub async fn list_revisions(
        &self,
        actor: &AuthenticatedUser,
        query: ListArticleRevisionsQuery,
    ) -> ApplicationResult<Vec<ArticleRevisionDto>> {
        let article_id = ArticleId::new(query.article_id)?;
        let article = self
            .read_repo
            .find_by_id(article_id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let spec = CanUpdateArticleSpec::new(&actor.capabilities, &article, actor.id);
        if !spec.is_satisfied() {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to view revisions",
            ));
        }

        let revisions = self.revision_repo.list_by_article(article_id).await?;

        Ok(revisions.into_iter().map(Into::into).collect())
    }

    fn normalize_listing(
        &self,
        actor: Option<&AuthenticatedUser>,
        include_drafts: bool,
        limit: u32,
    ) -> ApplicationResult<(bool, u32)> {
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

        const DEFAULT_LIMIT: u32 = 20;
        const MAX_LIMIT: u32 = 100;

        let limit = if limit == 0 {
            DEFAULT_LIMIT
        } else {
            limit.min(MAX_LIMIT)
        };

        Ok((include_drafts, limit))
    }

    fn decode_cursor(&self, token: Option<&str>) -> ApplicationResult<Option<ArticleListCursor>> {
        match token {
            Some(value) => match ArticleListCursor::decode(value) {
                Ok(cursor) => Ok(Some(cursor)),
                Err(DomainError::Validation(msg)) => Err(ApplicationError::validation(msg)),
                Err(other) => Err(ApplicationError::from(other)),
            },
            None => Ok(None),
        }
    }
}
