// src/domain/audit/repository.rs
use crate::domain::audit::cursor::Cursor;
use crate::domain::audit::entity::{AuditLog, NewAuditLog};
use crate::domain::errors::DomainResult;
use async_trait::async_trait;

#[async_trait]
pub trait AuditLogRepository: Send + Sync {
    async fn insert(&self, log: NewAuditLog) -> DomainResult<()>;

    async fn list(
        &self,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)>;

    async fn find_by_user(
        &self,
        user_id: i64,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)>;

    async fn find_by_resource(
        &self,
        resource_type: &str,
        resource_id: i64,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)>;
}
