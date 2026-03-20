// src/application/commands/articles/capability.rs
use crate::application::{
    AuthenticatedUser,
    error::{AppError, AppResult},
};

pub(super) fn ensure_capability(
    actor: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> AppResult<()> {
    if actor.has_capability(resource, action) {
        Ok(())
    } else {
        Err(AppError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
