// src/domain/article/entity.rs
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
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Article {
    pub fn publish(&mut self, now: DateTime<Utc>) {
        self.published = true;
        self.published_at = Some(now);
        self.updated_at = now;
    }

    pub fn unpublish(&mut self, now: DateTime<Utc>) {
        self.published = false;
        self.published_at = None;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::article::value_objects::{ArticleBody, ArticleSlug, ArticleTitle};
    use chrono::Utc;

    fn sample_article() -> Article {
        Article {
            id: ArticleId::new(1).unwrap(),
            title: ArticleTitle::new("title").unwrap(),
            slug: ArticleSlug::new("title").unwrap(),
            body: ArticleBody::new("body").unwrap(),
            published: false,
            published_at: None,
            author_id: crate::domain::user::UserId::new(1).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn publish_sets_state() {
        let mut article = sample_article();
        let now = Utc::now();
        article.publish(now);
        assert!(article.published);
        assert_eq!(article.published_at, Some(now));
        assert_eq!(article.updated_at, now);
    }

    #[test]
    fn unpublish_sets_state() {
        let mut article = sample_article();
        let now = Utc::now();
        article.publish(now);
        let later = now + chrono::Duration::seconds(10);
        article.unpublish(later);
        assert!(!article.published);
        assert!(article.published_at.is_none());
        assert_eq!(article.updated_at, later);
    }

    #[test]
    fn set_content_updates_fields() {
        let mut article = sample_article();
        let now = Utc::now();
        let title = ArticleTitle::new("new title").unwrap();
        let body = ArticleBody::new("new body").unwrap();
        article
            .set_content(title.clone(), body.clone(), now)
            .unwrap();
        assert_eq!(article.title.as_str(), title.as_str());
        assert_eq!(article.body.as_str(), body.as_str());
        assert_eq!(article.updated_at, now);
    }
}

#[derive(Debug, Clone)]
pub struct NewArticle {
    pub title: ArticleTitle,
    pub slug: ArticleSlug,
    pub body: ArticleBody,
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: UserId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PublishStateUpdate {
    pub published: bool,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct ArticleUpdate {
    pub id: ArticleId,
    pub title: Option<ArticleTitle>,
    pub slug: Option<ArticleSlug>,
    pub body: Option<ArticleBody>,
    pub publish_state: Option<PublishStateUpdate>,
    pub original_updated_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ArticleUpdate {
    pub fn new(id: ArticleId, original_updated_at: DateTime<Utc>) -> Self {
        Self {
            id,
            title: None,
            slug: None,
            body: None,
            publish_state: None,
            original_updated_at,
            updated_at: original_updated_at,
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

    pub fn with_publish_state(
        mut self,
        published: bool,
        published_at: Option<DateTime<Utc>>,
    ) -> Self {
        self.publish_state = Some(PublishStateUpdate {
            published,
            published_at,
        });
        self
    }

    pub fn set_updated_at(&mut self, updated_at: DateTime<Utc>) {
        self.updated_at = updated_at;
    }
}
