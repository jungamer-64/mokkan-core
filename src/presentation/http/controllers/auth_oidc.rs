// src/presentation/http/controllers/auth_oidc.rs
//! OIDC/OAuth2-style endpoints (authorization code + PKCE), token introspection and revocation.
//! This file parses either JSON or x-www-form-urlencoded bodies for /token.

use axum::{
    extract::Query,
    response::{IntoResponse, Redirect, Response},
    Extension, Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::application::dto::{AuthTokenDto, TokenSubject};
use crate::application::error::ApplicationError;
use crate::application::ports::authorization_code::AuthorizationCode;
use crate::presentation::http::error::{HttpResult, IntoHttpResult};
use crate::presentation::http::extractors::MaybeAuthenticated;
use crate::presentation::http::state::HttpState;

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
    #[serde(skip_serializing_if = "Option::is_none")] pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub sub: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub exp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub iat: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")] pub session_id: Option<String>,
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
        (status = 200, description = "Tokens issued", body = crate::application::dto::AuthTokenDto),
        (status = 400, description = "Bad request", body = crate::presentation::http::error::ErrorResponse),
    ),
    security([]),
    tag = "Auth"
)]
pub async fn token(
    Extension(state): Extension<HttpState>,
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
                crate::presentation::http::error::HttpError::from_error(
                    ApplicationError::validation("invalid token request"),
                )
            })?
        }
    };

    if payload.grant_type != "authorization_code" {
        return Err(crate::presentation::http::error::HttpError::from_error(
            ApplicationError::validation("unsupported grant_type"),
        ));
    }

    let code = payload.code.as_deref().ok_or_else(|| {
        crate::presentation::http::error::HttpError::from_error(
            ApplicationError::validation("code required"),
        )
    })?;

    let token = state
        .services
        .exchange_authorization_code(
            code,
            payload.code_verifier.as_deref(),
            payload.redirect_uri.as_deref(),
        )
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

#[utoipa::path(
    get,
    path = "/api/v1/auth/authorize",
    responses(
        (status = 302, description = "Redirect back to client with authorization code"),
        (status = 200, description = "Consent required / prompt (JSON)", body = serde_json::Value),
        (status = 400, description = "Bad request", body = crate::presentation::http::error::ErrorResponse),
    ),
    security([]),
    tag = "Auth"
)]
pub async fn authorize(
    Extension(state): Extension<HttpState>,
    Query(params): Query<AuthorizeRequest>,
    MaybeAuthenticated(maybe_user): MaybeAuthenticated,
) -> HttpResult<Response> {
    // Basic validation
    if params.response_type.as_deref() != Some("code") {
        return Err(crate::presentation::http::error::HttpError::from_error(
            ApplicationError::validation("unsupported response_type"),
        ));
    }

    let user = maybe_user.ok_or_else(|| {
        crate::presentation::http::error::HttpError::from_error(
            ApplicationError::unauthorized("login required"),
        )
    })?;

    // If consent wasn't explicitly granted, return a minimal consent prompt response so
    // clients (or a UI) can render a consent screen. For automated tests, client may pass
    // `consent=approve`.
    if let Some(prompt) = maybe_consent_prompt(&params, &user) {
        return Ok(Json(prompt).into_response());
    }

    // Create and persist the authorization code (delegated to helper)
    let code = create_and_store_code(&state, &user, &params).await.into_http()?;

    // Redirect back to client (per OAuth2). If redirect_uri isn't provided, return the code in JSON.
    if let Some(redirect) = params.redirect_uri.as_deref() {
        // Basic safety checks for redirect URIs to avoid open-redirect abuse.
        validate_redirect_uri(redirect)?;
        let uri = build_redirect_uri(redirect, &code, params.state.as_ref());
        return Ok(Redirect::to(&uri).into_response());
    }

    Ok(Json(serde_json::json!({"code": code, "state": params.state})).into_response())
}

// ---------- Helpers ----------

// Helper: create an authorization code and persist it using the configured store.
async fn create_and_store_code(
    state: &HttpState,
    user: &crate::application::dto::AuthenticatedUser,
    params: &AuthorizeRequest,
) -> crate::application::ApplicationResult<String> {
    let code = Uuid::new_v4().to_string();
    let now = Utc::now();
    let expires = now + chrono::Duration::minutes(5);

    let subject = TokenSubject::from_authenticated(user);

    let auth_code = AuthorizationCode {
        code: code.clone(),
        client_id: params.client_id.clone(),
        redirect_uri: params.redirect_uri.clone(),
        subject,
        scope: params.scope.clone(),
        code_challenge: params.code_challenge.clone(),
        code_challenge_method: params.code_challenge_method.clone(),
        created_at: now,
        expires_at: expires,
    };

    state
        .services
        .authorization_code_store()
        .create_code(auth_code)
        .await?;

    Ok(code)
}

// Return a consent prompt JSON when consent hasn't been granted yet.
fn maybe_consent_prompt(
    params: &AuthorizeRequest,
    user: &crate::application::dto::AuthenticatedUser,
) -> Option<JsonValue> {
    if params.consent.as_deref() != Some("approve") {
        Some(serde_json::json!({
            "consent_required": true,
            "user": { "id": i64::from(user.id), "username": user.username },
            "scopes": params.scope,
            "message": "Set consent=approve to grant and receive an authorization code"
        }))
    } else {
        None
    }
}

// Build a simple redirect URL (avoid adding a heavy URL parser dependency here).
fn build_redirect_uri(redirect: &str, code: &str, state: Option<&String>) -> String {
    let mut uri = redirect.to_string();
    if uri.contains('?') {
        uri.push_str(&format!("&code={}", code));
    } else {
        uri.push_str(&format!("?code={}", code));
    }
    if let Some(s) = state {
        uri.push_str(&format!("&state={}", s));
    }

    uri
}

// Very small validation to reduce risk of open-redirects. This is intentionally
// conservative: only allow http(s) schemes and refuse fragment identifiers.
fn validate_redirect_uri(redirect: &str) -> Result<(), crate::presentation::http::error::HttpError> {
    if redirect.contains('#') {
        return Err(crate::presentation::http::error::HttpError::from_error(
            ApplicationError::validation("redirect_uri must not contain fragment"),
        ));
    }

    if !(redirect.starts_with("http://") || redirect.starts_with("https://")) {
        return Err(crate::presentation::http::error::HttpError::from_error(
            ApplicationError::validation("invalid redirect_uri"),
        ));
    }

    Ok(())
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
