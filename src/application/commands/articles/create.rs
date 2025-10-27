// src/application/commands/articles/create.rs
use super::{ArticleCommandService, capability::ensure_capability};
use crate::{
    application::{
        dto::{ArticleDto, AuthenticatedUser},
        error::ApplicationResult,
    },
    domain::article::{ArticleBody, ArticleTitle, NewArticle},
};

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

impl ArticleCommandService {
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
}
