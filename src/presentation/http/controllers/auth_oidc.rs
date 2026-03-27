// src/presentation/http/controllers/auth_oidc.rs
//! OIDC/OAuth2-style endpoints (authorization code + PKCE), token introspection and revocation.
//! This file parses either JSON or x-www-form-urlencoded bodies for /token.

use axum::{
    Extension, Json,
    extract::Query,
    response::{IntoResponse, Redirect, Response},
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt::Write as _;

use crate::application::services::{
    ExchangeAuthorizationCodeRequest, IssueAuthorizationCodeRequest, TokenIntrospection,
};
use crate::application::{AuthTokenDto, error::AppError};
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::MaybeAuthenticated;
use crate::presentation::http::state::HttpContext;

// ---------- Requests / Responses ----------

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenRequest {
    pub token: String,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct TokenExchangeRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub client_id: Option<String>,
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

impl From<TokenIntrospection> for IntrospectResponse {
    fn from(value: TokenIntrospection) -> Self {
        Self {
            active: value.active,
            scope: value.scope,
            username: value.username,
            sub: value.sub,
            exp: value.exp,
            iat: value.iat,
            session_id: value.session_id,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct AuthorizeRequest {
    pub response_type: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    /// For programmatic test flows you can pass `consent=approve` (otherwise a consent prompt JSON is returned)
    pub consent: Option<String>,
}

// ---------- Endpoints ----------

#[utoipa::path(
    post,
    path = "/api/v1/auth/token",
    request_body = TokenExchangeRequest,
    responses(
        (status = 200, description = "Tokens issued", body = crate::application::AuthTokenDto),
        (status = 400, description = "Bad request", body = crate::presentation::http::error::ResponsePayload),
    ),
    security([]),
    tag = "Auth"
)]
/// Exchange an authorization code for tokens.
///
/// # Errors
///
/// Returns an error if the request body is malformed, the grant type is not
/// supported, the code is missing, or the exchange fails.
pub async fn token(
    Extension(state): Extension<HttpContext>,
    body_bytes: axum::body::Bytes,
) -> HttpResult<Json<AuthTokenDto>> {
    // Received body as Bytes extractor. Try to parse either JSON or x-www-form-urlencoded
    let whole = body_bytes;

    // Try JSON first, then fall back to form-urlencoded
    let payload: TokenExchangeRequest = match serde_json::from_slice(&whole) {
        Ok(p) => p,
        Err(_) => {
            // parse as application/x-www-form-urlencoded
            serde_urlencoded::from_bytes(&whole).map_err(|_e| {
                crate::presentation::http::error::Error::from_error(AppError::validation(
                    "invalid token request",
                ))
            })?
        }
    };

    if payload.grant_type != "authorization_code" {
        return Err(crate::presentation::http::error::Error::from_error(
            AppError::validation("unsupported grant_type"),
        ));
    }

    let code = payload.code.ok_or_else(|| {
        crate::presentation::http::error::Error::from_error(AppError::validation("code required"))
    })?;

    let token = state
        .services
        .auth
        .exchange_authorization_code(ExchangeAuthorizationCodeRequest {
            code,
            code_verifier: payload.code_verifier,
            redirect_uri: payload.redirect_uri,
        })
        .await
        .into_http()?;

    Ok(Json(token))
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
/// Introspect a token and report whether it is active.
///
/// # Errors
///
/// Returns an error only if request extraction fails before the handler runs.
pub async fn introspect(
    Extension(state): Extension<HttpContext>,
    Json(payload): Json<TokenRequest>,
) -> HttpResult<Json<IntrospectResponse>> {
    state
        .services
        .auth
        .introspect_token(&payload.token)
        .await
        .into_http()
        .map(IntrospectResponse::from)
        .map(Json)
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
/// Revoke a token's backing session when possible.
///
/// # Errors
///
/// Returns an error if session revocation fails after token authentication.
pub async fn revoke(
    Extension(state): Extension<HttpContext>,
    Json(payload): Json<TokenRequest>,
) -> HttpResult<Json<crate::presentation::http::openapi::StatusResponse>> {
    state
        .services
        .auth
        .revoke_token(&payload.token)
        .await
        .into_http()?;

    Ok(Json(crate::presentation::http::openapi::StatusResponse {
        status: "revoked".into(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    responses(
        (status = 302, description = "Redirect back to client with authorization code"),
        (status = 200, description = "Consent required / prompt (JSON)", body = serde_json::Value),
        (status = 400, description = "Bad request", body = crate::presentation::http::error::ResponsePayload),
    ),
    security([]),
    tag = "Auth"
)]
/// Start an `OAuth2` authorization code flow.
///
/// # Errors
///
/// Returns an error if the request is invalid, the caller is unauthenticated,
/// the redirect URI is rejected, or authorization code persistence fails.
pub async fn authorize(
    Extension(state): Extension<HttpContext>,
    Query(params): Query<AuthorizeRequest>,
    MaybeAuthenticated(maybe_user): MaybeAuthenticated,
) -> HttpResult<Response> {
    // Basic validation
    if params.response_type.as_deref() != Some("code") {
        return Err(crate::presentation::http::error::Error::from_error(
            AppError::validation("unsupported response_type"),
        ));
    }

    let user = maybe_user.ok_or_else(|| {
        crate::presentation::http::error::Error::from_error(AppError::unauthorized(
            "login required",
        ))
    })?;

    // If consent wasn't explicitly granted, return a minimal consent prompt response so
    // clients (or a UI) can render a consent screen. For automated tests, client may pass
    // `consent=approve`.
    if let Some(prompt) = maybe_consent_prompt(&params, &user) {
        return Ok(Json(prompt).into_response());
    }

    // Create and persist the authorization code (delegated to helper)
    let issued = state
        .services
        .auth
        .issue_authorization_code(
            &user,
            IssueAuthorizationCodeRequest {
                client_id: params.client_id.clone(),
                redirect_uri: params.redirect_uri.clone(),
                scope: params.scope.clone(),
                code_challenge: params.code_challenge.clone(),
                code_challenge_method: params.code_challenge_method.clone(),
            },
        )
        .await
        .into_http()?;

    // Redirect back to client (per OAuth2). If redirect_uri isn't provided, return the code in JSON.
    if let Some(redirect) = params.redirect_uri.as_deref() {
        let uri = build_redirect_uri(redirect, &issued.code, params.state.as_ref());
        return Ok(Redirect::to(&uri).into_response());
    }

    Ok(Json(serde_json::json!({"code": issued.code, "state": params.state})).into_response())
}

// ---------- Helpers ----------

// Return a consent prompt JSON when consent hasn't been granted yet.
fn maybe_consent_prompt(
    params: &AuthorizeRequest,
    user: &crate::application::AuthenticatedUser,
) -> Option<JsonValue> {
    if params.consent.as_deref() == Some("approve") {
        None
    } else {
        Some(serde_json::json!({
            "consent_required": true,
            "user": { "id": i64::from(user.id), "username": user.username },
            "scopes": params.scope,
            "message": "Set consent=approve to grant and receive an authorization code"
        }))
    }
}

// Build a simple redirect URL (avoid adding a heavy URL parser dependency here).
fn build_redirect_uri(redirect: &str, code: &str, state: Option<&String>) -> String {
    let mut uri = redirect.to_string();
    if uri.contains('?') {
        let _ = write!(uri, "&code={code}");
    } else {
        let _ = write!(uri, "?code={code}");
    }
    if let Some(s) = state {
        let _ = write!(uri, "&state={s}");
    }

    uri
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
/// Minimal OpenAPI-only stub for the authorize endpoint.
///
/// # Errors
///
/// Returns an error only if request extraction fails before the handler runs.
pub fn authorize_openapi_stub(
    Extension(_state): Extension<HttpContext>,
) -> std::future::Ready<HttpResult<Json<crate::presentation::http::openapi::StatusResponse>>> {
    std::future::ready(Ok(Json(
        crate::presentation::http::openapi::StatusResponse {
            status: "not_implemented".into(),
        },
    )))
}
