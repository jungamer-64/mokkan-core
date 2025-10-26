// src/presentation/http/middleware/require_capabilities.rs
use crate::application::error::ApplicationError;
use crate::presentation::http::error::HttpError;
use crate::presentation::http::state::HttpState;
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
    req: Request<Body>,
    next: Next,
    resource: &'static str,
    action: &'static str,
) -> Response {
    // Extract Authorization header
    if let Some(header) = req.headers().typed_get::<Authorization<Bearer>>() {
        let token = header.token();

        // Get application state from request extensions
        if let Some(state) = req.extensions().get::<HttpState>() {
            // Delegate to application services for authentication + authorization
            match state.services.authenticate_and_authorize(token, resource, action).await {
                Ok(_user) => {
                    return next.run(req).await;
                }
                Err(err) => {
                    return HttpError::from_error(err).into_response();
                }
            }
        } else {
            return HttpError::from_error(ApplicationError::infrastructure("application state missing")).into_response();
        }
    } else {
        return HttpError::from_error(ApplicationError::unauthorized("missing Authorization header")).into_response();
    }
}
