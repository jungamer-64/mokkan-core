use crate::application::{
    AuthenticatedUser,
    error::{AppError, AppResult},
};

pub(super) fn ensure_audit_capability(actor: &AuthenticatedUser) -> AppResult<()> {
    if actor.has_capability("audit", "read") {
        Ok(())
    } else {
        Err(AppError::forbidden("missing capability audit:read"))
    }
}

pub(super) fn normalize_limit(limit: u32) -> u32 {
    const DEFAULT_LIMIT: u32 = 20;
    const MAX_LIMIT: u32 = 100;

    if limit == 0 {
        DEFAULT_LIMIT
    } else {
        limit.min(MAX_LIMIT)
    }
}
