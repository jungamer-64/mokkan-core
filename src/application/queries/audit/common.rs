use crate::application::{
    dto::AuthenticatedUser,
    error::{ApplicationError, ApplicationResult},
};

pub(super) fn ensure_audit_capability(actor: &AuthenticatedUser) -> ApplicationResult<()> {
    if actor.has_capability("audit", "read") {
        Ok(())
    } else {
        Err(ApplicationError::forbidden("missing capability audit:read"))
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
