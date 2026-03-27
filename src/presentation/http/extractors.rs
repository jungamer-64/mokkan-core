// src/presentation/http/extractors.rs
use crate::{
    application::{AuthenticatedUser, error::AppError},
    presentation::http::state::HttpContext,
};
use axum::{Extension, extract::FromRequestParts, http::request::Parts};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};

use super::error::Error as HttpError;

#[derive(Debug, Clone)]
pub struct Authenticated(pub AuthenticatedUser);

#[derive(Debug, Clone)]
pub struct MaybeAuthenticated(pub Option<AuthenticatedUser>);

fn cached_authenticated_user(parts: &Parts) -> Option<AuthenticatedUser> {
    parts.extensions.get::<AuthenticatedUser>().cloned()
}

impl FromRequestParts<()> for Authenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let Extension(app_state) = Extension::<HttpContext>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                HttpError::from_error(AppError::infrastructure("application state missing"))
            })?;

        if let Some(user) = cached_authenticated_user(parts) {
            return Ok(Self(user));
        }

        let header = parts
            .headers
            .typed_get::<Authorization<Bearer>>()
            .ok_or_else(|| {
                HttpError::from_error(AppError::unauthorized("missing Authorization header"))
            })?;

        let token = header.token();
        let user = app_state
            .services
            .auth
            .authenticate(token)
            .await
            .map_err(HttpError::from_error)?;

        parts.extensions.insert(user.clone());
        Ok(Self(user))
    }
}

impl FromRequestParts<()> for MaybeAuthenticated {
    type Rejection = HttpError;

    async fn from_request_parts(parts: &mut Parts, state: &()) -> Result<Self, Self::Rejection> {
        let Extension(app_state) = Extension::<HttpContext>::from_request_parts(parts, state)
            .await
            .map_err(|_| {
                HttpError::from_error(AppError::infrastructure("application state missing"))
            })?;

        if let Some(user) = cached_authenticated_user(parts) {
            return Ok(Self(Some(user)));
        }

        if let Some(header) = parts.headers.typed_get::<Authorization<Bearer>>() {
            let token = header.token();
            let user = app_state
                .services
                .auth
                .authenticate(token)
                .await
                .map_err(HttpError::from_error)?;
            parts.extensions.insert(user.clone());
            Ok(Self(Some(user)))
        } else {
            Ok(Self(None))
        }
    }
}
