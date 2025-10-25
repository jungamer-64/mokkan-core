// tests/support/builders.rs
use chrono::Utc;

use mokkan_core::domain::article::*;
use mokkan_core::domain::user::UserId;

pub struct ArticleBuilder {
    id: i64,
    title: String,
    slug: String,
    body: String,
    published: bool,
    author_id: i64,
}

impl ArticleBuilder {
    pub fn new() -> Self {
        Self {
            id: 1,
            title: "Test Article".into(),
            slug: "test-article".into(),
            body: "Test body".into(),
            published: false,
            author_id: 1,
        }
    }

    pub fn id(mut self, id: i64) -> Self {
        self.id = id;
        self
    }

    pub fn published(mut self) -> Self {
        self.published = true;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn build(self) -> Article {
        Article {
            id: ArticleId::new(self.id).unwrap(),
            title: ArticleTitle::new(self.title).unwrap(),
            slug: ArticleSlug::new(self.slug).unwrap(),
            body: ArticleBody::new(self.body).unwrap(),
            published: self.published,
            published_at: if self.published { Some(Utc::now()) } else { None },
            author_id: UserId::new(self.author_id).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
