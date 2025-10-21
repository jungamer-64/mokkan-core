use crate::application::{ApplicationResult, error::ApplicationError};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

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
            ApplicationError::Infrastructure(msg) => {
                Self::new(StatusCode::INTERNAL_SERVER_ERROR, msg)
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
        let payload = ErrorBody {
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

#[derive(Serialize)]
struct ErrorBody {
    error: String,
    message: String,
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
