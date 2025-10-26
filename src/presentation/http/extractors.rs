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
                    HttpError::from_error(ApplicationError::infrastructure("application state missing"))
                })?;


            let header = parts
                .headers
                .typed_get::<Authorization<Bearer>>()
                .ok_or_else(|| {
                    HttpError::from_error(ApplicationError::unauthorized("missing Authorization header"))
                })?;


            let token = header.token();
            let manager = app_state.services.token_manager();
            // Authorization header present, attempt to authenticate
            let user = manager
                .authenticate(token)
                .await
                .map_err(HttpError::from_error)?;

            // Enforce session revocation and minimum token version checks here as
            // well so that routes which use the `Authenticated` extractor
            // (instead of the capability middleware) still observe revocation.
            if let Some(session_id) = &user.session_id {
                let session_store = app_state.services.session_revocation_store();
                match session_store.is_revoked(session_id).await {
                    Ok(true) => {
                        return Err(HttpError::from_error(ApplicationError::unauthorized("session revoked")));
                    }
                    Ok(false) => {}
                    Err(err) => {
                        return Err(HttpError::from_error(err));
                    }
                }
            }

            if let Some(token_ver) = user.token_version {
                let session_store = app_state.services.session_revocation_store();
                match session_store.get_min_token_version(user.id.into()).await {
                    Ok(Some(min_ver)) if token_ver < min_ver => {
                        return Err(HttpError::from_error(ApplicationError::unauthorized("token revoked")));
                    }
                    Ok(_) => {}
                    Err(err) => {
                        return Err(HttpError::from_error(err));
                    }
                }
            }

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
                    HttpError::from_error(ApplicationError::infrastructure("application state missing"))
                })?;

            // MaybeAuthenticated: proceed if header present

            if let Some(header) = parts.headers.typed_get::<Authorization<Bearer>>() {
                let token = header.token();
                let manager = app_state.services.token_manager();
                let user = manager
                    .authenticate(token)
                    .await
                    .map_err(HttpError::from_error)?;
                // Perform the same revocation / token-version checks as in
                // the strict `Authenticated` extractor so that presence of a
                // valid token also implies it is not revoked.
                if let Some(session_id) = &user.session_id {
                    let session_store = app_state.services.session_revocation_store();
                    match session_store.is_revoked(session_id).await {
                        Ok(true) => {
                            return Err(HttpError::from_error(ApplicationError::unauthorized("session revoked")));
                        }
                        Ok(false) => {}
                        Err(err) => {
                            return Err(HttpError::from_error(err));
                        }
                    }
                }

                if let Some(token_ver) = user.token_version {
                    let session_store = app_state.services.session_revocation_store();
                    match session_store.get_min_token_version(user.id.into()).await {
                        Ok(Some(min_ver)) if token_ver < min_ver => {
                            return Err(HttpError::from_error(ApplicationError::unauthorized("token revoked")));
                        }
                        Ok(_) => {}
                        Err(err) => {
                            return Err(HttpError::from_error(err));
                        }
                    }
                }

                Ok(Self(Some(user)))
            } else {
                Ok(Self(None))
            }
        }
    }
}
