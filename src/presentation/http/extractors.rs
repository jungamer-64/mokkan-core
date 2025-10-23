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
                    HttpError::from_error(ApplicationError::Infrastructure(
                        "application state missing".into(),
                    ))
                })?;


            let header = parts
                .headers
                .typed_get::<Authorization<Bearer>>()
                .ok_or_else(|| {
                    HttpError::from_error(ApplicationError::Unauthorized(
                        "missing Authorization header".into(),
                    ))
                })?;


            let token = header.token();
            let manager = app_state.services.token_manager();
            // Authorization header present, attempt to authenticate
            let user = manager
                .authenticate(token)
                .await
                .map_err(HttpError::from_error)?;

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
                    HttpError::from_error(ApplicationError::Infrastructure(
                        "application state missing".into(),
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
                Ok(Self(Some(user)))
            } else {
                Ok(Self(None))
            }
        }
    }
}
