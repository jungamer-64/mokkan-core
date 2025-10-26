// src/presentation/http/controllers/auth_sessions.rs
use crate::presentation::http::state::HttpState;
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::Authenticated;
use axum::{Extension, Json, extract::Path};
use chrono::{Utc, TimeZone};

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

    let is_owner = {
        let sessions = store.list_sessions_for_user(user.id.into()).await.into_http()?;
        sessions.contains(&id)
    };

    if !is_owner && !user.has_capability("users", "update") {
        return Err(crate::presentation::http::error::HttpError::from_error(
            crate::application::error::ApplicationError::forbidden("not authorized to revoke this session"),
        ));
    }

    store.revoke(&id).await.into_http()?;

    if let Some(meta) = store.get_session_metadata(&id).await.into_http()? {
        if meta.user_id != 0 {
            let _ = store.remove_session_for_user(meta.user_id, &id).await;
        }
    }
    let _ = store.delete_session_metadata(&id).await;

    Ok(Json(crate::presentation::http::openapi::StatusResponse { status: "session_revoked".into() }))
}
