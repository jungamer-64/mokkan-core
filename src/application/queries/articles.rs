use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{ArticleId, ArticleReadRepository, ArticleSlug},
};
use std::sync::Arc;

pub struct ListArticlesQuery {
    pub include_drafts: bool,
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
    ) -> ApplicationResult<Vec<ArticleDto>> {
        let include_drafts = if query.include_drafts {
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

        let records = self.read_repo.list(include_drafts).await?;
        Ok(records.into_iter().map(Into::into).collect())
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
}
