// src/presentation/http/middleware/require_capabilities.rs
use crate::application::error::AppError;
use crate::presentation::http::error::Error as HttpError;
use crate::presentation::http::state::HttpContext;
use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use headers::{Authorization, HeaderMapExt, authorization::Bearer};

/// Middleware function that enforces a single capability (resource, action).
///
/// Usage: `axum::middleware::from_fn(move |req, next| require_capability(req, next, "articles", "create"))`
pub async fn require_capability(
    mut req: Request<Body>,
    next: Next,
    resource: &'static str,
    action: &'static str,
) -> Response {
    if let Some(header) = req.headers().typed_get::<Authorization<Bearer>>() {
        let token = header.token();

        if let Some(state) = req.extensions().get::<HttpContext>() {
            match state
                .services
                .authenticate_and_authorize(token, resource, action)
                .await
            {
                Ok(user) => {
                    req.extensions_mut().insert(user);
                    next.run(req).await
                }
                Err(err) => HttpError::from_error(err).into_response(),
            }
        } else {
            HttpError::from_error(AppError::infrastructure("application state missing"))
                .into_response()
        }
    } else {
        HttpError::from_error(AppError::unauthorized("missing Authorization header"))
            .into_response()
    }
}
