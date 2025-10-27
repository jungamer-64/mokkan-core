use super::ArticleQueryService;
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::ArticleSlug,
};

pub struct GetArticleBySlugQuery {
    pub slug: String,
}

impl ArticleQueryService {
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
}
