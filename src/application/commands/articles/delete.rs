// src/application/commands/articles/delete.rs
use super::ArticleCommandService;
use crate::{
    application::{
        AuthenticatedUser,
        error::{AppError, AppResult},
    },
    domain::{
        ArticleId,
        article::specifications::{ArticleSpecification, CanDeleteArticleSpec},
    },
};

pub struct DeleteArticleCommand {
    pub id: i64,
}

impl ArticleCommandService {
    /// Delete an existing article.
    ///
    /// # Errors
    ///
    /// Returns an error if the id is invalid, the article is missing, the
    /// actor is not allowed to delete it, or repository operations fail.
    pub async fn delete_article(
        &self,
        actor: &AuthenticatedUser,
        command: DeleteArticleCommand,
    ) -> AppResult<()> {
        let id = ArticleId::new(command.id)?;
        let article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("article not found"))?;

        let delete_spec = CanDeleteArticleSpec::new(&actor.capabilities, &article, actor.id);

        if !delete_spec.is_satisfied() {
            return Err(AppError::forbidden(
                "insufficient privileges to delete article",
            ));
        }

        self.revision_repo.append(&article, Some(actor.id)).await?;

        self.write_repo.delete(id).await?;
        Ok(())
    }
}
