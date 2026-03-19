use std::sync::Arc;

use crate::domain::audit::repository::AuditLogRepository;

#[must_use]
pub struct AuditQueryService {
    pub(super) repo: Arc<dyn AuditLogRepository>,
}

impl AuditQueryService {
    pub fn new(repo: Arc<dyn AuditLogRepository>) -> Self {
        Self { repo }
    }
}
