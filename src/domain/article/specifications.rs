use std::collections::HashSet;

use crate::domain::article::entity::Article;
use crate::domain::user::value_objects::{Capability, UserId};

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

    pub fn is_satisfied(&self) -> bool {
        self.has_capability("articles", "update:any")
            || (self.has_capability("articles", "update:own")
                && self.article.author_id == self.user_id)
    }

    fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
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

    pub fn is_satisfied(&self) -> bool {
        self.has_capability("articles", "delete:any")
            || (self.has_capability("articles", "delete:own")
                && self.article.author_id == self.user_id)
    }

    fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
    }
}
