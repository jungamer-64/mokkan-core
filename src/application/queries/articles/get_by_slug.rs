use super::ArticleQueryService;
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{Article, ArticleSlug},
};

pub struct GetArticleBySlugQuery {
    pub slug: String,
}

impl ArticleQueryService {
    fn ensure_actor_can_view_unpublished(
        actor: Option<&AuthenticatedUser>,
        article: &Article,
    ) -> ApplicationResult<()> {
        if article.published {
            return Ok(());
        }

        let actor = actor.ok_or_else(|| ApplicationError::not_found("article not found"))?;
        if !actor.has_capability("articles", "view:drafts") && actor.id != article.author_id {
            return Err(ApplicationError::not_found("article not found"));
        }

        Ok(())
    }

    /// Load an article by slug, including draft visibility checks.
    ///
    /// # Errors
    ///
    /// Returns an error if the slug is invalid, the article is missing, the
    /// caller cannot view the draft, or the repository lookup fails.
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

        Self::ensure_actor_can_view_unpublished(actor, &article)?;

        Ok(article.into())
    }
}
