use crate::domain::audit::entity::AuditLog;
use crate::domain::errors::DomainResult;
use async_trait::async_trait;

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn insert(&self, log: AuditLog) -> DomainResult<()>;
}
