use crate::domain::user::{Capability, Role, User};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use super::{auth::AuthenticatedUser, serde_time};

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfileDto {
    pub user: UserDto,
    pub capabilities: Vec<CapabilityView>,
    #[serde(with = "serde_time")]
    pub expires_at: DateTime<Utc>,
    pub expires_in: i64,
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
