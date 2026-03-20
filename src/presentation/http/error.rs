// src/presentation/http/error.rs
use crate::application::{AppResult, error::AppError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug)]
pub struct Error {
    status: StatusCode,
    message: String,
}

impl Error {
    #[must_use]
    pub fn from_error(err: AppError) -> Self {
        match err {
            AppError::Validation(msg) => Self::new(StatusCode::BAD_REQUEST, msg),
            AppError::NotFound(msg) => Self::new(StatusCode::NOT_FOUND, msg),
            AppError::Conflict(msg) => Self::new(StatusCode::CONFLICT, msg),
            AppError::Unauthorized(msg) => Self::new(StatusCode::UNAUTHORIZED, msg),
            AppError::Forbidden(msg) => Self::new(StatusCode::FORBIDDEN, msg),
            AppError::Infrastructure(err) => {
                // Log the detailed internal error for observability, but return a
                // generic message to the client to avoid leaking internals.
                tracing::error!(error = %err, "infrastructure error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
            AppError::Domain(domain_err) => {
                Self::new(StatusCode::BAD_REQUEST, domain_err.to_string())
            }
        }
    }

    const fn new(status: StatusCode, message: String) -> Self {
        Self { status, message }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let payload = ResponsePayload {
            error: self
                .status
                .canonical_reason()
                .unwrap_or("error")
                .to_string(),
            message: self.message,
        };
        (self.status, Json(payload)).into_response()
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResponsePayload {
    pub error: String,
    pub message: String,
}

pub type HttpResult<T> = Result<T, Error>;

pub trait IntoHttpResult<T> {
    /// Convert an application-layer `Result` into an HTTP-layer `Result`.
    ///
    /// # Errors
    /// Returns [`Error`] when the source value is an application error.
    fn into_http(self) -> HttpResult<T>;
}

impl<T> IntoHttpResult<T> for AppResult<T> {
    fn into_http(self) -> HttpResult<T> {
        self.map_err(Error::from_error)
    }
}
