// src/presentation/http/controllers/auth.rs
use crate::application::{
    commands::users::{
        ChangePasswordCommand, LoginUserCommand, RefreshTokenCommand, RegisterUserCommand,
        UpdateUserCommand, GrantRoleCommand, RevokeRoleCommand,
    },
    dto::{AuthTokenDto, UserDto, UserProfileDto},
    queries::users::ListUsersQuery,
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::{Authenticated, MaybeAuthenticated};
use crate::presentation::http::openapi::{StatusResponse, UserListResponse};
use crate::presentation::http::state::HttpState;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};
use chrono::{Utc, TimeZone};
use serde_json::Value as JsonValue;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use crate::presentation::http::controllers::user_requests::{
    RegisterRequest, LoginRequest, RefreshTokenRequest, LoginResponse,
    ListUsersParams, UpdateUserRequest, ChangePasswordRequest, GrantRoleRequest,
};

#[utoipa::path(
    post,
    path = "/api/v1/auth/register",
    request_body = RegisterRequest,
    responses(
        (status = 200, description = "User registered.", body = UserDto),
        (status = 400, description = "Validation failed.", body = crate::presentation::http::error::ErrorResponse),
        (status = 409, description = "Username already exists.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security([]),
    tag = "Auth"
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful.", body = LoginResponse),
        (status = 401, description = "Invalid credentials.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security([]),
    tag = "Auth"
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Token refreshed.", body = AuthTokenDto),
        (status = 400, description = "Token not eligible for refresh.", body = crate::presentation::http::error::ErrorResponse),
        (status = 401, description = "Invalid token.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security([]),
    tag = "Auth"
)]
pub async fn refresh_token(
    Extension(state): Extension<HttpState>,
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
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
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

#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    responses(
        (status = 200, description = "List of sessions for the current user", body = [crate::application::dto::SessionInfoDto]),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn list_sessions(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
) -> HttpResult<Json<Vec<crate::application::dto::SessionInfoDto>>> {
    let store = state.services.session_revocation_store();
    let infos = store
        .list_sessions_for_user_with_meta(user.id.into())
        .await
        .into_http()?;

    let dtos: Vec<crate::application::dto::SessionInfoDto> = infos
        .into_iter()
        .map(|si| {
            let created = if si.created_at_unix > 0 {
                // Use the TimeZone::timestamp_opt API which returns a LocalResult.
                // Prefer `.single()` to get an Option<DateTime<Utc>> and fall back to now if invalid.
                Utc.timestamp_opt(si.created_at_unix, 0)
                    .single()
                    .unwrap_or_else(|| Utc::now())
            } else {
                Utc::now()
            };

            crate::application::dto::SessionInfoDto {
                session_id: si.session_id,
                user_agent: si.user_agent,
                ip_address: si.ip_address,
                created_at: created,
                revoked: si.revoked,
            }
        })
        .collect();

    Ok(Json(dtos))
}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/sessions/{id}",
    params(("id" = String, Path, description = "Session identifier")),
    responses(
        (status = 200, description = "Session revoked.", body = crate::presentation::http::openapi::StatusResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn revoke_session(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<String>,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    let store = state.services.session_revocation_store();

    // Allow owners to revoke their own sessions, or admins with users:update capability
    let is_owner = {
        let sessions = store.list_sessions_for_user(user.id.into()).await.into_http()?;
        sessions.contains(&id)
    };

    if !is_owner && !user.has_capability("users", "update") {
        return Err(crate::presentation::http::error::HttpError::from_error(
            crate::application::error::ApplicationError::forbidden("not authorized to revoke this session"),
        ));
    }

    // Revoke the session and remove metadata/association
    store.revoke(&id).await.into_http()?;

    // If metadata contains owner user_id, remove association there as well.
    if let Some(meta) = store.get_session_metadata(&id).await.into_http()? {
        if meta.user_id != 0 {
            let _ = store.remove_session_for_user(meta.user_id, &id).await;
        }
    }
    let _ = store.delete_session_metadata(&id).await;

    Ok(Json(crate::presentation::http::openapi::StatusResponse { status: "session_revoked".into() }))
}

#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(ListUsersParams),
    responses(
        (status = 200, description = "List of users.", body = UserListResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn list_users(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Query(params): Query<ListUsersParams>,
) -> HttpResult<Json<UserListResponse>> {
    let page = state
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
        .into_http()?;

    Ok(Json(UserListResponse::from(page)))
}

#[utoipa::path(
    patch,
    path = "/api/v1/users/{id}",
    params(
        ("id" = i64, Path, description = "User identifier")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated.", body = UserDto),
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ErrorResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
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

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/change-password",
    params(
        ("id" = i64, Path, description = "User identifier")
    ),
    request_body = ChangePasswordRequest,
    responses(
        (status = 200, description = "Password changed.", body = StatusResponse),
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ErrorResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn change_password(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<ChangePasswordRequest>,
) -> HttpResult<Json<StatusResponse>> {
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

    Ok(Json(StatusResponse {
        status: "password_changed".into(),
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/grant-role",
    params(
        ("id" = i64, Path, description = "User identifier")
    ),
    request_body = GrantRoleRequest,
    responses(
        (status = 200, description = "Role granted.", body = UserDto),
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ErrorResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn grant_role(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<GrantRoleRequest>,
) -> HttpResult<Json<UserDto>> {
    let command = GrantRoleCommand { user_id: id, role: payload.role };

    state
        .services
        .user_commands
        .grant_role(&user, command)
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/users/{id}/revoke-role",
    params(
        ("id" = i64, Path, description = "User identifier")
    ),
    responses(
        (status = 200, description = "Role revoked.", body = UserDto),
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ErrorResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ErrorResponse),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
pub async fn revoke_role(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
) -> HttpResult<Json<UserDto>> {
    let command = RevokeRoleCommand { user_id: id };

    state
        .services
        .user_commands
        .revoke_role(&user, command)
        .await
        .into_http()
        .map(Json)
}

// JWKS-like public keys endpoint. Returns the public key material used to verify tokens.
pub async fn keys(
    Extension(state): Extension<HttpState>,
) -> HttpResult<Json<JsonValue>> {
    state
        .services
        .token_manager()
        .public_jwk()
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out (session revoked).", body = crate::presentation::http::openapi::StatusResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ErrorResponse),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ErrorResponse)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
pub async fn logout(
    Extension(state): Extension<HttpState>,
    Authenticated(user): Authenticated,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    if let Some(session_id) = &user.session_id {
        state
            .services
            .session_revocation_store()
            .revoke(session_id)
            .await
            .into_http()?;

        Ok(Json(crate::presentation::http::openapi::StatusResponse {
            status: "logged_out".into(),
        }))
    } else {
        Err(crate::presentation::http::error::HttpError::from_error(
            crate::application::error::ApplicationError::validation("token is not session-based"),
        ))
    }
}
