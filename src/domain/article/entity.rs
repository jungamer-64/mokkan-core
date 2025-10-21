use crate::domain::article::value_objects::{ArticleBody, ArticleId, ArticleSlug, ArticleTitle};
use crate::domain::errors::DomainResult;
use crate::domain::user::UserId;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct Article {
    pub id: ArticleId,
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Article {
    pub fn publish(&mut self, now: DateTime<Utc>) {
        self.published = true;
        self.updated_at = now;
    }

    pub fn unpublish(&mut self, now: DateTime<Utc>) {
        self.published = false;
        self.updated_at = now;
    }

    pub fn set_slug(&mut self, slug: ArticleSlug, now: DateTime<Utc>) {
        self.slug = slug;
        self.updated_at = now;
    }

    pub fn set_content(
        &mut self,
        title: ArticleTitle,
        body: ArticleBody,
        now: DateTime<Utc>,
    ) -> DomainResult<()> {
        self.title = title;
        self.body = body;
        self.updated_at = now;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct NewArticle {
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ArticleUpdate {
    pub id: ArticleId,
    pub title: Option<ArticleTitle>,
    pub slug: Option<ArticleSlug>,
    pub body: Option<ArticleBody>,
    pub published: Option<bool>,
    pub updated_at: DateTime<Utc>,
}

impl ArticleUpdate {
    pub fn new(id: ArticleId, updated_at: DateTime<Utc>) -> Self {
        Self {
            id,
            title: None,
            slug: None,
            body: None,
            published: None,
            updated_at,
        }
    }

    pub fn with_title(mut self, title: ArticleTitle) -> Self {
        self.title = Some(title);
        self
    }

    pub fn with_slug(mut self, slug: ArticleSlug) -> Self {
        self.slug = Some(slug);
        self
    }

    pub fn with_body(mut self, body: ArticleBody) -> Self {
        self.body = Some(body);
        self
    }

    pub fn with_published(mut self, published: bool) -> Self {
        self.published = Some(published);
        self
    }
}
