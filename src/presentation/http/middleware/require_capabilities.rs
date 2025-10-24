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
            let manager = state.services.token_manager();

            // Authenticate token
            match manager.authenticate(token).await {
                Ok(user) => {
                    // Session revocation and token-version checks
                    if let Some(session_id) = &user.session_id {
                        let session_store = state.services.session_revocation_store();
                        match session_store.is_revoked(session_id).await {
                            Ok(true) => {
                                return HttpError::from_error(ApplicationError::unauthorized("session revoked")).into_response();
                            }
                            Ok(false) => {}
                            Err(err) => {
                                return HttpError::from_error(err).into_response();
                            }
                        }
                    }

                    if let Some(token_ver) = user.token_version {
                        let session_store = state.services.session_revocation_store();
                        match session_store.get_min_token_version(user.id.into()).await {
                            Ok(Some(min_ver)) if token_ver < min_ver => {
                                return HttpError::from_error(ApplicationError::unauthorized("token revoked")).into_response();
                            }
                            Ok(_) => {}
                            Err(err) => {
                                return HttpError::from_error(err).into_response();
                            }
                        }
                    }
                    if user.has_capability(resource, action) {
                        // authorized, continue
                        return next.run(req).await;
                    } else {
                        return HttpError::from_error(ApplicationError::forbidden(format!("missing capability {resource}:{action}"))).into_response();
                    }
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
