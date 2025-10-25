// src/application/commands/articles.rs
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
        ports::time::Clock,
    },
    domain::article::{
        ArticleBody, ArticleId, ArticleReadRepository, ArticleRevisionRepository, ArticleTitle,
        ArticleUpdate, ArticleWriteRepository, NewArticle,
        services::ArticleSlugService,
        specifications::{ArticleSpecification, CanDeleteArticleSpec, CanUpdateArticleSpec},
    },
};
use std::sync::Arc;

pub struct CreateArticleCommand {
    pub title: String,
    pub body: String,
    pub publish: bool,
}

impl CreateArticleCommand {
    pub fn builder() -> CreateArticleCommandBuilder {
        CreateArticleCommandBuilder::default()
    }
}

#[derive(Default)]
pub struct CreateArticleCommandBuilder {
    title: Option<String>,
    body: Option<String>,
    publish: bool,
}

impl CreateArticleCommandBuilder {
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn publish(mut self, publish: bool) -> Self {
        self.publish = publish;
        self
    }

    pub fn build(self) -> Result<CreateArticleCommand, &'static str> {
        Ok(CreateArticleCommand {
            title: self.title.ok_or("title is required")?,
            body: self.body.ok_or("body is required")?,
            publish: self.publish,
        })
    }
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
    revision_repo: Arc<dyn ArticleRevisionRepository>,
    slug_service: Arc<ArticleSlugService>,
    clock: Arc<dyn Clock>,
}

impl ArticleCommandService {
    pub fn new(
        write_repo: Arc<dyn ArticleWriteRepository>,
        read_repo: Arc<dyn ArticleReadRepository>,
        revision_repo: Arc<dyn ArticleRevisionRepository>,
        slug_service: Arc<ArticleSlugService>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            write_repo,
            read_repo,
            revision_repo,
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

        let slug = self.slug_service.generate_unique_slug(&title, None).await?;

        let new_article = NewArticle {
            title,
            slug,
            body,
            published: command.publish,
            published_at: if command.publish { Some(now) } else { None },
            author_id: actor.id,
            created_at: now,
            updated_at: now,
        };

        let created = self.write_repo.insert(new_article).await?;
        self.revision_repo.append(&created, Some(actor.id)).await?;
        Ok(created.into())
    }

    pub async fn update_article(
        &self,
        actor: &AuthenticatedUser,
        command: UpdateArticleCommand,
    ) -> ApplicationResult<ArticleDto> {
        let id = ArticleId::new(command.id)?;
        let mut article = self
            .read_repo
            .find_by_id(id)
            .await?
            .ok_or_else(|| ApplicationError::not_found("article not found"))?;

        let update_spec = CanUpdateArticleSpec::new(&actor.capabilities, &article, actor.id);

        if !update_spec.is_satisfied() {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to update article",
            ));
        }

        let UpdateArticleCommand {
            id: _,
            title,
            body,
            publish,
        } = command;
        let original_updated_at = article.updated_at;
        let mut update = ArticleUpdate::new(id, original_updated_at);

        let title_opt = match title {
            Some(value) => Some(ArticleTitle::new(value)?),
            None => None,
        };
        let body_opt = match body {
            Some(value) => Some(ArticleBody::new(value)?),
            None => None,
        };

        if title_opt.is_some() || body_opt.is_some() {
            let now = self.clock.now();
            let new_title = title_opt.clone().unwrap_or_else(|| article.title.clone());
            let new_body = body_opt.clone().unwrap_or_else(|| article.body.clone());
            article.set_content(new_title.clone(), new_body.clone(), now)?;
            update = update.with_title(new_title).with_body(new_body);
            update.set_updated_at(article.updated_at);

            if let Some(title) = &title_opt {
                let slug = self
                    .slug_service
                    .generate_unique_slug(title, Some(article.id))
                    .await?;
                article.set_slug(slug.clone(), now);
                update = update.with_slug(slug);
                update.set_updated_at(article.updated_at);
            }
        }

        if let Some(publish_flag) = publish {
            if publish_flag != article.published {
                ensure_capability(actor, "articles", "publish")?;
                let now = self.clock.now();
                if publish_flag {
                    article.publish(now);
                } else {
                    article.unpublish(now);
                }
                update = update.with_publish_state(article.published, article.published_at);
                update.set_updated_at(article.updated_at);
            }
        }

        let updated = self.write_repo.update(update).await?;
        self.revision_repo.append(&updated, Some(actor.id)).await?;
        Ok(updated.into())
    }

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
