// src/presentation/http/controllers/discovery.rs
use crate::presentation::http::error::HttpResult;
use crate::presentation::http::state::HttpState;
use axum::{Extension, Json};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct OpenIdConfiguration {
    pub issuer: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub userinfo_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_session_endpoint: Option<String>,
    pub jwks_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revocation_endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub introspection_endpoint: Option<String>,

    pub response_types_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub subject_types_supported: Vec<String>,
    pub id_token_signing_alg_values_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub scopes_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub claims_supported: Vec<String>,
    pub claim_types_supported: Vec<String>,
    pub request_parameter_supported: bool,
}

#[utoipa::path(
    get,
    path = "/.well-known/openid-configuration",
    responses(
        (status = 200, description = "OpenID Connect Discovery document", body = OpenIdConfiguration),
    ),
    security([]),
    tag = "Auth"
)]
/// Serve the `OpenID Connect` discovery document.
///
/// # Errors
///
/// Returns an error only if request extraction fails before the handler runs.
pub async fn openid_configuration(
    Extension(_state): Extension<HttpState>,
) -> HttpResult<Json<OpenIdConfiguration>> {
    let issuer = crate::config::AppConfig::oidc_issuer_from_env();
    let issuer = issuer.trim_end_matches('/').to_string();

    // Map discovery endpoints to our existing (or conventional) routes
    let authorization_endpoint = format!("{issuer}/api/v1/auth/authorize");
    let token_endpoint = format!("{issuer}/api/v1/auth/token");
    let userinfo_endpoint = format!("{issuer}/api/v1/auth/me");
    let end_session_endpoint = format!("{issuer}/api/v1/auth/logout");
    let jwks_uri = format!("{issuer}/api/v1/auth/keys");
    let revocation_endpoint = format!("{issuer}/api/v1/auth/revoke");
    let introspection_endpoint = format!("{issuer}/api/v1/auth/introspect");

    let cfg = OpenIdConfiguration {
        issuer,
        authorization_endpoint: Some(authorization_endpoint),
        token_endpoint: Some(token_endpoint),
        userinfo_endpoint: Some(userinfo_endpoint),
        end_session_endpoint: Some(end_session_endpoint),
        jwks_uri,
        revocation_endpoint: Some(revocation_endpoint),
        introspection_endpoint: Some(introspection_endpoint),

        response_types_supported: vec!["code".into(), "token".into(), "id_token".into()],
        response_modes_supported: vec!["query".into(), "fragment".into(), "form_post".into()],
        grant_types_supported: vec![
            "authorization_code".into(),
            "refresh_token".into(),
            "client_credentials".into(),
        ],
        subject_types_supported: vec!["public".into()],
        id_token_signing_alg_values_supported: vec!["RS256".into()],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic".into(),
            "private_key_jwt".into(),
        ],
        scopes_supported: vec![
            "openid".into(),
            "profile".into(),
            "email".into(),
            "offline_access".into(),
        ],
        code_challenge_methods_supported: vec!["S256".into()],
        claims_supported: vec![
            "sub".into(),
            "name".into(),
            "email".into(),
            "email_verified".into(),
            "preferred_username".into(),
        ],
        claim_types_supported: vec!["normal".into()],
        request_parameter_supported: false,
    };

    Ok(Json(cfg))
}
