use crate::application::queries::audit::{AuditQueryService, ListAuditQuery};
use crate::application::dto::AuthenticatedUser;
use crate::presentation::http::state::HttpState;
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::Authenticated;
use crate::application::dto::AuditLogDto;
use crate::application::dto::CursorPage;
use axum::{Extension, Json, extract::{Path, Query}};

#[derive(Debug, serde::Deserialize)]
pub struct ListAuditParams {
    #[serde(default = "default_limit")]
    pub limit: u32,
    #[serde(default)]
    pub cursor: Option<String>,
}

fn default_limit() -> u32 { 20 }

pub async fn list_audit_logs(
    Extension(state): Extension<HttpState>,
    Authenticated(actor): Authenticated,
    Query(params): Query<ListAuditParams>,
) -> HttpResult<Json<CursorPage<AuditLogDto>>> {
    let service = AuditQueryService::new(state.services.audit_log_repo());
    let res = service
        .list_audit_logs(&actor, ListAuditQuery { limit: params.limit, cursor: params.cursor.clone() })
        .await
        .into_http()?;
    Ok(Json(res))
}

pub async fn list_audit_logs_by_user(
    Extension(state): Extension<HttpState>,
    Authenticated(actor): Authenticated,
    Path(user_id): Path<i64>,
    Query(params): Query<ListAuditParams>,
) -> HttpResult<Json<CursorPage<AuditLogDto>>> {
    let service = AuditQueryService::new(state.services.audit_log_repo());
    let res = service
        .list_by_user(&actor, user_id, ListAuditQuery { limit: params.limit, cursor: params.cursor.clone() })
        .await
        .into_http()?;
    Ok(Json(res))
}

pub async fn list_audit_logs_by_resource(
    Extension(state): Extension<HttpState>,
    Authenticated(actor): Authenticated,
    Path((resource_type, resource_id)): Path<(String, i64)>,
    Query(params): Query<ListAuditParams>,
) -> HttpResult<Json<CursorPage<AuditLogDto>>> {
    let service = AuditQueryService::new(state.services.audit_log_repo());
    let res = service
        .list_by_resource(&actor, &resource_type, resource_id, ListAuditQuery { limit: params.limit, cursor: params.cursor.clone() })
        .await
        .into_http()?;
    Ok(Json(res))
}
