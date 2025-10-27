use super::ArticleQueryService;
use crate::{
    application::{
        dto::{ArticleRevisionDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{
        ArticleId,
        specifications::{ArticleSpecification, CanUpdateArticleSpec},
    },
};

pub struct ListArticleRevisionsQuery {
    pub article_id: i64,
}

impl ArticleQueryService {
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
}
