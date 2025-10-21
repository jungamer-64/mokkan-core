use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
        ports::time::Clock,
    },
    domain::article::{
        services::ArticleSlugService,
        specifications::{CanDeleteArticleSpec, CanUpdateArticleSpec},
        ArticleBody, ArticleId, ArticleReadRepository, ArticleTitle, ArticleUpdate,
        ArticleWriteRepository, NewArticle,
    },
};
use std::sync::Arc;

pub struct CreateArticleCommand {
    pub title: String,
    pub body: String,
    pub publish: bool,
}

pub struct UpdateArticleCommand {
    pub id: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub publish: Option<bool>,
}

pub struct DeleteArticleCommand {
    pub id: i64,
}

pub struct SetPublishStateCommand {
    pub id: i64,
    pub publish: bool,
}

pub struct ArticleCommandService {
    write_repo: Arc<dyn ArticleWriteRepository>,
    read_repo: Arc<dyn ArticleReadRepository>,
    slug_service: Arc<ArticleSlugService>,
    clock: Arc<dyn Clock>,
}

impl ArticleCommandService {
    pub fn new(
        write_repo: Arc<dyn ArticleWriteRepository>,
        read_repo: Arc<dyn crate::domain::article::ArticleReadRepository>,
        slug_service: Arc<ArticleSlugService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            write_repo,
            read_repo,
            slug_service,
            clock,
        }
    }

    pub async fn create_article(
        &self,
        actor: &AuthenticatedUser,
        command: CreateArticleCommand,
    ) -> ApplicationResult<ArticleDto> {
        ensure_capability(actor, "articles", "create")?;

        let title = ArticleTitle::new(command.title)?;
        let body = ArticleBody::new(command.body)?;
        let now = self.clock.now();

        let slug = self
            .slug_service
            .generate_unique_slug(&title, None)
            .await?;

        let new_article = NewArticle {
            title,
            slug,
            body,
            published: command.publish,
            author_id: actor.id,
            created_at: now,
            updated_at: now,
        };

        let created = self.write_repo.insert(new_article).await?;
        Ok(created.into())
    }

    pub async fn update_article(
        &self,
        actor: &AuthenticatedUser,
        command: UpdateArticleCommand,
    ) -> ApplicationResult<ArticleDto> {
        let id = ArticleId::new(command.id)?;
        let existing = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let update_spec = CanUpdateArticleSpec::new(&actor.capabilities, &existing, actor.id);

        if !update_spec.is_satisfied() {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to update article",
            ));
        }

        let mut update = ArticleUpdate::new(id, self.clock.now());

        if let Some(title) = command.title {
            let title = ArticleTitle::new(title)?;
            let slug = self
                .slug_service
                .generate_unique_slug(&title, Some(existing.id))
                .await?;
            update = update.with_title(title).with_slug(slug);
        }

        if let Some(body) = command.body {
            let body = ArticleBody::new(body)?;
            update = update.with_body(body);
        }

        if let Some(publish) = command.publish {
            if publish != existing.published {
                ensure_capability(actor, "articles", "publish")?;
                update = update.with_published(publish);
            }
        }

        let updated = self.write_repo.update(update).await?;
        Ok(updated.into())
    }

    pub async fn delete_article(
        &self,
        actor: &AuthenticatedUser,
        command: DeleteArticleCommand,
    ) -> ApplicationResult<()> {
        let id = ArticleId::new(command.id)?;
        let existing = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let delete_spec = CanDeleteArticleSpec::new(&actor.capabilities, &existing, actor.id);

        if !delete_spec.is_satisfied() {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to delete article",
            ));
        }

        self.write_repo.delete(id).await?;
        Ok(())
    }

    pub async fn set_publish_state(
        &self,
        actor: &AuthenticatedUser,
        command: SetPublishStateCommand,
    ) -> ApplicationResult<ArticleDto> {
        ensure_capability(actor, "articles", "publish")?;
        let id = ArticleId::new(command.id)?;
        self.read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let update = ArticleUpdate::new(id, self.clock.now()).with_published(command.publish);
        let updated = self.write_repo.update(update).await?;
        Ok(updated.into())
    }

}

fn ensure_capability(
    actor: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> ApplicationResult<()> {
    if actor.has_capability(resource, action) {
        Ok(())
    } else {
        Err(ApplicationError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
