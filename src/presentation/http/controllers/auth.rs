use crate::application::{
    commands::users::{LoginUserCommand, RegisterUserCommand},
    dto::{AuthTokenDto, UserDto, UserProfileDto},
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::state::HttpState;
use axum::{Extension, Json};
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
