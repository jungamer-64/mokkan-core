use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub role: Option<crate::domain::user::Role>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LoginResponse {
    pub token: crate::application::dto::AuthTokenDto,
    pub user: crate::application::dto::UserDto,
}

#[derive(Debug, Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ListUsersParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub is_active: Option<bool>,
    pub role: Option<crate::domain::user::Role>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub current_password: Option<String>,
    pub new_password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct GrantRoleRequest {
    pub role: crate::domain::user::Role,
}
