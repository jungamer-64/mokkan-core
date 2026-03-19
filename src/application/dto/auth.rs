use crate::domain::user::{Capability, Role, UserId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

use super::serde_time;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TokenDto {
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

#[derive(Debug, Clone)]
pub struct UserIdentity {
    pub id: UserId,
    pub username: String,
    pub role: Role,
    pub capabilities: HashSet<Capability>,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub session_id: Option<String>,
    pub token_version: Option<u32>,
}

impl UserIdentity {
    #[must_use]
    pub fn has_capability(&self, resource: &str, action: &str) -> bool {
        self.capabilities
            .iter()
            .any(|cap| cap.matches(resource, action))
    }
}

#[derive(Debug, Clone)]
pub struct Subject {
    pub user_id: UserId,
    pub username: String,
    pub role: Role,
    pub capabilities: HashSet<Capability>,
    pub session_id: Option<String>,
    pub token_version: Option<u32>,
}

impl Subject {
    #[must_use]
    pub fn from_authenticated(auth: &UserIdentity) -> Self {
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
