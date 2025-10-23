use super::map_sqlx;
use crate::domain::audit::entity::AuditLog;
use crate::domain::errors::DomainResult;
use async_trait::async_trait;
use sqlx::PgPool;

#[derive(Clone)]
pub struct PostgresAuditLogRepository {
    pool: PgPool,
}

impl PostgresAuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::audit::repository::AuditLogRepository for PostgresAuditLogRepository {
    async fn insert(&self, log: AuditLog) -> DomainResult<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_logs (user_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(log.user_id.map(i64::from))
        .bind(log.action)
        .bind(log.resource_type)
        .bind(log.resource_id)
        .bind(log.details)
        .bind(log.ip_address)
        .bind(log.user_agent)
        .execute(&self.pool)
        .await
        .map_err(map_sqlx)?;

        Ok(())
    }
}
