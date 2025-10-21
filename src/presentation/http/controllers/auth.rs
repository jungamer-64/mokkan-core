// src/presentation/http/controllers/auth.rs
use crate::application::{
    commands::users::{
        ChangePasswordCommand, LoginUserCommand, RegisterUserCommand, UpdateUserCommand,
    },
    dto::{AuthTokenDto, CursorPage, UserDto, UserProfileDto},
    queries::users::ListUsersQuery,
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::state::HttpState;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub role: Option<crate::domain::user::Role>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: AuthTokenDto,
    pub user: UserDto,
}

fn default_limit() -> u32 {
    20
}

#[derive(Debug, Deserialize)]
pub struct ListUsersParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub q: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub is_active: Option<bool>,
    pub role: Option<crate::domain::user::Role>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn register(
    Extension(state): Extension<HttpState>,
    actor: MaybeAuthenticated,
    Json(payload): Json<RegisterRequest>,
) -> HttpResult<Json<UserDto>> {
    let command = RegisterUserCommand {
        username: payload.username,
        password: payload.password,
        role: payload.role,
    };

    state
        .services
        .user_commands
        .register(actor.0.as_ref(), command)
        .await
        .into_http()
        .map(Json)
}

pub async fn login(
    Extension(state): Extension<HttpState>,
    Json(payload): Json<LoginRequest>,
) -> HttpResult<Json<LoginResponse>> {
    let command = LoginUserCommand {
        username: payload.username,
        password: payload.password,
    };

    let result = state
        .services
        .user_commands
        .login(command)
        .await
        .into_http()?;

    Ok(Json(LoginResponse {
        token: result.token,
        user: result.user,
    }))
}

pub async fn profile(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
) -> HttpResult<Json<UserProfileDto>> {
    state
        .services
        .user_queries
        .get_profile(&user)
        .await
        .into_http()
        .map(Json)
}

pub async fn list_users(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Query(params): Query<ListUsersParams>,
) -> HttpResult<Json<CursorPage<UserDto>>> {
    state
        .services
        .user_queries
        .list_users(
            &user,
            ListUsersQuery {
                limit: params.limit,
                cursor: params.cursor,
                q: params.q,
            },
        )
        .await
        .into_http()
        .map(Json)
}

pub async fn update_user(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> HttpResult<Json<UserDto>> {
    let command = UpdateUserCommand {
        user_id: id,
        is_active: payload.is_active,
        role: payload.role,
    };

    state
        .services
        .user_commands
        .update_user(&user, command)
        .await
        .into_http()
        .map(Json)
}

pub async fn change_password(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<ChangePasswordRequest>,
) -> HttpResult<Json<serde_json::Value>> {
    let command = ChangePasswordCommand {
        user_id: id,
        current_password: payload.current_password,
        new_password: payload.new_password,
    };

    state
        .services
        .user_commands
        .change_password(&user, command)
        .await
        .into_http()?;

    Ok(Json(serde_json::json!({ "status": "password_changed" })))
}
