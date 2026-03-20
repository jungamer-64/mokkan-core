use crate::application::{
    AuthenticatedUser,
    error::{AppError, AppResult},
};

pub(super) fn ensure_capability(
    user: &AuthenticatedUser,
    resource: &str,
    action: &str,
) -> AppResult<()> {
    if user.has_capability(resource, action) {
        Ok(())
    } else {
        Err(AppError::forbidden(format!(
            "missing capability {resource}:{action}"
        )))
    }
}
