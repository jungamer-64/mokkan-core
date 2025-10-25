// src/presentation/http/error.rs
use crate::application::{ApplicationResult, error::ApplicationError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug)]
pub struct HttpError {
    status: StatusCode,
    message: String,
}

impl HttpError {
    pub fn from_error(err: ApplicationError) -> Self {
        match err {
            ApplicationError::Validation(msg) => Self::new(StatusCode::BAD_REQUEST, msg),
            ApplicationError::NotFound(msg) => Self::new(StatusCode::NOT_FOUND, msg),
            ApplicationError::Conflict(msg) => Self::new(StatusCode::CONFLICT, msg),
            ApplicationError::Unauthorized(msg) => Self::new(StatusCode::UNAUTHORIZED, msg),
            ApplicationError::Forbidden(msg) => Self::new(StatusCode::FORBIDDEN, msg),
            ApplicationError::Infrastructure(err) => {
                // Log the detailed internal error for observability, but return a
                // generic message to the client to avoid leaking internals.
                tracing::error!(error = %err, "infrastructure error");
                Self::new(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_string(),
                )
            }
            ApplicationError::Domain(domain_err) => {
                Self::new(StatusCode::BAD_REQUEST, domain_err.to_string())
            }
        }
    }

    fn new(status: StatusCode, message: String) -> Self {
        Self { status, message }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        let payload = ErrorResponse {
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
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

pub type HttpResult<T> = Result<T, HttpError>;

pub trait IntoHttpResult<T> {
    fn into_http(self) -> HttpResult<T>;
}

impl<T> IntoHttpResult<T> for ApplicationResult<T> {
    fn into_http(self) -> HttpResult<T> {
        self.map_err(HttpError::from_error)
    }
}
