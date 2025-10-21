// src/presentation/http/extractors.rs
use crate::{
    application::{dto::AuthenticatedUser, error::ApplicationError},
    presentation::http::state::HttpState,
};
use async_trait::async_trait;
use axum::{Extension, extract::FromRequestParts, http::request::Parts};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};

use super::error::HttpError;

#[derive(Debug, Clone)]
pub struct Authenticated(pub AuthenticatedUser);

#[derive(Debug, Clone)]
pub struct MaybeAuthenticated(pub Option<AuthenticatedUser>);

#[async_trait]
impl FromRequestParts<()> for Authenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
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
        let user = manager
            .authenticate(token)
            .await
            .map_err(HttpError::from_error)?;

        Ok(Self(user))
    }
}

#[async_trait]
impl FromRequestParts<()> for MaybeAuthenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let Extension(app_state) = Extension::<HttpState>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                HttpError::from_error(ApplicationError::Infrastructure(
                    "application state missing".into(),
                ))
            })?;

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
