// src/presentation/http/controllers/auth_oidc.rs
use crate::presentation::http::state::HttpState;
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct IntrospectResponse {
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/introspect",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token introspection", body = IntrospectResponse),
    ),
    security([]),
    tag = "Auth"
)]
pub async fn introspect(
    Extension(state): Extension<HttpState>,
    Json(payload): Json<TokenRequest>,
) -> HttpResult<Json<IntrospectResponse>> {
    match state.services.token_manager().authenticate(&payload.token).await {
        Ok(user) => {
            let resp = IntrospectResponse {
                active: true,
                scope: Some("openid profile email".into()),
                username: Some(user.username.clone()),
                sub: Some(i64::from(user.id).to_string()),
                exp: Some(user.expires_at.timestamp()),
                iat: Some(user.issued_at.timestamp()),
                session_id: user.session_id.clone(),
            };
            Ok(Json(resp))
        }
        Err(_e) => Ok(Json(IntrospectResponse {
            active: false,
            scope: None,
            username: None,
            sub: None,
            exp: None,
            iat: None,
            session_id: None,
        })),
    }
}

#[utoipa::path(
    post,
    path = "/api/v1/auth/revoke",
    request_body = TokenRequest,
    responses(
        (status = 200, description = "Token revocation acknowledged", body = crate::presentation::http::openapi::StatusResponse),
    ),
    security([]),
    tag = "Auth"
)]
pub async fn revoke(
    Extension(state): Extension<HttpState>,
    Json(payload): Json<TokenRequest>,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    if let Ok(user) = state.services.token_manager().authenticate(&payload.token).await {
        if let Some(session_id) = user.session_id.as_ref() {
            state
                .services
                .session_revocation_store()
                .revoke(session_id)
                .await
                .into_http()?;
        }
    }

    Ok(Json(crate::presentation::http::openapi::StatusResponse { status: "revoked".into() }))
}

pub async fn authorize(
    Extension(_state): Extension<HttpState>,
) -> HttpResult<Json<JsonValue>> {
    Ok(Json(serde_json::json!({"message":"authorization endpoint not implemented"})))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    responses(
        (status = 200, description = "Authorization endpoint (placeholder).", body = crate::presentation::http::openapi::StatusResponse),
    ),
    security([]),
    tag = "Auth"
)]
pub async fn authorize_openapi_stub(
    Extension(_state): Extension<HttpState>,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    Ok(Json(crate::presentation::http::openapi::StatusResponse { status: "not_implemented".into() }))
}
