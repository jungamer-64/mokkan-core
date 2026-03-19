// src/presentation/http/extractors.rs
use crate::{
    application::{dto::AuthenticatedUser, error::ApplicationError},
    presentation::http::state::HttpContext,
};
use axum::{Extension, extract::FromRequestParts, http::request::Parts};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};

use super::error::HttpError;

#[derive(Debug, Clone)]
pub struct Authenticated(pub AuthenticatedUser);

#[derive(Debug, Clone)]
pub struct MaybeAuthenticated(pub Option<AuthenticatedUser>);

fn cached_authenticated_user(parts: &Parts) -> Option<AuthenticatedUser> {
    parts.extensions.get::<AuthenticatedUser>().cloned()
}

// Validate that the authenticated user/session is not revoked and that the
// token version in the token is greater-or-equal to the user's minimum
// allowed token version. This centralizes the logic used by both the
// `Authenticated` and `MaybeAuthenticated` extractors.
async fn validate_not_revoked(
    app_state: &HttpContext,
    user: &AuthenticatedUser,
) -> Result<(), HttpError> {
    // Session-level revocation check
    if let Some(session_id) = &user.session_id {
        let session_store = app_state.services.session_revocation();
        if session_store
            .is_revoked(session_id)
            .await
            .map_err(HttpError::from_error)?
        {
            return Err(HttpError::from_error(ApplicationError::unauthorized(
                "session revoked",
            )));
        }
    }

    // Global token-version check
    if let Some(token_ver) = user.token_version {
        let session_store = app_state.services.token_version_store();
        if let Some(min_ver) = session_store
            .get_min_token_version(user.id.into())
            .await
            .map_err(HttpError::from_error)?
            && token_ver < min_ver
        {
            return Err(HttpError::from_error(ApplicationError::unauthorized(
                "token revoked",
            )));
        }
    }

    Ok(())
}

impl FromRequestParts<()> for Authenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let Extension(app_state) = Extension::<HttpContext>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                HttpError::from_error(ApplicationError::infrastructure(
                    "application state missing",
                ))
            })?;

        if let Some(user) = cached_authenticated_user(parts) {
            return Ok(Self(user));
        }

        let header = parts
            .headers
            .typed_get::<Authorization<Bearer>>()
            .ok_or_else(|| {
                HttpError::from_error(ApplicationError::unauthorized(
                    "missing Authorization header",
                ))
            })?;

        let token = header.token();
        let manager = app_state.services.token_manager();
        let user = manager
            .authenticate(token)
            .await
            .map_err(HttpError::from_error)?;

        validate_not_revoked(&app_state, &user).await?;
        Ok(Self(user))
    }
}

impl FromRequestParts<()> for MaybeAuthenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let Extension(app_state) = Extension::<HttpContext>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                HttpError::from_error(ApplicationError::infrastructure(
                    "application state missing",
                ))
            })?;

        if let Some(user) = cached_authenticated_user(parts) {
            return Ok(Self(Some(user)));
        }

        if let Some(header) = parts.headers.typed_get::<Authorization<Bearer>>() {
            let token = header.token();
            let manager = app_state.services.token_manager();
            let user = manager
                .authenticate(token)
                .await
                .map_err(HttpError::from_error)?;
            validate_not_revoked(&app_state, &user).await?;
            Ok(Self(Some(user)))
        } else {
            Ok(Self(None))
        }
    }
}
