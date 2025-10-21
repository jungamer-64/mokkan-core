use crate::domain::article::entity::Article;
use crate::domain::user::{value_objects::Capability, value_objects::Role};

pub trait CapabilitySpecification {
    fn is_satisfied_by(&self, capabilities: &[Capability]) -> bool;
}

pub struct HasCapability<'a> {
    pub resource: &'a str,
    pub action: &'a str,
}

impl CapabilitySpecification for HasCapability<'_> {
    fn is_satisfied_by(&self, capabilities: &[Capability]) -> bool {
        capabilities.iter().any(|cap| cap.matches(self.resource, self.action))
    }
}

pub struct CanUpdateArticle<'a> {
    pub role: Role,
    pub capabilities: &'a [Capability],
    pub article: &'a Article,
    pub user_id: crate::domain::user::value_objects::UserId,
}

impl CapabilitySpecification for CanUpdateArticle<'_> {
    fn is_satisfied_by(&self, _: &[Capability]) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches("articles", "update:any"))
            || (self
                .capabilities
                .iter()
                .any(|cap| cap.matches("articles", "update:own"))
                && self.article.author_id == self.user_id)
    }
}
