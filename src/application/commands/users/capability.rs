use crate::application::{
    dto::AuthenticatedUser,
    error::{ApplicationError, ApplicationResult},
};

pub(super) fn ensure_capability(
    user: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> ApplicationResult<()> {
    if user.has_capability(resource, action) {
        Ok(())
    } else {
        Err(ApplicationError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
