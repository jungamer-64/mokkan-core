use super::{ArticleCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{
        ArticleBody, ArticleId, ArticleTitle, ArticleUpdate,
        specifications::{ArticleSpecification, CanUpdateArticleSpec},
    },
};

pub struct UpdateArticleCommand {
    pub id: i64,
    pub title: Option<String>,
    pub body: Option<String>,
    pub publish: Option<bool>,
}

impl ArticleCommandService {
    #[allow(clippy::too_many_lines)]
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
}
