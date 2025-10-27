// src/presentation/http/extractors.rs
use crate::{
    application::{dto::AuthenticatedUser, error::ApplicationError},
    presentation::http::state::HttpState,
};
use axum::{Extension, extract::FromRequestParts, http::request::Parts};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};
use std::future::Future;

use super::error::HttpError;

#[derive(Debug, Clone)]
pub struct Authenticated(pub AuthenticatedUser);

#[derive(Debug, Clone)]
pub struct MaybeAuthenticated(pub Option<AuthenticatedUser>);

// Validate that the authenticated user/session is not revoked and that the
// token version in the token is greater-or-equal to the user's minimum
// allowed token version. This centralizes the logic used by both the
// `Authenticated` and `MaybeAuthenticated` extractors.
async fn validate_not_revoked(
    app_state: &HttpState,
    user: &AuthenticatedUser,
) -> Result<(), HttpError> {
    // Session-level revocation check
    if let Some(session_id) = &user.session_id {
        let session_store = app_state.services.session_revocation_store();
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
        let session_store = app_state.services.session_revocation_store();
        if let Some(min_ver) = session_store
            .get_min_token_version(user.id.into())
            .await
            .map_err(HttpError::from_error)?
        {
            if token_ver < min_ver {
                return Err(HttpError::from_error(ApplicationError::unauthorized(
                    "token revoked",
                )));
            }
        }
    }

    Ok(())
}

impl FromRequestParts<()> for Authenticated {
    type Rejection = HttpError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &(),
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let Extension(app_state) = Extension::<HttpState>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    HttpError::from_error(ApplicationError::infrastructure(
                        "application state missing",
                    ))
                })?;

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
            // Authorization header present, attempt to authenticate
            let user = manager
                .authenticate(token)
                .await
                .map_err(HttpError::from_error)?;

            // Validate revocation/token-version using the shared helper
            validate_not_revoked(&app_state, &user).await?;
            Ok(Self(user))
        }
    }
}

impl FromRequestParts<()> for MaybeAuthenticated {
    type Rejection = HttpError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &(),
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let Extension(app_state) = Extension::<HttpState>::from_request_parts(parts, state)
                .await
                .map_err(|_| {
                    HttpError::from_error(ApplicationError::infrastructure(
                        "application state missing",
                    ))
                })?;

            // MaybeAuthenticated: proceed if header present

            if let Some(header) = parts.headers.typed_get::<Authorization<Bearer>>() {
                let token = header.token();
                let manager = app_state.services.token_manager();
                let user = manager
                    .authenticate(token)
                    .await
                    .map_err(HttpError::from_error)?;
                // Validate revocation/token-version using the shared helper
                validate_not_revoked(&app_state, &user).await?;
                Ok(Self(Some(user)))
            } else {
                Ok(Self(None))
            }
        }
    }
}
