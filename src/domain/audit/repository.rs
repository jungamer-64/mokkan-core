// src/domain/audit/repository.rs
use crate::async_support::BoxFuture;
use crate::domain::audit::cursor::Cursor;
use crate::domain::audit::entity::{AuditLog, NewAuditLog};
use crate::domain::errors::DomainResult;

pub trait AuditLogRepository: Send + Sync {
    fn insert(&self, log: NewAuditLog) -> BoxFuture<'_, DomainResult<()>>;

    fn list(
        &self,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> BoxFuture<'_, DomainResult<(Vec<AuditLog>, Option<String>)>>;

    fn find_by_user(
        &self,
        user_id: i64,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> BoxFuture<'_, DomainResult<(Vec<AuditLog>, Option<String>)>>;

    fn find_by_resource<'a>(
        &'a self,
        resource_type: &'a str,
        resource_id: i64,
        limit: u32,
        cursor: Option<Cursor>,
    ) -> BoxFuture<'a, DomainResult<(Vec<AuditLog>, Option<String>)>>;
}
