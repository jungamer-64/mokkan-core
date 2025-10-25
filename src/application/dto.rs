// src/application/dto.rs
use crate::domain::{
    article::{Article, ArticleRevision},
    user::{Capability, Role, User, UserId},
};
use crate::domain::audit::entity::AuditLog;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserDto {
    pub id: i64,
    pub username: String,
    pub role: Role,
    pub is_active: bool,
    #[serde(with = "serde_time")]
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        Self {
            id: user.id.into(),
            username: user.username.to_string(),
            role: user.role,
            is_active: user.is_active,
            created_at: user.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArticleDto {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub published: bool,
    #[serde(default, with = "serde_time::option")]
    pub published_at: Option<DateTime<Utc>>,
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
            title: article.title.into_inner(),
            slug: article.slug.into_inner(),
            body: article.body.into_inner(),
            published: article.published,
            published_at: article.published_at,
            author_id: article.author_id.into(),
            created_at: article.created_at,
            updated_at: article.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ArticleRevisionDto {
    pub version: i32,
    pub title: String,
    pub slug: String,
    pub body: String,
    pub published: bool,
    #[serde(default, with = "serde_time::option")]
    pub published_at: Option<DateTime<Utc>>,
    pub author_id: i64,
    #[serde(default)]
    pub edited_by: Option<i64>,
    #[serde(with = "serde_time")]
    pub recorded_at: DateTime<Utc>,
}

impl From<ArticleRevision> for ArticleRevisionDto {
    fn from(revision: ArticleRevision) -> Self {
        Self {
            version: revision.version,
            title: revision.title.into_inner(),
            slug: revision.slug.into_inner(),
            body: revision.body.into_inner(),
            published: revision.published,
            published_at: revision.published_at,
            author_id: revision.author_id.into(),
            edited_by: revision.edited_by.map(Into::into),
            recorded_at: revision.recorded_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuditLogDto {
    pub id: i64,
    pub user_id: Option<i64>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<i64>,
    pub details: Option<serde_json::Value>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl From<AuditLog> for AuditLogDto {
    fn from(a: AuditLog) -> Self {
        Self {
            id: a.id.unwrap_or_default(),
            user_id: a.user_id.map(Into::into),
            action: a.action,
            resource_type: a.resource_type,
            resource_id: a.resource_id,
            details: a.details,
            ip_address: a.ip_address,
            user_agent: a.user_agent,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(bound(
    serialize = "T: Serialize",
    deserialize = "T: serde::de::DeserializeOwned"
))]
pub struct CursorPage<T> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

impl<T> CursorPage<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>) -> Self {
        let has_more = next_cursor.is_some();
        Self {
            items,
            next_cursor,
            has_more,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthTokenDto {
    pub token: String,
    #[serde(with = "serde_time")]
    pub issued_at: DateTime<Utc>,
    #[serde(with = "serde_time")]
    pub expires_at: DateTime<Utc>,
    pub expires_in: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
    pub session_id: Option<String>,
    pub token_version: Option<u32>,
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
    pub session_id: Option<String>,
    pub token_version: Option<u32>,
}

impl TokenSubject {
    pub fn from_authenticated(auth: &AuthenticatedUser) -> Self {
        Self {
            user_id: auth.id,
            username: auth.username.clone(),
            role: auth.role,
            capabilities: auth.capabilities.clone(),
            session_id: auth.session_id.clone(),
            token_version: auth.token_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

    #[allow(dead_code)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    }

    pub mod option {
        use super::*;

        pub fn serialize<S>(value: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match value {
                Some(dt) => serializer.serialize_some(&dt.to_rfc3339()),
                None => serializer.serialize_none(),
            }
        }

        #[allow(dead_code)]
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
        where
            D: Deserializer<'de>,
        {
            let opt = Option::<String>::deserialize(deserializer)?;
            opt.map(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .map(|dt| dt.with_timezone(&Utc))
                    .map_err(serde::de::Error::custom)
            })
            .transpose()
        }
    }
}
