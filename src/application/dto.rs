use crate::domain::{
    article::Article,
    user::{Capability, Role, User, UserId},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct UserDto {
    pub id: i64,
    pub username: String,
    pub role: Role,
    #[serde(with = "serde_time")]
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.into(),
            username: user.username.to_string(),
            role: user.role,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ArticleDto {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub published: bool,
    pub author_id: i64,
    #[serde(with = "serde_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "serde_time")]
    pub updated_at: DateTime<Utc>,
}

impl From<Article> for ArticleDto {
    fn from(article: Article) -> Self {
        Self {
            id: article.id.into(),
            title: article.title.to_string(),
            slug: article.slug.to_string(),
            body: article.body.to_string(),
            published: article.published,
            author_id: article.author_id.into(),
            created_at: article.created_at,
            updated_at: article.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuthTokenDto {
    pub token: String,
    #[serde(with = "serde_time")]
    pub issued_at: DateTime<Utc>,
    #[serde(with = "serde_time")]
    pub expires_at: DateTime<Utc>,
    pub expires_in: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct UserProfileDto {
    pub user: UserDto,
    pub capabilities: Vec<CapabilityView>,
    #[serde(with = "serde_time")]
    pub expires_at: DateTime<Utc>,
    pub expires_in: i64,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: UserId,
    pub username: String,
    pub role: Role,
    pub capabilities: HashSet<Capability>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthenticatedUser {
    pub fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
    }
}

#[derive(Debug, Clone)]
pub struct TokenSubject {
    pub user_id: UserId,
    pub username: String,
    pub role: Role,
    pub capabilities: HashSet<Capability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityView {
    pub resource: String,
    pub action: String,
}

impl From<Capability> for CapabilityView {
    fn from(value: Capability) -> Self {
        Self {
            resource: value.resource,
            action: value.action,
        }
    }
}

impl UserProfileDto {
    pub fn from_parts(user: User, auth: &AuthenticatedUser) -> Self {
        let user_dto: UserDto = user.into();
        let mut capabilities: Vec<_> = auth
            .capabilities
            .iter()
            .cloned()
            .map(CapabilityView::from)
            .collect();
        capabilities.sort_by(|a, b| {
            a.resource
                .cmp(&b.resource)
                .then_with(|| a.action.cmp(&b.action))
        });
        let expires_in = auth
            .expires_at
            .signed_duration_since(Utc::now())
            .num_seconds()
            .max(0);

        Self {
            user: user_dto,
            capabilities,
            expires_at: auth.expires_at,
            expires_in,
        }
    }
}

pub mod serde_time {
    use chrono::{DateTime, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_rfc3339())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }
}
