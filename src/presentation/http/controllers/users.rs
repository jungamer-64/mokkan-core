use crate::application::{
    UserDto,
    commands::users::{
        ChangePasswordCommand, GrantRoleCommand, RevokeRoleCommand, UpdateUserCommand,
    },
    queries::users::ListUsersQuery,
};
use crate::presentation::http::controllers::user_requests::{
    ChangePasswordRequest, GrantRoleRequest, ListUsersParams, UpdateUserRequest,
};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::Authenticated;
use crate::presentation::http::openapi::{StatusResponse, UserListResponse};
use crate::presentation::http::state::HttpContext;
use axum::{
    Extension, Json,
    extract::{Path, Query},
};

#[utoipa::path(
    get,
    path = "/api/v1/users",
    params(ListUsersParams),
    responses(
        (status = 200, description = "List of users.", body = UserListResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
/// List users for an authorized caller.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller lacks permission, the
/// cursor is invalid, or the user query fails.
pub async fn list_users(
    Extension(state): Extension<HttpContext>,
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
/// Update a user's role or active state.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller lacks permission, the
/// payload is invalid, or the update command fails.
pub async fn update_user(
    Extension(state): Extension<HttpContext>,
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
/// Change a user's password.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller lacks permission, the
/// payload is invalid, or the password update fails.
pub async fn change_password(
    Extension(state): Extension<HttpContext>,
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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
/// Grant a role to a user.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller lacks permission, the
/// payload is invalid, or the command fails.
pub async fn grant_role(
    Extension(state): Extension<HttpContext>,
    Authenticated(user): Authenticated,
    Path(id): Path<i64>,
    Json(payload): Json<GrantRoleRequest>,
) -> HttpResult<Json<UserDto>> {
    let command = GrantRoleCommand {
        user_id: id,
        role: payload.role,
    };

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
        (status = 400, description = "Invalid input.", body = crate::presentation::http::error::ResponsePayload),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 404, description = "User not found.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Users"
)]
/// Revoke an elevated role from a user.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller lacks permission, or
/// the command fails.
pub async fn revoke_role(
    Extension(state): Extension<HttpContext>,
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
