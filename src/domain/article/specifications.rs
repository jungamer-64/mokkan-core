use std::collections::HashSet;

use crate::domain::article::entity::Article;
use crate::domain::user::value_objects::{Capability, UserId};

pub trait ArticleSpecification {
    fn is_satisfied(&self) -> bool;
}

pub struct CanUpdateArticleSpec<'a> {
    capabilities: &'a HashSet<Capability>,
    article: &'a Article,
    user_id: UserId,
}

impl<'a> CanUpdateArticleSpec<'a> {
    pub fn new(
        capabilities: &'a HashSet<Capability>,
        article: &'a Article,
        user_id: UserId,
    ) -> Self {
        Self {
            capabilities,
            article,
            user_id,
        }
    }

    fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
    }
}

impl ArticleSpecification for CanUpdateArticleSpec<'_> {
    fn is_satisfied(&self) -> bool {
        self.has_capability("articles", "update:any")
            || (self.has_capability("articles", "update:own")
                && self.article.author_id == self.user_id)
    }
}

pub struct CanDeleteArticleSpec<'a> {
    capabilities: &'a HashSet<Capability>,
    article: &'a Article,
    user_id: UserId,
}

impl<'a> CanDeleteArticleSpec<'a> {
    pub fn new(
        capabilities: &'a HashSet<Capability>,
        article: &'a Article,
        user_id: UserId,
    ) -> Self {
        Self {
            capabilities,
            article,
            user_id,
        }
    }

    fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
    }
}

impl ArticleSpecification for CanDeleteArticleSpec<'_> {
    fn is_satisfied(&self) -> bool {
        self.has_capability("articles", "delete:any")
            || (self.has_capability("articles", "delete:own")
                && self.article.author_id == self.user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::article::entity::Article;
    use crate::domain::article::value_objects::{
        ArticleBody, ArticleId, ArticleSlug, ArticleTitle,
    };
    use crate::domain::user::value_objects::{Capability, UserId};
    use chrono::Utc;
    use std::collections::HashSet;

    fn article(author_id: i64) -> Article {
        Article {
            id: ArticleId::new(1).unwrap(),
            title: ArticleTitle::new("title").unwrap(),
            slug: ArticleSlug::new("title").unwrap(),
            body: ArticleBody::new("body").unwrap(),
            published: false,
            author_id: UserId::new(author_id).unwrap(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn update_spec_allows_admin_capability() {
        let mut caps = HashSet::new();
        caps.insert(Capability::new("articles", "update:any"));
        let article = article(1);
        let spec = CanUpdateArticleSpec::new(&caps, &article, UserId::new(2).unwrap());
        assert!(spec.is_satisfied());
    }

    #[test]
    fn update_spec_denies_without_capability() {
        let caps = HashSet::new();
        let article = article(1);
        let spec = CanUpdateArticleSpec::new(&caps, &article, UserId::new(2).unwrap());
        assert!(!spec.is_satisfied());
    }

    #[test]
    fn delete_spec_allows_owner_with_capability() {
        let mut caps = HashSet::new();
        caps.insert(Capability::new("articles", "delete:own"));
        let user_id = UserId::new(1).unwrap();
        let article = article(1);
        let spec = CanDeleteArticleSpec::new(&caps, &article, user_id);
        assert!(spec.is_satisfied());
    }
}
