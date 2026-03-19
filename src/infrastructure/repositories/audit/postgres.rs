// src/infrastructure/repositories/audit/postgres.rs
use super::super::map_sqlx;
use crate::domain::audit::cursor::AuditLogCursor;
use crate::domain::audit::entity::{AuditLog, NewAuditLog};
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
#[must_use]
pub struct PostgresAuditLogRepository {
    pool: PgPool,
}

impl PostgresAuditLogRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::audit::repository::AuditLogRepository for PostgresAuditLogRepository {
    async fn insert(&self, log: NewAuditLog) -> DomainResult<()> {
        sqlx::query(
            r"
            INSERT INTO audit_logs (user_id, action, resource_type, resource_id, details, ip_address, user_agent)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ",
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
                .bind(i64::from(limit) + 1)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        // no cursor
        let rows = sqlx::query(QUERY_LIST_NO_CURSOR)
            .bind(i64::from(limit) + 1)
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
                .bind(i64::from(limit) + 1)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        let rows = sqlx::query(QUERY_FIND_BY_USER_NO_CURSOR)
            .bind(user_id)
            .bind(i64::from(limit) + 1)
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
                .bind(i64::from(limit) + 1)
                .fetch_all(&self.pool)
                .await
                .map_err(map_sqlx)?;
            return Ok(map_rows_to_logs(rows, limit));
        }

        let rows = sqlx::query(QUERY_FIND_BY_RESOURCE_NO_CURSOR)
            .bind(resource_type)
            .bind(resource_id)
            .bind(i64::from(limit) + 1)
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
    let mut items = rows
        .into_iter()
        .map(|row| {
            let id: i64 = row.try_get("id").expect("audit log id");
            let user_id: Option<i64> = row.try_get::<Option<i64>, _>("user_id").ok().flatten();
            let user_id =
                user_id.and_then(|id| crate::domain::user::value_objects::UserId::new(id).ok());
            let action: String = row.try_get("action").expect("audit log action");
            let resource_type: String = row
                .try_get("resource_type")
                .expect("audit log resource type");
            let resource_id: Option<i64> = row.try_get("resource_id").ok().flatten();
            let details: Option<serde_json::Value> = row.try_get("details").ok().flatten();
            let ip_address: Option<String> = row.try_get("ip_address").ok().flatten();
            let user_agent: Option<String> = row.try_get("user_agent").ok().flatten();
            let created_at: chrono::DateTime<Utc> =
                row.try_get("created_at").expect("audit log created_at");

            AuditLog {
                id,
                user_id,
                action,
                resource_type,
                resource_id,
                details,
                ip_address,
                user_agent,
                created_at,
            }
        })
        .collect::<Vec<_>>();

    let next_cursor = trim_to_page_and_build_cursor(&mut items, limit);

    (items, next_cursor)
}

fn trim_to_page_and_build_cursor(items: &mut Vec<AuditLog>, limit: u32) -> Option<String> {
    if items.len() <= limit as usize {
        return None;
    }

    items.truncate(limit as usize);
    items
        .last()
        .map(|last| AuditLogCursor::new(last.created_at, last.id).encode())
}

#[cfg(test)]
mod tests {
    use super::trim_to_page_and_build_cursor;
    use crate::domain::audit::cursor::AuditLogCursor;
    use crate::domain::audit::entity::AuditLog;
    use chrono::{Duration, Utc};

    fn audit_log(id: i64, created_at: chrono::DateTime<Utc>) -> AuditLog {
        AuditLog {
            id,
            user_id: None,
            action: "test".into(),
            resource_type: "article".into(),
            resource_id: Some(id),
            details: None,
            ip_address: None,
            user_agent: None,
            created_at,
        }
    }

    #[test]
    fn next_cursor_uses_last_item_in_current_page() {
        let now = Utc::now();
        let third = audit_log(3, now - Duration::minutes(3));
        let mut items = vec![
            audit_log(1, now - Duration::minutes(1)),
            audit_log(2, now - Duration::minutes(2)),
            third.clone(),
        ];

        let next_cursor = trim_to_page_and_build_cursor(&mut items, 2);

        assert_eq!(items.len(), 2);
        let cursor = AuditLogCursor::decode(&next_cursor.expect("next cursor")).unwrap();
        assert_eq!(cursor.id, 2);
        assert_eq!(cursor.created_at, items[1].created_at);
        assert_ne!(cursor.id, third.id);
    }
}
