// src/infrastructure/repositories/postgres_audit_log.rs
use super::map_sqlx;
use crate::domain::audit::cursor::AuditLogCursor;
use crate::domain::audit::entity::AuditLog;
use crate::domain::errors::DomainResult;
use async_trait::async_trait;
use chrono::Utc;
use sqlx::PgPool;
const QUERY_LIST_WITH_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs WHERE (created_at, id) < ($1, $2) ORDER BY created_at DESC, id DESC LIMIT $3";
const QUERY_LIST_NO_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs ORDER BY created_at DESC, id DESC LIMIT $1";
const QUERY_FIND_BY_USER_WITH_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs WHERE user_id = $1 AND (created_at, id) < ($2, $3) ORDER BY created_at DESC, id DESC LIMIT $4";
const QUERY_FIND_BY_USER_NO_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs WHERE user_id = $1 ORDER BY created_at DESC, id DESC LIMIT $2";
const QUERY_FIND_BY_RESOURCE_WITH_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs WHERE resource_type = $1 AND resource_id = $2 AND (created_at, id) < ($3, $4) ORDER BY created_at DESC, id DESC LIMIT $5";
const QUERY_FIND_BY_RESOURCE_NO_CURSOR: &str = "SELECT id, user_id, action, resource_type, resource_id, details, ip_address, user_agent, created_at FROM audit_logs WHERE resource_type = $1 AND resource_id = $2 ORDER BY created_at DESC, id DESC LIMIT $3";

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

    async fn list(
        &self,
        limit: u32,
        cursor: Option<AuditLogCursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)> {
        if let Some(c) = cursor {
            let rows = sqlx::query(QUERY_LIST_WITH_CURSOR)
                .bind(c.created_at)
                .bind(c.id)
                .bind((limit + 1) as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        // no cursor
        let rows = sqlx::query(QUERY_LIST_NO_CURSOR)
            .bind((limit + 1) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;

        Ok(map_rows_to_logs(rows, limit))
    }

    async fn find_by_user(
        &self,
        user_id: i64,
        limit: u32,
        cursor: Option<AuditLogCursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)> {
        if let Some(c) = cursor {
            let rows = sqlx::query(QUERY_FIND_BY_USER_WITH_CURSOR)
                .bind(user_id)
                .bind(c.created_at)
                .bind(c.id)
                .bind((limit + 1) as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        let rows = sqlx::query(QUERY_FIND_BY_USER_NO_CURSOR)
            .bind(user_id)
            .bind((limit + 1) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;

        Ok(map_rows_to_logs(rows, limit))
    }

    async fn find_by_resource(
        &self,
        resource_type: &str,
        resource_id: i64,
        limit: u32,
        cursor: Option<AuditLogCursor>,
    ) -> DomainResult<(Vec<AuditLog>, Option<String>)> {
        if let Some(c) = cursor {
            let rows = sqlx::query(QUERY_FIND_BY_RESOURCE_WITH_CURSOR)
                .bind(resource_type)
                .bind(resource_id)
                .bind(c.created_at)
                .bind(c.id)
                .bind((limit + 1) as i64)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        let rows = sqlx::query(QUERY_FIND_BY_RESOURCE_NO_CURSOR)
            .bind(resource_type)
            .bind(resource_id)
            .bind((limit + 1) as i64)
            .fetch_all(&self.pool)
            .await
            .map_err(map_sqlx)?;

        Ok(map_rows_to_logs(rows, limit))
    }
}

fn map_rows_to_logs(
    rows: Vec<sqlx::postgres::PgRow>,
    limit: u32,
) -> (Vec<AuditLog>, Option<String>) {
    use sqlx::Row;
    let mut items = Vec::new();
    // detect if we have an extra row (rows.len() > limit) to set next cursor
    let mut next_cursor: Option<String> = None;
    let mut iter = rows.into_iter();
    for _ in 0..(limit as usize) {
        if let Some(row) = iter.next() {
            let id: i64 = row.try_get("id").unwrap_or_default();
            let user_id: Option<i64> = row.try_get::<Option<i64>, _>("user_id").ok().flatten();
            let user_id =
                user_id.and_then(|id| crate::domain::user::value_objects::UserId::new(id).ok());
            let action: String = row.try_get("action").unwrap_or_default();
            let resource_type: String = row.try_get("resource_type").unwrap_or_default();
            let resource_id: Option<i64> = row.try_get("resource_id").ok().flatten();
            let details: Option<serde_json::Value> = row.try_get("details").ok().flatten();
            let ip_address: Option<String> = row.try_get("ip_address").ok().flatten();
            let user_agent: Option<String> = row.try_get("user_agent").ok().flatten();
            let created_at: Option<chrono::DateTime<Utc>> =
                row.try_get("created_at").ok().flatten();

            items.push(AuditLog {
                id: Some(id),
                user_id,
                action,
                resource_type,
                resource_id,
                details,
                ip_address,
                user_agent,
                created_at,
            });
        }
    }

    // if remaining iterator has an element, that means there were more than limit rows
    if let Some(last_row) = iter.next() {
        // build a cursor from the extra row's created_at and id
        if let (Ok(created_at_opt), Ok(last_id)) = (
            last_row.try_get::<Option<chrono::DateTime<Utc>>, _>("created_at"),
            last_row.try_get::<i64, _>("id"),
        ) {
            if let Some(created_at) = created_at_opt {
                let cursor = AuditLogCursor::new(created_at, last_id);
                next_cursor = Some(cursor.encode());
            }
        }
    }

    (items, next_cursor)
}
