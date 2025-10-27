// src/application/error.rs
use crate::domain::errors::DomainError;
use anyhow::Error as AnyhowError;
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
    Infrastructure(#[source] AnyhowError),
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

    /// Create an infrastructure error from a message or an existing error.
    ///
    /// Many call sites pass `err.to_string()`; to keep those call sites simple
    /// we accept `impl Into<String>` here and convert it into an `anyhow::Error`.
    pub fn infrastructure(msg: impl Into<String>) -> Self {
        Self::Infrastructure(AnyhowError::msg(msg.into()))
    }

    /// Create an infrastructure error from any error type, preserving the
    /// original error as the source. This is useful when you already have an
    /// `anyhow::Error` (or something that can be converted into one) and want to
    /// keep the richer error context instead of just converting it to a string.
    pub fn infrastructure_error(err: impl Into<AnyhowError>) -> Self {
        Self::Infrastructure(err.into())
    }
}
