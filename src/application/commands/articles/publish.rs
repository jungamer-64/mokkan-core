// src/application/commands/articles/publish.rs
use super::{ArticleCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{ArticleId, ArticleUpdate},
};

pub struct SetPublishStateCommand {
    pub id: i64,
    pub publish: bool,
}

impl ArticleCommandService {
    pub async fn set_publish_state(
        &self,
        actor: &AuthenticatedUser,
        command: SetPublishStateCommand,
    ) -> ApplicationResult<ArticleDto> {
        ensure_capability(actor, "articles", "publish")?;
        let id = ArticleId::new(command.id)?;
        let original_updated_at;
        let mut article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;
        original_updated_at = article.updated_at;

        if article.published == command.publish {
            return Ok(article.into());
        }

        let now = self.clock.now();
        if command.publish {
            article.publish(now);
        } else {
            article.unpublish(now);
        }

        let mut update = ArticleUpdate::new(id, original_updated_at)
            .with_publish_state(article.published, article.published_at);
        update.set_updated_at(article.updated_at);
        let updated = self.write_repo.update(update).await?;
        self.revision_repo.append(&updated, Some(actor.id)).await?;
        Ok(updated.into())
    }
}
