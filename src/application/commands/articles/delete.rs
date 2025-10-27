// src/application/commands/articles/delete.rs
use super::ArticleCommandService;
use crate::{
    application::{
        dto::AuthenticatedUser,
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{
        ArticleId,
        specifications::{ArticleSpecification, CanDeleteArticleSpec},
    },
};

pub struct DeleteArticleCommand {
    pub id: i64,
}

impl ArticleCommandService {
    pub async fn delete_article(
        &self,
        actor: &AuthenticatedUser,
        command: DeleteArticleCommand,
    ) -> ApplicationResult<()> {
        let id = ArticleId::new(command.id)?;
        let article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let delete_spec = CanDeleteArticleSpec::new(&actor.capabilities, &article, actor.id);

        if !delete_spec.is_satisfied() {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to delete article",
            ));
        }

        self.revision_repo.append(&article, Some(actor.id)).await?;

        self.write_repo.delete(id).await?;
        Ok(())
    }
}
