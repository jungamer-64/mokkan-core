// src/application/error.rs
use crate::domain::errors::DomainError;
use thiserror::Error;

pub type ApplicationResult<T> = Result<T, ApplicationError>;

#[derive(Debug, Error)]
pub enum ApplicationError {
    #[error(transparent)]
    Domain(#[from] DomainError),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("resource not found: {0}")]
    NotFound(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("infrastructure failure: {0}")]
    Infrastructure(String),
}

impl ApplicationError {
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::Forbidden(msg.into())
    }

    pub fn infrastructure(msg: impl Into<String>) -> Self {
        Self::Infrastructure(msg.into())
    }
}
