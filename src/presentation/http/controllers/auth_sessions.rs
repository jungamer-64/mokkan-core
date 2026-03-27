// src/presentation/http/controllers/auth_sessions.rs
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::Authenticated;
use crate::presentation::http::state::HttpContext;
use axum::{Extension, Json, extract::Path};

#[utoipa::path(
    get,
    path = "/api/v1/auth/sessions",
    responses(
        (status = 200, description = "List of sessions for the current user", body = [crate::application::SessionInfoDto]),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
/// List the current user's active and revoked sessions.
///
/// # Errors
///
/// Returns an error if authentication fails or session metadata lookup fails.
pub async fn list_sessions(
    Extension(state): Extension<HttpContext>,
    Authenticated(user): Authenticated,
) -> HttpResult<Json<Vec<crate::application::SessionInfoDto>>> {
    state
        .services
        .sessions
        .list_sessions(crate::application::services::ListSessionsRequest {
            user_id: user.id.into(),
        })
        .await
        .into_http()
        .map(Json)
}

#[utoipa::path(
    delete,
    path = "/api/v1/auth/sessions/{id}",
    params(("id" = String, Path, description = "Session identifier")),
    responses(
        (status = 200, description = "Session revoked.", body = crate::presentation::http::openapi::StatusResponse),
        (status = 401, description = "Unauthorized.", body = crate::presentation::http::error::ResponsePayload),
        (status = 403, description = "Forbidden.", body = crate::presentation::http::error::ResponsePayload),
        (status = 500, description = "Unexpected server error.", body = crate::presentation::http::error::ResponsePayload)
    ),
    security(("bearerAuth" = [])),
    tag = "Auth"
)]
/// Revoke a session by id.
///
/// # Errors
///
/// Returns an error if authentication fails, the caller is not allowed to
/// revoke the session, or session metadata and revocation operations fail.
pub async fn revoke_session(
    Extension(state): Extension<HttpContext>,
    Authenticated(user): Authenticated,
    Path(id): Path<String>,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    state
        .services
        .sessions
        .revoke_session(
            &user,
            crate::application::services::RevokeSessionRequest { session_id: id },
        )
        .await
        .into_http()?;

    Ok(Json(crate::presentation::http::openapi::StatusResponse {
        status: "session_revoked".into(),
    }))
}
