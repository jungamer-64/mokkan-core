use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
        ports::{time::Clock, util::SlugGenerator},
    },
    domain::article::{
        ArticleBody, ArticleId, ArticleSlug, ArticleTitle, ArticleUpdate, ArticleWriteRepository,
        NewArticle,
    },
};
use chrono::Utc;
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
    read_repo: Arc<dyn crate::domain::article::ArticleReadRepository>,
    slugger: Arc<dyn SlugGenerator>,
    clock: Arc<dyn Clock>,
}

impl ArticleCommandService {
    pub fn new(
        write_repo: Arc<dyn ArticleWriteRepository>,
        read_repo: Arc<dyn crate::domain::article::ArticleReadRepository>,
        slugger: Arc<dyn SlugGenerator>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            write_repo,
            read_repo,
            slugger,
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

        let slug = self.generate_unique_slug(title.as_str(), None).await?;

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

        let can_update_any = actor.has_capability("articles", "update:any");
        let can_update_own =
            actor.has_capability("articles", "update:own") && existing.author_id == actor.id;

        if !(can_update_any || can_update_own) {
            return Err(ApplicationError::forbidden(
                "insufficient privileges to update article",
            ));
        }

        let mut update = ArticleUpdate::new(id, self.clock.now());

        if let Some(title) = command.title {
            let title = ArticleTitle::new(title)?;
            let slug = self
                .generate_unique_slug(title.as_str(), Some(existing.id))
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

        let can_delete_any = actor.has_capability("articles", "delete:any");
        let can_delete_own =
            actor.has_capability("articles", "delete:own") && existing.author_id == actor.id;

        if !(can_delete_any || can_delete_own) {
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

    async fn generate_unique_slug(
        &self,
        title: &str,
        ignore_id: Option<ArticleId>,
    ) -> ApplicationResult<ArticleSlug> {
        let base = self.slugger.slugify(title);
        let base_slug = if base.is_empty() {
            format!("article-{}", Utc::now().timestamp())
        } else {
            base
        };

        let mut candidate = base_slug.clone();
        let mut counter = 1u64;

        loop {
            let slug = ArticleSlug::new(candidate.clone())?;
            if let Some(existing) = self.read_repo.find_by_slug(&slug).await? {
                if ignore_id.map(|id| id == existing.id).unwrap_or(false) {
                    return Ok(slug);
                }
                candidate = format!("{}-{}", base_slug, counter);
                counter += 1;
                continue;
            } else {
                return Ok(slug);
            }
        }
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
