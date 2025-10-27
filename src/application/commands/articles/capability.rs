// src/application/commands/articles/capability.rs
use crate::application::{
    dto::AuthenticatedUser,
    error::{ApplicationError, ApplicationResult},
};

pub(super) fn ensure_capability(
    actor: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> ApplicationResult<()> {
    if actor.has_capability(resource, action) {
        Ok(())
    } else {
        Err(ApplicationError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
