// src/presentation/http/controllers/auth.rs
use crate::application::{
    AuthTokenDto, UserDto, UserProfileDto,
    commands::users::{LoginUserCommand, RefreshTokenCommand, RegisterUserCommand},
};
use crate::presentation::http::controllers::user_requests::{
    LoginRequest, LoginResponse, RefreshTokenRequest, RegisterRequest,
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::state::HttpContext;
use axum::{Extension, Json};
use serde_json::Value as JsonValue;

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered.", body = UserDto),
        (status = 400, description = "Validation failed.", body = crate::presentation::http::error::ResponsePayload),
        (status = 409, description = "Username already exists.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security([]),
    tag = "Auth"
)]
/// Register a new user account.
///
/// # Errors
///
/// Returns an error if the payload is invalid, the username already exists,
/// or the registration command fails.
pub async fn register(
    Extension(state): Extension<HttpContext>,
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful.", body = LoginResponse),
        (status = 401, description = "Invalid credentials.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security([]),
    tag = "Auth"
)]
/// Log a user in and issue tokens.
///
/// # Errors
///
/// Returns an error if the credentials are invalid or token issuance fails.
pub async fn login(
    Extension(state): Extension<HttpContext>,
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed.", body = AuthTokenDto),
        (status = 400, description = "Token not eligible for refresh.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Invalid token.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security([]),
    tag = "Auth"
)]
/// Refresh a token pair from a refresh token.
///
/// # Errors
///
/// Returns an error if the refresh token is invalid, expired, revoked, or the
/// refresh command fails.
pub async fn refresh_token(
    Extension(state): Extension<HttpContext>,
    Json(payload): Json<RefreshTokenRequest>,
) -> HttpResult<Json<AuthTokenDto>> {
    let command = RefreshTokenCommand {
        token: payload.token,
    };

    state
        .services
        .user_commands
        .refresh_token(command)
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/me",
    responses(
        (status = 200, description = "Current user profile.", body = UserProfileDto),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
/// Return the current authenticated user's profile.
///
/// # Errors
///
/// Returns an error if authentication fails or the user record cannot be
/// loaded.
pub async fn profile(
    Extension(state): Extension<HttpContext>,
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

// Session endpoints are implemented in `auth_sessions.rs` (OpenAPI paths defined there)

// JWKS-like public keys endpoint. Returns the public key material used to verify tokens.
///
/// # Errors
///
/// Returns an error if the public key material cannot be rendered.
pub async fn keys(Extension(state): Extension<HttpContext>) -> HttpResult<Json<JsonValue>> {
    state.services.auth.public_jwk().await.into_http().map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out (session revoked).", body = crate::presentation::http::openapi::StatusResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
/// Revoke the current session-backed token.
///
/// # Errors
///
/// Returns an error if the token is not session-based or session revocation
/// fails.
pub async fn logout(
    Extension(state): Extension<HttpContext>,
    Authenticated(user): Authenticated,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    state.services.auth.logout(&user).await.into_http()?;

    Ok(Json(crate::presentation::http::openapi::StatusResponse {
        status: "logged_out".into(),
    }))
}
