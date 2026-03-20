use super::ArticleQueryService;
use crate::{
    application::{
        ArticleRevisionDto, AuthenticatedUser,
        error::{AppError, AppResult},
    },
    domain::{
        ArticleId,
        article::specifications::{ArticleSpecification, CanUpdateArticleSpec},
    },
};

pub struct ListArticleRevisionsQuery {
    pub article_id: i64,
}

impl ArticleQueryService {
    /// List revision history for an article.
    ///
    /// # Errors
    ///
    /// Returns an error if the article id is invalid, the article is missing,
    /// the actor lacks access, or repository reads fail.
    pub async fn list_revisions(
        &self,
        actor: &AuthenticatedUser,
        query: ListArticleRevisionsQuery,
    ) -> AppResult<Vec<ArticleRevisionDto>> {
        let article_id = ArticleId::new(query.article_id)?;
        let article = self
            .read_repo
            .find_by_id(article_id)
            .await?
            .ok_or_else(|| AppError::not_found("article not found"))?;

        let spec = CanUpdateArticleSpec::new(&actor.capabilities, &article, actor.id);
        if !spec.is_satisfied() {
            return Err(AppError::forbidden(
                "insufficient privileges to view revisions",
            ));
        }

        let revisions = self.revision_repo.list_by_article(article_id).await?;

        Ok(revisions.into_iter().map(Into::into).collect())
    }
}
