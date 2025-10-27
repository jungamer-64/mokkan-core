use super::{ArticleCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::{ApplicationError, ApplicationResult},
    },
    domain::article::{
        Article, ArticleBody, ArticleId, ArticleTitle, ArticleUpdate,
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

        let title_opt = title.map(ArticleTitle::new).transpose()?;
        let body_opt = body.map(ArticleBody::new).transpose()?;

        update = self
            .apply_content_updates(&mut article, title_opt, body_opt, update)
            .await?;

        if let Some(publish_flag) = publish {
            update = self.apply_publish_update(actor, &mut article, publish_flag, update)?;
        }

        let updated = self.write_repo.update(update).await?;
        self.revision_repo.append(&updated, Some(actor.id)).await?;
        Ok(updated.into())
    }

    async fn apply_content_updates(
        &self,
        article: &mut Article,
        title_opt: Option<ArticleTitle>,
        body_opt: Option<ArticleBody>,
        mut update: ArticleUpdate,
    ) -> ApplicationResult<ArticleUpdate> {
        if title_opt.is_none() && body_opt.is_none() {
            return Ok(update);
        }

        let now = self.clock.now();
        let new_title = title_opt.clone().unwrap_or_else(|| article.title.clone());
        let new_body = body_opt.clone().unwrap_or_else(|| article.body.clone());
        article.set_content(new_title.clone(), new_body.clone(), now)?;
        update = update.with_title(new_title.clone()).with_body(new_body.clone());
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

        Ok(update)
    }

    fn apply_publish_update(
        &self,
        actor: &AuthenticatedUser,
        article: &mut Article,
        publish_flag: bool,
        mut update: ArticleUpdate,
    ) -> ApplicationResult<ArticleUpdate> {
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

        Ok(update)
    }
}
